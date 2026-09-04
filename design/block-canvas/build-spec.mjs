// Builds spec.html from SPEC.md, inlining the figures under fig/.
// Handles exactly the Markdown subset SPEC.md uses. Run: node build-spec.mjs
import { readFileSync, writeFileSync } from 'node:fs';

const CANVAS = 'https://claude.ai/code/artifact/54c656f3-a5fe-47db-b858-e7b3794ee92e';
const TYPES = { text:'#56c7d6', tools:'#e0a458', memory:'#7e9ff0', data:'#a78bd0', stream:'#6fc98a', image:'#d77bd0', audio:'#dcc65b', file:'#7f93c9', exec:'#e8ebf0', any:'#8a93a3' };

const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

function inline(s) {
  let out = '';
  let i = 0;
  // code spans first, so nothing inside them is touched
  const parts = s.split(/(`[^`]+`)/);
  for (const p of parts) {
    if (p.startsWith('`') && p.endsWith('`')) {
      const t = p.slice(1, -1);
      out += TYPES[t] ? `<code class="ty" style="--ty:${TYPES[t]}">${esc(t)}</code>` : `<code>${esc(t)}</code>`;
    } else {
      let x = esc(p);
      x = x.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
      x = x.replace(/\*([^*]+)\*/g, '<em>$1</em>');
      x = x.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');
      x = x.replace(/&lt;(https?:\/\/[^&]+)&gt;/g, '<a href="$1">$1</a>');
      x = x.replace(/§(\d+(?:\.\d+)?)/g, '<a class="ref" href="#s$1">§$1</a>');
      out += x;
    }
  }
  return out;
}

const slug = (h) => {
  const m = h.match(/^(\d+)(?:\.(\d+))?\.?\s/);
  if (m) return 's' + m[1] + (m[2] ? '.' + m[2] : '');
  return h.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '');
};

function render(md) {
  const lines = md.split('\n');
  const html = [];
  const toc = [];
  let i = 0;
  let skip = false; // skip the Contents section; the sidebar replaces it
  let figN = 0;
  const para = [];
  const flush = () => { if (para.length) { html.push(`<p>${inline(para.join(' '))}</p>`); para.length = 0; } };
  while (i < lines.length) {
    const l = lines[i];
    if (/^## /.test(l)) { flush(); skip = /^## Contents/.test(l); }
    if (skip) { i++; continue; }
    if (/^# /.test(l)) { flush(); html.push(`<h1>${inline(l.slice(2))}</h1>`); i++; continue; }
    if (/^## /.test(l)) { const t = l.slice(3); const id = slug(t); toc.push({ id, t, lvl: 2 }); html.push(`<h2 id="${id}">${inline(t)}</h2>`); i++; continue; }
    if (/^### /.test(l)) { flush(); const t = l.slice(4); const id = slug(t); toc.push({ id, t, lvl: 3 }); html.push(`<h3 id="${id}">${inline(t)}</h3>`); i++; continue; }
    if (/^---\s*$/.test(l)) { flush(); i++; continue; }
    if (/^!\[/.test(l)) {
      flush();
      const m = l.match(/^!\[([^\]]*)\]\(([^)]+)\)/);
      const b64 = readFileSync(m[2]).toString('base64');
      figN++;
      html.push(`<figure id="fig${figN}"><a href="${CANVAS}" title="Open on the design canvas"><img src="data:image/png;base64,${b64}" alt="${esc(m[1])}" loading="lazy"></a><figcaption>${inline(m[1])}</figcaption></figure>`);
      i++; continue;
    }
    if (/^```/.test(l)) {
      flush();
      const buf = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) buf.push(lines[i++]);
      i++;
      html.push(`<pre><code>${esc(buf.join('\n'))}</code></pre>`);
      continue;
    }
    if (/^\|/.test(l)) {
      flush();
      const rows = [];
      while (i < lines.length && /^\|/.test(lines[i])) rows.push(lines[i++]);
      const cells = (r) => r.replace(/^\||\|$/g, '').split('|').map(c => c.trim());
      const head = cells(rows[0]);
      const body = rows.slice(2).map(cells);
      html.push(`<div class="tbl"><table><thead><tr>${head.map(c => `<th>${inline(c)}</th>`).join('')}</tr></thead><tbody>${body.map(r => `<tr>${r.map(c => `<td>${inline(c)}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`);
      continue;
    }
    if (/^(\d+)\. /.test(l) || /^- /.test(l)) {
      flush();
      const ordered = /^\d+\. /.test(l);
      const items = [];
      while (i < lines.length && (/^(\d+)\. /.test(lines[i]) || /^- /.test(lines[i]))) {
        let item = lines[i].replace(/^(\d+\. |- )/, '');
        i++;
        while (i < lines.length && /^ {2,}\S/.test(lines[i])) item += ' ' + lines[i++].trim();
        items.push(item);
      }
      html.push(`<${ordered ? 'ol' : 'ul'}>${items.map(it => `<li>${inline(it)}</li>`).join('')}</${ordered ? 'ol' : 'ul'}>`);
      continue;
    }
    if (/^\s*$/.test(l)) { flush(); i++; continue; }
    para.push(l.trim());
    i++;
  }
  flush();
  return { body: html.join('\n'), toc };
}

