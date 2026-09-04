//! A graph that never finishes (SPEC §8.1, §8.2).
//!
//! Once mode runs a plan top to bottom and stops. Live mode arms the sources
//! and then runs, per event, only the part of the graph downstream of whatever
//! fired. Two sources on one canvas are two programs sharing a page, and the
//! plan is what keeps them apart.
//!
//! # One event at a time
//!
//! Events are handled in sequence rather than concurrently, and the overlap
//! policy (§8.3) is what happens to the ones that arrive meanwhile. That is a
//! decision, not a limitation: a graph is a program with shared state — a
//! Memory hub, a Terminal's working directory, a model's context — and running
//! two events through it at once would make every one of those a race the user
//! never wrote. Parallelism belongs inside a Loop frame, where §8.3 puts it
//! and where the items are independent by construction.

use super::event::{Level, RunEvent, RunOutcome};
use super::plan::{Plan, plan};
use super::runner::{Decision, Runner, Warning};
use super::source::{Armed, Fired, arm};
use super::value::Value;
use graph_format::{Endpoint, Graph, OverlapPolicy, RunMode};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// How long the loop waits before looking at the stop flag again.
const IDLE: Duration = Duration::from_millis(120);

/// What a live run did while it was up.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tally {
    pub events: u32,
    /// Events the overlap policy threw away, which is a number the graph panel
    /// shows rather than something to hide.
    pub dropped: u32,
    pub errors: u32,
}

pub struct Live<'a> {
    pub graph: &'a Graph,
    pub root: &'a Path,
    pub provider: &'a dyn super::model::ModelProvider,
    pub run: String,
    pub cancel: Arc<AtomicBool>,
    /// Set while the graph is paused: events keep queueing, nothing runs
    /// (SPEC §8.1).
    pub paused: Arc<AtomicBool>,
    pub scratch: Arc<super::sense::Scratch>,
    pub eye: Arc<dyn super::perceive::Perception>,
    pub vault: Arc<super::memory::Vault>,
}

