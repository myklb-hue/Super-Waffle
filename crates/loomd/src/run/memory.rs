//! What a graph remembers (SPEC §6.5, §9.2).
//!
//! Two stores and a hub. Working memory is in process, fast and windowed: it
//! holds the last few minutes and it goes away with the run. Long-term memory
//! is SQLite on disk with a vector beside each episode, and it is the reason
//! the assistant knows your name tomorrow. The hub is what a model is actually
//! given — one handle, `remember()` and `forget()`, and a recall it never has
//! to ask for.
//!
//! Relevance is cosine similarity over embeddings the `Perception` provider
//! makes, which is the same arrangement as everywhere else in the engine: a
//! real implementation and a scripted one, so the ordering can be proven
//! without a model on the machine.
//!
//! One deliberate limit, written down rather than discovered: recall over the
//! long-term store reads every vector and scores it. That is linear, and at the
//! scale a person's assistant reaches — thousands of episodes, not millions —
//! it is a few milliseconds and no index to keep correct. An index is the
//! answer at a scale this program does not have yet, and a wrong index is worse
//! than a scan.

use rusqlite::Connection;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// One thing that happened, as it is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct Episode {
    pub id: String,
    pub text: String,
    /// What kind of thing it is: `episode`, `person`, `place`, `fact`.
    pub kind: String,
    /// Unix seconds.
    pub at: i64,
    /// How well it matched what was asked for, between 0 and 1. Recency alone
    /// scores 1: a store with no vectors still recalls, it just cannot rank.
    pub score: f64,
}

impl Episode {
    /// One line, the way it goes into a model's context.
    pub fn line(&self) -> String {
        format!("[{}] {}", self.kind, self.text)
    }
}

/// A place episodes go (SPEC §6.5).
pub trait Store: Send + Sync {
    /// Keep this, and answer with its id.
    fn remember(&self, text: &str, kind: &str, vector: Option<&[f32]>) -> Result<String, String>;

    /// The best `max` matches for a vector, or the most recent when there is
    /// none to match against.
    fn recall(
        &self,
        about: Option<&[f32]>,
        max: usize,
        cutoff: f64,
    ) -> Result<Vec<Episode>, String>;

    /// Everything whose text contains `what`, gone. Answers with how many.
    fn forget(&self, what: &str) -> Result<usize, String>;

    /// How many episodes are in it, for the panel.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// What the panel's row says: `in-process · 128 items · 5 min`.
    fn describe(&self) -> String;