const md = readFileSync('SPEC.md', 'utf8');
const { body, toc } = render(md);

const nav = toc.filter(t => t.lvl === 2).map(t => `<a href="#${t.id}">${inline(t.t)}</a>`).join('');

const page = `<title>Block Canvas Spec</title>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@500;600;700&family=Source+Serif+4:ital,opsz,wght@0,8..60,400;0,8..60,600;1,8..60,400&family=JetBrains+Mono:wght@400;600&display=swap">
<style>
:root {
  --paper:#f2f4f6; --surface:#ffffff; --ink:#14181e; --mid:#58616e; --low:#8a93a0; --line:#d6dbe2; --soft:#e7ebef;
  --accent:#0e7c8c; --accent-ink:#0a5c68; --amber:#2e7d4f; --code-bg:#e9edf1; --fig-bg:#0d0f13;
  --sans:'Space Grotesk','Helvetica Neue',Arial,sans-serif;
  --serif:'Source Serif 4',Georgia,'Times New Roman',serif;
  --mono:'JetBrains Mono',ui-monospace,'SF Mono',Menlo,monospace;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --paper:#0d0f13; --surface:#14171c; --ink:#e8ebf0; --mid:#98a2ae; --low:#5f6875; --line:#262b33; --soft:#1a1e25;
    --accent:#56c7d6; --accent-ink:#8fdce7; --amber:#6fc98a; --code-bg:#1a1e25; --fig-bg:#08090b;
  }
}
:root[data-theme="dark"] {
  --paper:#0d0f13; --surface:#14171c; --ink:#e8ebf0; --mid:#98a2ae; --low:#5f6875; --line:#262b33; --soft:#1a1e25;
  --accent:#56c7d6; --accent-ink:#8fdce7; --amber:#6fc98a; --code-bg:#1a1e25; --fig-bg:#08090b;
}
body { background:var(--paper); color:var(--ink); font-family:var(--serif); font-size:16px; line-height:1.6; margin:0; -webkit-font-smoothing:antialiased; }
a { color:var(--accent-ink); text-decoration:none; border-bottom:1px solid color-mix(in srgb, var(--accent) 40%, transparent); }
a:hover { border-bottom-color:var(--accent); }
a:focus-visible { outline:2px solid var(--accent); outline-offset:2px; }
.wrap { display:grid; grid-template-columns:240px minmax(0,1fr); gap:48px; max-width:1400px; margin:0 auto; padding:40px 32px 96px; }
nav { position:sticky; top:24px; align-self:start; display:flex; flex-direction:column; gap:2px; font-family:var(--sans); font-size:13px; }
nav .k { font-family:var(--mono); font-size:10px; letter-spacing:.16em; text-transform:uppercase; color:var(--low); margin:0 0 10px 10px; }
nav a { border:0; color:var(--mid); padding:5px 10px; border-radius:6px; line-height:1.3; }
nav a:hover { color:var(--ink); background:var(--soft); }
nav .canvas { margin-top:18px; padding:12px 10px; border-top:1px solid var(--line); color:var(--accent-ink); }
main { min-width:0; }
.banner { display:flex; flex-wrap:wrap; align-items:center; gap:10px 18px; padding:12px 16px; margin-bottom:28px; border:1px solid var(--line); border-radius:8px; background:var(--surface); font-family:var(--sans); font-size:13px; color:var(--mid); }
.banner b { color:var(--ink); font-weight:600; }
.banner .pill { font-family:var(--mono); font-size:10.5px; letter-spacing:.08em; text-transform:uppercase; color:var(--amber); border:1px solid color-mix(in srgb, var(--amber) 45%, transparent); border-radius:4px; padding:2px 7px; }
h1 { font-family:var(--sans); font-weight:700; font-size:40px; line-height:1.05; letter-spacing:-.025em; margin:0 0 14px; text-wrap:balance; }
h2 { font-family:var(--sans); font-weight:600; font-size:24px; letter-spacing:-.015em; margin:64px 0 14px; padding-top:20px; border-top:1px solid var(--line); text-wrap:balance; }
h3 { font-family:var(--sans); font-weight:600; font-size:16.5px; letter-spacing:-.005em; margin:34px 0 8px; }
p, li { max-width:70ch; }
p { margin:0 0 14px; }
ul, ol { padding-left:22px; margin:0 0 16px; }
li { margin:0 0 6px; }
li::marker { color:var(--low); font-family:var(--sans); font-size:.9em; }
strong { font-weight:600; }
code { font-family:var(--mono); font-size:.82em; background:var(--code-bg); border-radius:4px; padding:.1em .38em; color:var(--ink); }
code.ty { padding-left:1.35em; position:relative; }
code.ty::before { content:''; position:absolute; left:.42em; top:50%; width:.55em; height:.55em; border-radius:50%; background:var(--ty); transform:translateY(-50%); box-shadow:0 0 0 1px var(--code-bg); }
pre { background:var(--code-bg); border-radius:8px; padding:14px 16px; overflow-x:auto; margin:0 0 18px; }
pre code { background:none; padding:0; font-size:12.5px; line-height:1.6; }
a.ref { border:0; font-family:var(--sans); font-size:.9em; color:var(--accent-ink); }
.tbl { overflow-x:auto; margin:0 0 20px; border:1px solid var(--line); border-radius:8px; background:var(--surface); }
table { border-collapse:collapse; width:100%; font-size:14px; font-family:var(--sans); }
th { text-align:left; font-family:var(--mono); font-size:10.5px; letter-spacing:.1em; text-transform:uppercase; color:var(--low); font-weight:600; padding:10px 12px; border-bottom:1px solid var(--line); white-space:nowrap; }
td { padding:9px 12px; border-bottom:1px solid var(--soft); vertical-align:top; line-height:1.45; }
tr:last-child td { border-bottom:0; }
td:first-child { white-space:nowrap; color:var(--ink); font-weight:500; }
td code { font-size:.85em; }
figure { margin:26px 0 30px; }
figure a { border:0; display:block; }
figure img { display:block; width:100%; height:auto; border-radius:8px; border:1px solid var(--line); background:var(--fig-bg); }
figcaption { font-family:var(--sans); font-size:13px; line-height:1.45; color:var(--mid); margin:10px 2px 0; max-width:80ch; }
figcaption code { font-size:.85em; }
@media (max-width: 1100px) { .wrap { grid-template-columns:1fr; gap:24px; } nav { position:static; flex-direction:row; flex-wrap:wrap; } nav .k, nav .canvas { display:none; } }
@media (prefers-reduced-motion: no-preference) { html { scroll-behavior:smooth; } }
</style>
<div class="wrap">
  <nav><div class="k">Sections</div>${nav}<a class="canvas" href="${CANVAS}">Open the design canvas &rarr;</a></nav>
  <main>
    <div class="banner"><span class="pill">v1.0 approved</span><span><b>Reference specification.</b> Approved 4 September 2026; changes from here are versioned.</span><span>Figures are rendered from the design canvas; click any to open it.</span></div>
    ${body}
  </main>
</div>
`;

writeFileSync('spec.html', page);
console.log('spec.html', (page.length / 1024 / 1024).toFixed(2), 'MB,', toc.length, 'headings');