impl Live<'_> {
    /// Arm the sources and run until stopped.
    pub fn execute(
        &self,
        emit: &mut dyn FnMut(RunEvent),
        ask: &mut dyn FnMut(&Warning) -> Decision,
    ) -> Tally {
        let started = Instant::now();
        let plan = plan(self.graph);
        let (tx, rx) = channel::<Fired>();

        emit(RunEvent::Started {
            run: self.run.clone(),
            graph: self.graph.id.clone(),
            order: plan.order.clone(),
        });
        for problem in &plan.problems {
            self.say(emit, None, Level::Warn, problem.clone());
        }

        let armed = self.arm_sources(&plan, &tx, emit);
        if armed.is_empty() {
            // A live graph with nothing armed would sit forever doing nothing,
            // which looks exactly like a graph that is working.
            self.say(
                emit,
                None,
                Level::Error,
                "nothing is armed: a live graph needs a source".into(),
            );
            emit(RunEvent::Finished {
                run: self.run.clone(),
                outcome: RunOutcome::Failed,
                ms: started.elapsed().as_millis() as u32,
                results: Vec::new(),
            });
            return Tally::default();
        }

        let mut tally = Tally::default();
        let mut queue: VecDeque<Fired> = VecDeque::new();
        // What blocks are holding between events (SPEC §8.4). Carried across
        // events when the graph says to, and dropped between them when not.
        let mut carried: HashMap<Endpoint, Value> = HashMap::new();

        // "Consolidation: every n minutes" (SPEC §9.2). Working memory is
        // windowed, so a graph that never consolidates loses what it learned as
        // the window slides — the timer is what makes a live assistant's memory
        // last longer than its attention span.
        let mut consolidators = self.consolidators(&plan);

        while !self.cancel.load(Ordering::Relaxed) {
            self.absorb(&rx, &mut queue, &mut tally, emit);
            self.consolidate_due(&plan, &mut consolidators, emit, ask);

            if self.paused.load(Ordering::Relaxed) {
                std::thread::sleep(IDLE);
                continue;
            }
            let Some(event) = queue.pop_front() else {
                std::thread::sleep(IDLE);
                continue;
            };

            tally.events += 1;
            let errors = self.fire(&plan, &event, &mut carried, emit, ask);
            tally.errors += errors;
        }

        for source in armed {
            source.disarm();
        }
        emit(RunEvent::Finished {
            run: self.run.clone(),
            outcome: RunOutcome::Stopped,
            ms: started.elapsed().as_millis() as u32,
            results: Vec::new(),
        });
        tally
    }

    /// The memory hubs with an interval set, and when each is next due.
    fn consolidators(&self, plan: &Plan) -> Vec<(String, Duration, Instant)> {
        let _ = plan;
        self.graph
            .blocks
            .iter()
            .filter(|b| b.kind == "memory-hub")
            .filter_map(|b| {
                let every = super::blocks::setting(b, "consolidateEvery")?;
                let period = super::memory::parse_window(every)?;
                Some((b.id.clone(), period, Instant::now() + period))
            })
            .collect()
    }

    /// Carry what is due, if anything is.
    ///
    /// It runs on the loop thread, between events rather than during one: a
    /// consolidation that summarises asks the model, and a model call in the
    /// middle of an event would put the graph's own work behind it.
    fn consolidate_due(
        &self,
        plan: &Plan,
        due: &mut [(String, Duration, Instant)],
        emit: &mut dyn FnMut(RunEvent),
        ask: &mut dyn FnMut(&Warning) -> Decision,
    ) {
        let now = Instant::now();
        for (id, period, next) in due.iter_mut() {
            if now < *next {
                continue;
            }
            *next = now + *period;
            let runner = Runner {
                graph: self.graph,
                root: self.root,
                provider: self.provider,
                run: self.run.clone(),
                cancel: Arc::clone(&self.cancel),
                scratch: Arc::clone(&self.scratch),
                eye: Arc::clone(&self.eye),
                vault: Arc::clone(&self.vault),
            };
            runner.consolidate_hub(id, plan, emit, ask);
        }
    }

    /// Arm every source the mode calls for.
    ///
    /// Schedule mode arms only the Schedule blocks; between ticks the graph
    /// sleeps (SPEC §8.1). A source that will not arm is reported and the rest
    /// still run, because a webhook whose port is taken should not take the
    /// folder watcher down with it.
    fn arm_sources(
        &self,
        plan: &Plan,
        tx: &Sender<Fired>,
        emit: &mut dyn FnMut(RunEvent),
    ) -> Vec<Armed> {
        let mut armed = Vec::new();
        for id in plan.sources() {
            let Some(block) = self.graph.blocks.iter().find(|b| &b.id == id) else {
                continue;
            };
            if block.disabled {
                continue;
            }
            if self.graph.run_mode == RunMode::Schedule && block.kind != "schedule" {
                continue;
            }
            match arm(block, self.root, tx.clone()) {
                Ok(None) => {}
                Ok(Some(source)) => {
                    self.say(
                        emit,
                        Some(id),
                        Level::Info,
                        format!("armed · {}", source.state),
                    );
                    emit(RunEvent::BlockState {
                        run: self.run.clone(),
                        block: id.clone(),
                        state: super::event::BlockState::Ready,
                    });
                    emit(RunEvent::SourceArmed {
                        run: self.run.clone(),
                        block: id.clone(),
                        state: source.state.clone(),
                    });
                    armed.push(source);
                }
                Err(why) => {
                    self.say(emit, Some(id), Level::Error, why.clone());
                    emit(RunEvent::BlockError {
                        run: self.run.clone(),
                        block: id.clone(),
                        message: why,
                        detail: None,
                    });
                }
            }
        }
        armed
    }

    /// Take everything waiting on the channel, applying the overlap policy
    /// (SPEC §8.3).
    fn absorb(
        &self,
        rx: &Receiver<Fired>,
        queue: &mut VecDeque<Fired>,
        tally: &mut Tally,
        emit: &mut dyn FnMut(RunEvent),
    ) {
        let overlap = &self.graph.overlap;
        let max = overlap.max_queue.max(1) as usize;

        while let Ok(event) = rx.try_recv() {
            let busy = !queue.is_empty();
            match overlap.policy {
                OverlapPolicy::Queue => {
                    if queue.len() >= max {
                        tally.dropped += 1;
                        self.say(
                            emit,
                            Some(&event.node),
                            Level::Warn,
                            format!("queue is full at {max}; dropped an event"),
                        );
                        continue;
                    }
                    queue.push_back(event);
                }
                OverlapPolicy::DropNewest => {
                    if busy {
                        tally.dropped += 1;
                        continue;
                    }
                    queue.push_back(event);
                }
                OverlapPolicy::DropOldest => {
                    queue.push_back(event);
                    while queue.len() > max {
                        queue.pop_front();
                        tally.dropped += 1;
                    }
                }
                OverlapPolicy::Coalesce => {
                    // A burst becomes its last event: a file saved four times
                    // in a second is one thing that happened, not four.
                    let window = Duration::from_millis(u64::from(overlap.coalesce_ms));
                    if let Some(last) = queue.back()
                        && last.node == event.node
                        && event.at.duration_since(last.at) <= window
                    {
                        queue.pop_back();
                        tally.dropped += 1;
                    }
                    queue.push_back(event);
                }
            }
        }
    }

    /// Run the part of the graph below one event.
    fn fire(
        &self,
        plan: &Plan,
        event: &Fired,
        carried: &mut HashMap<Endpoint, Value>,
        emit: &mut dyn FnMut(RunEvent),
        ask: &mut dyn FnMut(&Warning) -> Decision,
    ) -> u32 {
        let mut steps = plan.downstream_of(&event.node, &event.port);
        if steps.is_empty() {
            self.say(
                emit,
                Some(&event.node),
                Level::Warn,
                format!("{}.{} is wired to nothing", event.node, event.port),
            );
            return 0;
        }

        self.say(
            emit,
            Some(&event.node),
            Level::Info,
            format!("{} → {}", event.port, steps.join(" → ")),
        );

        let mut seeded = if self.graph.between.keep_state {
            carried.clone()
        } else {
            HashMap::new()
        };
        if event.reads {
            // A device hands nothing over: it goes at the head of its own run
            // and is read there. Seeding its port instead would put a `null` on
            // the wire and call it a frame.
            steps.insert(0, event.node.clone());
        } else {
            seeded.insert(Endpoint::new(&event.node, &event.port), event.value.clone());
        }

        let runner = Runner {
            graph: self.graph,
            root: self.root,
            provider: self.provider,
            run: self.run.clone(),
            cancel: Arc::clone(&self.cancel),
            scratch: Arc::clone(&self.scratch),
            eye: Arc::clone(&self.eye),
            vault: Arc::clone(&self.vault),
        };
        let summary = runner.execute_steps(&steps, seeded, emit, ask);

        // "A hardware fault *pauses*; one click resumes" (SPEC §12.1). A camera
        // that is not there fails on every tick, and at a camera's tick rate
        // that is not an error report, it is a flood — the same sentence a few
        // times a second until someone notices. Pausing says it once and stops,
        // and Resume is the click: the source stays armed, so a camera plugged
        // back in works from the next tick.
        //
        // The fault is the *device* failing to be read, not any error in the
        // pass it started: a block downstream that throws is a program with a
        // bug in it, and holding the graph over one would be holding it over
        // something no amount of resuming fixes. So the test is whether the
        // port the tick asked for came back with anything.
        let could_not_read = event.reads
            && !summary
                .values
                .contains_key(&Endpoint::new(&event.node, &event.port));
        if could_not_read && !self.paused.load(Ordering::Relaxed) {
            self.paused.store(true, Ordering::Relaxed);
            self.say(
                emit,
                Some(&event.node),
                Level::Warn,
                format!(
                    "{} could not be read, so the graph is held. Resume to try again.",
                    event.node
                ),
            );
            emit(RunEvent::Held {
                run: self.run.clone(),
                held: true,
            });
        }

        if self.graph.between.keep_state {
            // What the blocks produced is what they hold until the next event
            // (SPEC §8.4).
            *carried = summary.values;
        }
        summary.errors.len() as u32
    }

    fn say(
        &self,
        emit: &mut dyn FnMut(RunEvent),
        source: Option<&str>,
        level: Level,
        message: String,
    ) {
        emit(RunEvent::Console {
            run: self.run.clone(),
            source: source.map(str::to_owned),
            level,
            message,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graph_format::{Between, Overlap};
    use std::time::Instant;

    fn fired(node: &str, at: Instant) -> Fired {
        Fired {
            node: node.into(),
            port: "file".into(),
            value: Value::Null,
            reads: false,
            at,
        }
    }

    fn graph_with(overlap: Overlap) -> Graph {
        let mut graph = graph_format::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/graphs/inbox-triage.loom"
        ))
        .unwrap();
        graph.overlap = overlap;
        graph.between = Between {
            keep_state: true,
            restart_on_crash: false,
        };
        graph
    }

    /// Drive `absorb` directly: the policies are arithmetic on a queue, and
    /// pinning them down needs no threads.
    fn absorb_all(overlap: Overlap, events: Vec<Fired>) -> (Vec<String>, Tally) {
        let graph = graph_with(overlap);
        let provider = super::super::model::Scripted::new([]);
        let live = Live {
            graph: &graph,
            root: Path::new("/tmp"),
            provider: &provider,
            run: "r".into(),
            cancel: Default::default(),
            paused: Default::default(),
            scratch: Arc::new(crate::run::sense::Scratch::open("absorb-test").unwrap()),
            eye: Arc::new(crate::run::perceive::Scripted::default()),
            vault: Arc::new(crate::run::memory::Vault::new("/tmp")),
        };
        let (tx, rx) = channel();
        for event in events {
            tx.send(event).unwrap();
        }
        drop(tx);
        let mut queue = VecDeque::new();
        let mut tally = Tally::default();
        live.absorb(&rx, &mut queue, &mut tally, &mut |_| {});
        (queue.iter().map(|f| f.node.clone()).collect(), tally)
    }

    fn burst(names: &[&str], apart: Duration) -> Vec<Fired> {
        let start = Instant::now();
        names
            .iter()
            .enumerate()
            .map(|(i, n)| fired(n, start + apart * i as u32))
            .collect()
    }

    /// Queue keeps them in order and stops at its maximum (SPEC §8.3).
    #[test]
    fn queue_holds_up_to_its_maximum_and_says_when_it_drops() {
        let (kept, tally) = absorb_all(
            Overlap {
                policy: OverlapPolicy::Queue,
                max_queue: 3,
                coalesce_ms: 0,
                loop_parallel: 2,
            },
            burst(&["a", "b", "c", "d", "e"], Duration::from_millis(10)),
        );
        assert_eq!(kept, ["a", "b", "c"]);
        assert_eq!(tally.dropped, 2);
    }

    /// Drop newest keeps what is already waiting and refuses the rest.
    #[test]
    fn drop_newest_keeps_the_first() {
        let (kept, tally) = absorb_all(
            Overlap {
                policy: OverlapPolicy::DropNewest,
                max_queue: 10,
                coalesce_ms: 0,
                loop_parallel: 2,
            },
            burst(&["a", "b", "c"], Duration::from_millis(10)),
        );
        assert_eq!(kept, ["a"]);
        assert_eq!(tally.dropped, 2);
    }

    /// Drop oldest keeps the freshest, which is what a graph watching a
    /// changing world usually wants.
    #[test]
    fn drop_oldest_keeps_the_last() {
        let (kept, tally) = absorb_all(
            Overlap {
                policy: OverlapPolicy::DropOldest,
                max_queue: 2,
                coalesce_ms: 0,
                loop_parallel: 2,
            },
            burst(&["a", "b", "c", "d"], Duration::from_millis(10)),
        );
        assert_eq!(kept, ["c", "d"]);
        assert_eq!(tally.dropped, 2);
    }

    /// Coalesce turns a burst from one source into the last of it: a file
    /// saved four times in a second is one thing that happened.
    #[test]
    fn coalesce_merges_a_burst_from_one_source() {
        let (kept, tally) = absorb_all(
            Overlap {
                policy: OverlapPolicy::Coalesce,
                max_queue: 10,
                coalesce_ms: 500,
                loop_parallel: 2,
            },
            burst(&["watch", "watch", "watch"], Duration::from_millis(50)),
        );
        assert_eq!(kept, ["watch"]);
        assert_eq!(tally.dropped, 2);
    }

    /// Two sources are two things happening, however close together.
    #[test]
    fn coalesce_does_not_merge_across_sources() {
        let (kept, _) = absorb_all(
            Overlap {
                policy: OverlapPolicy::Coalesce,
                max_queue: 10,
                coalesce_ms: 500,
                loop_parallel: 2,
            },
            burst(&["watch", "webhook", "watch"], Duration::from_millis(10)),
        );
        assert_eq!(kept, ["watch", "webhook", "watch"]);
    }

    /// Outside the window they are separate events again.
    #[test]
    fn coalesce_leaves_events_further_apart_alone() {
        let (kept, tally) = absorb_all(
            Overlap {
                policy: OverlapPolicy::Coalesce,
                max_queue: 10,
                coalesce_ms: 100,
                loop_parallel: 2,
            },
            burst(&["watch", "watch"], Duration::from_millis(400)),
        );
        assert_eq!(kept, ["watch", "watch"]);
        assert_eq!(tally.dropped, 0);
    }
}