    /// What has been learned since the last time this was asked, and never
    /// again after.
    ///
    /// Consolidation *copies* rather than moves. Moving would make working
    /// memory a queue with a hole at the end: a window that slides is supposed
    /// to lose things, and if losing them is also how they become permanent
    /// then the two behaviours fight, and which one wins depends on whether a
    /// run happened to be busy. Copying keeps them separate — working memory
    /// forgets on its own schedule, the long-term store keeps what mattered,
    /// and nothing depends on the order the two happen in.
    ///
    /// The default is nothing, which is right for a store that is itself a
    /// destination: long-term memory has nowhere further to consolidate to.
    fn pending(&self) -> Vec<Episode> {
        Vec::new()
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Between −1 and 1, and 0 when either side has nothing to say.
pub fn similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f64, 0.0f64, 0.0f64);
    for (x, y) in a.iter().zip(b) {
        dot += (*x as f64) * (*y as f64);
        na += (*x as f64).powi(2);
        nb += (*y as f64).powi(2);
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

// ------------------------------------------------------------ working memory

struct Held {
    episode: Episode,
    vector: Option<Vec<f32>>,
    /// Whether this has already been carried to the long-term store.
    consolidated: bool,
}

/// In process, fast, windowed (SPEC §6.5).
///
/// It belongs to the run: a live graph carries one across every event, and
/// stopping the graph is what empties it. Nothing here touches disk, which is
/// the point — this is the store that is allowed to forget.
pub struct Working {
    items: Mutex<VecDeque<Held>>,
    /// How many it holds before the oldest fall out.
    limit: usize,
    /// How long an episode stays, or none for as long as the run.
    window: Option<Duration>,
    next: Mutex<u64>,
}

impl Working {
    pub fn new(limit: usize, window: Option<Duration>) -> Self {
        Self {
            items: Mutex::new(VecDeque::new()),
            limit: limit.max(1),
            window,
            next: Mutex::new(0),
        }
    }

    /// Whatever has aged out of the window, dropped.
    fn expire(&self, items: &mut VecDeque<Held>) {
        let Some(window) = self.window else { return };
        let cutoff = now() - window.as_secs() as i64;
        while items.front().is_some_and(|h| h.episode.at < cutoff) {
            items.pop_front();
        }
    }
}

impl Store for Working {
    fn remember(&self, text: &str, kind: &str, vector: Option<&[f32]>) -> Result<String, String> {
        let mut items = self.items.lock().unwrap();
        self.expire(&mut items);
        // The window is the point of this store: past the limit, the oldest
        // goes. It has been consolidated by now if it was ever going to be.
        while items.len() >= self.limit {
            items.pop_front();
        }
        let mut next = self.next.lock().unwrap();
        *next += 1;
        let id = format!("w{next}");
        items.push_back(Held {
            consolidated: false,
            episode: Episode {
                id: id.clone(),
                text: text.to_owned(),
                kind: kind.to_owned(),
                at: now(),
                score: 1.0,
            },
            vector: vector.map(<[f32]>::to_vec),
        });
        Ok(id)
    }

    fn recall(
        &self,
        about: Option<&[f32]>,
        max: usize,
        cutoff: f64,
    ) -> Result<Vec<Episode>, String> {
        let mut items = self.items.lock().unwrap();
        self.expire(&mut items);
        let mut found: Vec<Episode> = match about {
            // Nothing to match against: the most recent, newest first. A
            // working memory asked with no question is a short-term log.
            None => items
                .iter()
                .rev()
                .map(|h| h.episode.clone())
                .take(max)
                .collect(),
            Some(vector) => {
                let mut scored: Vec<Episode> = items
                    .iter()
                    .map(|h| {
                        let mut e = h.episode.clone();
                        e.score = h
                            .vector
                            .as_deref()
                            .map(|v| similarity(v, vector))
                            .unwrap_or(0.0);
                        e
                    })
                    .filter(|e| e.score >= cutoff)
                    .collect();
                scored.sort_by(|a, b| b.score.total_cmp(&a.score));
                scored.truncate(max);
                scored
            }
        };
        found.truncate(max);
        Ok(found)
    }

    fn forget(&self, what: &str) -> Result<usize, String> {
        let mut items = self.items.lock().unwrap();
        let before = items.len();
        items.retain(|h| !h.episode.text.contains(what) && h.episode.id != what);
        Ok(before - items.len())
    }

    fn len(&self) -> usize {
        let mut items = self.items.lock().unwrap();
        self.expire(&mut items);
        items.len()
    }

    fn describe(&self) -> String {
        match self.window {
            Some(w) => format!(
                "in-process · {} of {} items · {}",
                self.len(),
                self.limit,
                human_window(w)
            ),
            None => format!("in-process · {} of {} items", self.len(), self.limit),
        }
    }

    fn pending(&self) -> Vec<Episode> {
        let mut items = self.items.lock().unwrap();
        self.expire(&mut items);
        let mut out = Vec::new();
        for held in items.iter_mut() {
            if !held.consolidated {
                held.consolidated = true;
                out.push(held.episode.clone());
            }
        }
        out
    }
}

fn human_window(w: Duration) -> String {
    let s = w.as_secs();
    if s > 0 && s.is_multiple_of(3600) {
        format!("{} h", s / 3600)
    } else if s > 0 && s.is_multiple_of(60) {
        format!("{} min", s / 60)
    } else {
        format!("{s} s")
    }
}

/// `5m`, `30s`, `2h`, or a bare number of minutes.
pub fn parse_window(text: &str) -> Option<Duration> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let (digits, unit) = text.split_at(text.len() - 1);
    let (value, seconds): (f64, f64) = match unit {
        "s" => (digits.parse().ok()?, 1.0),
        "m" => (digits.parse().ok()?, 60.0),
        "h" => (digits.parse().ok()?, 3600.0),
        "d" => (digits.parse().ok()?, 86400.0),
        // A bare number is minutes: every window in the specification is
        // written in minutes, and `5` meaning five seconds would surprise.
        _ => (text.parse().ok()?, 60.0),
    };
    (value > 0.0).then(|| Duration::from_secs_f64(value * seconds))
}

// ---------------------------------------------------------- long-term memory

/// SQLite and vectors: people, places, episodes (SPEC §6.5).
pub struct LongTerm {
    db: Mutex<Connection>,
    where_it_is: String,
}

impl LongTerm {
    /// Open or create the database.
    ///
    /// The schema is created every time rather than migrated, because there is
    /// one version of it. When there is a second, this is where the migration
    /// goes, and `IF NOT EXISTS` will not be enough.
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not make {}: {e}", parent.display()))?;
        }
        let db = Connection::open(path).map_err(|e| format!("could not open the memory: {e}"))?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS episodes (
                 id     TEXT PRIMARY KEY,
                 text   TEXT NOT NULL,
                 kind   TEXT NOT NULL,
                 at     INTEGER NOT NULL,
                 vector BLOB
             );
             CREATE INDEX IF NOT EXISTS episodes_at ON episodes (at);",
        )
        .map_err(|e| format!("could not prepare the memory: {e}"))?;
        Ok(Self {
            db: Mutex::new(db),
            where_it_is: path.display().to_string(),
        })
    }

    /// A database that lives and dies with the process, for tests.
    pub fn in_memory() -> Result<Self, String> {
        let db =
            Connection::open_in_memory().map_err(|e| format!("could not open the memory: {e}"))?;
        db.execute_batch(
            "CREATE TABLE episodes (
                 id TEXT PRIMARY KEY, text TEXT NOT NULL, kind TEXT NOT NULL,
                 at INTEGER NOT NULL, vector BLOB);",
        )
        .map_err(|e| format!("could not prepare the memory: {e}"))?;
        Ok(Self {
            db: Mutex::new(db),
            where_it_is: ":memory:".into(),
        })
    }
}

/// A vector as bytes, little-endian, four bytes to a number.
///
/// A BLOB rather than JSON: it is half the size and it parses by pointer
/// arithmetic rather than by parser, which for the one column that is read on
/// every single recall is worth the four lines it costs.
fn pack(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn unpack(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

impl Store for LongTerm {
    fn remember(&self, text: &str, kind: &str, vector: Option<&[f32]>) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        // Content-addressed rather than counted: remembering the same thing
        // twice is one memory, not two, and a graph that runs every minute
        // would otherwise fill the database with the same sentence.
        let id = format!("e{:016x}", fingerprint(text, kind));
        db.execute(
            "INSERT INTO episodes (id, text, kind, at, vector) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET at = excluded.at",
            rusqlite::params![id, text, kind, now(), vector.map(pack)],
        )
        .map_err(|e| format!("could not remember that: {e}"))?;
        Ok(id)
    }

    fn recall(
        &self,
        about: Option<&[f32]>,
        max: usize,
        cutoff: f64,
    ) -> Result<Vec<Episode>, String> {
        let db = self.db.lock().unwrap();
        let mut query = db
            .prepare("SELECT id, text, kind, at, vector FROM episodes ORDER BY at DESC")
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map([], |row| {
                Ok((
                    Episode {
                        id: row.get(0)?,
                        text: row.get(1)?,
                        kind: row.get(2)?,
                        at: row.get(3)?,
                        score: 1.0,
                    },
                    row.get::<_, Option<Vec<u8>>>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut found = Vec::new();
        for row in rows {
            let (mut episode, vector) = row.map_err(|e| e.to_string())?;
            match about {
                None => found.push(episode),
                Some(wanted) => {
                    episode.score = vector
                        .as_deref()
                        .map(|v| similarity(&unpack(v), wanted))
                        .unwrap_or(0.0);
                    if episode.score >= cutoff {
                        found.push(episode);
                    }
                }
            }
        }
        if about.is_some() {
            found.sort_by(|a, b| b.score.total_cmp(&a.score));
        }
        found.truncate(max);
        Ok(found)
    }

    fn forget(&self, what: &str) -> Result<usize, String> {
        let db = self.db.lock().unwrap();
        let gone = db
            .execute(
                "DELETE FROM episodes WHERE id = ?1 OR text LIKE '%' || ?1 || '%'",
                rusqlite::params![what],
            )
            .map_err(|e| format!("could not forget that: {e}"))?;
        Ok(gone)
    }

    fn len(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.query_row("SELECT COUNT(*) FROM episodes", [], |r| r.get::<_, i64>(0))
            .map(|n| n as usize)
            .unwrap_or(0)
    }

    fn describe(&self) -> String {
        let where_it_is = self
            .where_it_is
            .rsplit_once('/')
            .map(|(_, f)| f.to_owned())
            .unwrap_or_else(|| self.where_it_is.clone());
        format!("sqlite + vectors · {} episodes · {where_it_is}", self.len())
    }
}

/// A stable id for a piece of text, so the same memory is the same row.
///
/// FNV-1a: a few lines, no dependency, and the only thing asked of it is that
/// two different sentences almost never collide. It is not a security hash and
/// nothing here pretends otherwise.
fn fingerprint(text: &str, kind: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in kind.as_bytes().iter().chain(b"\0").chain(text.as_bytes()) {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

// ------------------------------------------------------------------- vault

/// The stores a run has open.
///
/// One per run rather than one per event: working memory is windowed, and a
/// window means nothing if the store is rebuilt for every event the graph
/// handles. The long-term store would survive either way — it is a file — but
/// opening SQLite five times a second to write one row would be its own kind of
/// silly.
pub struct Vault {
    open: Mutex<std::collections::HashMap<String, Arc<dyn Store>>>,
    /// The workspace, which is what a relative database path is relative to.
    root: std::path::PathBuf,
}

impl Vault {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            open: Mutex::new(std::collections::HashMap::new()),
            root: root.into(),
        }
    }

    /// The store this block is, opening it the first time it is asked for.
    pub fn store(&self, block: &graph_format::Block) -> Result<Arc<dyn Store>, String> {
        if let Some(already) = self.open.lock().unwrap().get(&block.id) {
            return Ok(Arc::clone(already));
        }
        let made: Arc<dyn Store> = match block.kind.as_str() {
            "working-memory" => Arc::new(Working::new(
                super::blocks::number(block, "items").unwrap_or(128.0) as usize,
                super::blocks::setting(block, "window").and_then(parse_window),
            )),
            "long-term-memory" | "episode-log" => {
                let named = super::blocks::setting(block, "path")
                    .filter(|p| !p.trim().is_empty())
                    .unwrap_or("memory.db");
                let path = std::path::Path::new(named);
                let path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    self.root.join(path)
                };
                Arc::new(LongTerm::open(&path)?)
            }
            other => return Err(format!("`{other}` is not a memory store")),
        };
        self.open
            .lock()
            .unwrap()
            .insert(block.id.clone(), Arc::clone(&made));
        Ok(made)
    }
}

// --------------------------------------------------------------------- hub

/// How recall orders what it found (SPEC §9.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Order {
    /// Store order, newest first inside each: working memory before long-term,
    /// which is the specification's default and the one a person expects.
    RecentFirst,
    /// Best match first, wherever it came from.
    Relevance,
    /// Store order kept, ranked by relevance inside each.
    Mixed,
}

impl Order {
    pub fn read(name: &str) -> Self {
        match name {
            "relevance" => Order::Relevance,
            "mixed" => Order::Mixed,
            _ => Order::RecentFirst,
        }
    }
}

/// One slot per store, one handle out (SPEC §9.2).
///
/// The hub is not transparent the way a Toolbox is. A model holding a Toolbox
/// calls `terminal.run` and never names the Toolbox; a model holding a hub
/// calls `remember()`, and which store it lands in is the hub's business. That
/// is the whole difference between bundling tools and bundling memory, and it
/// is why the hub stays in the model's tool list where a Toolbox disappears
/// from it.
pub struct Hub {
    /// The stores, in slot order: the first is where new memories land, the
    /// last is where consolidation carries them.
    pub stores: Vec<(String, Arc<dyn Store>)>,
    pub order: Order,
    pub max: usize,
    pub cutoff: f64,
}

impl Hub {
    /// What to put in front of the model, before it has asked for anything.
    ///
    /// Recall is not a tool (§9.2 gives the model `remember()` and `forget()`,
    /// and nothing else): a model that has to ask to remember will forget to,
    /// and the one thing memory must not depend on is the model's diligence.
    pub fn recall(&self, about: Option<&[f32]>) -> Vec<Episode> {
        let mut found: Vec<Episode> = Vec::new();
        for (_, store) in &self.stores {
            let from_here = match self.order {
                // Recency, so the vector is not what decides — but the cutoff
                // still is, because an irrelevant memory is noise wherever it
                // sits in the order.
                Order::RecentFirst => store.recall(None, self.max, 0.0),
                _ => store.recall(about, self.max, self.cutoff),
            };
            match from_here {
                Ok(mut some) => found.append(&mut some),
                // A store that cannot be read is a broken store, not a broken
                // graph: the others still answer.
                Err(_) => continue,
            }
        }
        if self.order == Order::Relevance {
            found.sort_by(|a, b| b.score.total_cmp(&a.score));
        }
        found.truncate(self.max);
        found
    }

    /// Into the first store: new memories are working memories, and
    /// consolidation is what makes them permanent.
    pub fn remember(
        &self,
        text: &str,
        kind: &str,
        vector: Option<&[f32]>,
    ) -> Result<String, String> {
        let Some((_, first)) = self.stores.first() else {
            return Err("this memory hub has no stores wired into it".into());
        };
        first.remember(text, kind, vector)
    }

    /// Out of all of them: forgetting something from one store and not another
    /// is not forgetting it.
    pub fn forget(&self, what: &str) -> Result<usize, String> {
        let mut gone = 0;
        let mut trouble = None;
        for (_, store) in &self.stores {
            match store.forget(what) {
                Ok(n) => gone += n,
                Err(e) => trouble = Some(e),
            }
        }
        match trouble {
            Some(e) if gone == 0 => Err(e),
            _ => Ok(gone),
        }
    }

    /// Carry what the earlier stores have learned into the last one,
    /// summarising on the way (SPEC §9.2).
    ///
    /// `summarise` is passed in rather than done here because summarising is a
    /// model's job and this file knows nothing about models. Handing back the
    /// text unchanged is a valid summariser and is what happens when the
    /// setting is off.
    pub fn consolidate(&self, summarise: &dyn Fn(&Episode) -> String) -> Vec<Episode> {
        if self.stores.len() < 2 {
            return Vec::new();
        }
        let (_, last) = self.stores.last().expect("checked above");
        let mut moved = Vec::new();
        for (_, store) in &self.stores[..self.stores.len() - 1] {
            for episode in store.pending() {
                let text = summarise(&episode);
                if last.remember(&text, &episode.kind, None).is_ok() {
                    moved.push(Episode { text, ..episode });
                }
            }
        }
        moved
    }

    /// What the panel's Stores section lists.
    pub fn rows(&self) -> Vec<(String, String)> {
        self.stores
            .iter()
            .map(|(id, store)| (id.clone(), store.describe()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector(seed: f32) -> Vec<f32> {
        (0..8).map(|i| ((seed + i as f32) * 0.125).sin()).collect()
    }

    #[test]
    fn a_thing_remembered_is_a_thing_recalled() {
        let store = Working::new(8, None);
        store
            .remember("Mykl's partner is Sam", "person", None)
            .unwrap();
        let found = store.recall(None, 4, 0.0).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "Mykl's partner is Sam");
    }

    /// The whole reason for the vectors: what comes back is what the question
    /// was about, not what happened to be last.
    #[test]
    fn recall_is_ordered_by_what_it_is_about() {
        let store = Working::new(8, None);
        store
            .remember("about a door", "episode", Some(&vector(1.0)))
            .unwrap();
        store
            .remember("about a cat", "episode", Some(&vector(9.0)))
            .unwrap();
        let found = store.recall(Some(&vector(1.0)), 2, -1.0).unwrap();
        assert_eq!(found[0].text, "about a door");
        assert!(found[0].score > found[1].score);
    }

    #[test]
    fn a_cutoff_leaves_out_what_is_not_relevant() {
        let store = Working::new(8, None);
        store
            .remember("about a door", "episode", Some(&vector(1.0)))
            .unwrap();
        store
            .remember("about a cat", "episode", Some(&vector(9.0)))
            .unwrap();
        let strict = store.recall(Some(&vector(1.0)), 8, 0.999).unwrap();
        assert_eq!(strict.len(), 1, "only the one it asked about survives");
    }

    /// Working memory is the store that is allowed to forget: past its limit,
    /// the oldest goes.
    #[test]
    fn working_memory_keeps_only_its_window() {
        let store = Working::new(2, None);
        for text in ["one", "two", "three"] {
            store.remember(text, "episode", None).unwrap();
        }
        assert_eq!(store.len(), 2);
        let kept: Vec<String> = store
            .recall(None, 8, 0.0)
            .unwrap()
            .into_iter()
            .map(|e| e.text)
            .collect();
        assert_eq!(kept, ["three", "two"], "the oldest should have gone");
    }

    /// Consolidation copies, and copies each thing once. A graph that
    /// consolidates every ten minutes must not write the same episode six
    /// times an hour.
    #[test]
    fn what_is_pending_is_what_has_not_been_carried_yet() {
        let store = Working::new(8, None);
        store.remember("one", "episode", None).unwrap();
        assert_eq!(store.pending().len(), 1);
        assert_eq!(store.pending().len(), 0, "it was carried already");
        store.remember("two", "episode", None).unwrap();
        assert_eq!(store.pending().len(), 1);
        assert_eq!(store.len(), 2, "consolidating does not empty it");
    }

    #[test]
    fn a_window_drops_what_is_older_than_it() {
        let store = Working::new(8, Some(Duration::from_secs(60)));
        store.remember("recent", "episode", None).unwrap();
        // Reach in and age it: waiting a minute is not a test.
        store.items.lock().unwrap()[0].episode.at = now() - 3600;
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn long_term_memory_survives_being_closed_and_opened() {
        let dir = std::env::temp_dir().join(format!("cyberloom-mem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("memory.db");
        {
            let store = LongTerm::open(&path).unwrap();
            store
                .remember("my name is Mykl", "person", Some(&vector(2.0)))
                .unwrap();
        }
        let reopened = LongTerm::open(&path).unwrap();
        let found = reopened.recall(None, 4, 0.0).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "my name is Mykl");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A graph that runs every minute must not fill the database with the same
    /// sentence sixty times an hour.
    #[test]
    fn remembering_the_same_thing_twice_is_one_memory() {
        let store = LongTerm::in_memory().unwrap();
        store
            .remember("the door was closed", "episode", None)
            .unwrap();
        store
            .remember("the door was closed", "episode", None)
            .unwrap();
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn a_vector_survives_the_round_trip_through_sqlite() {
        let store = LongTerm::in_memory().unwrap();
        store
            .remember("about a door", "episode", Some(&vector(1.0)))
            .unwrap();
        store
            .remember("about a cat", "episode", Some(&vector(9.0)))
            .unwrap();
        let found = store.recall(Some(&vector(1.0)), 1, -1.0).unwrap();
        assert_eq!(found[0].text, "about a door");
        assert!(found[0].score > 0.99, "score was {}", found[0].score);
    }

    /// §12.3: delete a person and every sighting goes with them.
    #[test]
    fn forgetting_a_person_takes_their_sightings_too() {
        let store = LongTerm::in_memory().unwrap();
        store
            .remember("Sam is Mykl's partner", "person", None)
            .unwrap();
        store
            .remember("saw Sam at the door", "episode", None)
            .unwrap();
        store
            .remember("saw a cat at the door", "episode", None)
            .unwrap();
        assert_eq!(store.forget("Sam").unwrap(), 2);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn windows_are_read_the_way_they_are_written() {
        assert_eq!(parse_window("5m"), Some(Duration::from_secs(300)));
        assert_eq!(parse_window("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_window("2h"), Some(Duration::from_secs(7200)));
        assert_eq!(parse_window("5"), Some(Duration::from_secs(300)));
        assert_eq!(parse_window(""), None);
        assert_eq!(parse_window("soon"), None);
    }

    #[test]
    fn similarity_is_one_for_the_same_vector_and_zero_for_nothing() {
        assert!((similarity(&vector(1.0), &vector(1.0)) - 1.0).abs() < 1e-9);
        assert_eq!(similarity(&[], &[]), 0.0);
        assert_eq!(similarity(&[1.0, 0.0], &[0.0, 1.0, 2.0]), 0.0);
    }
}
