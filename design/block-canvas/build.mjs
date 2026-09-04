// Generates the .dc.html artboards for the Block Canvas UI mockups.
// Run:  node build.mjs
import { writeFileSync } from 'node:fs';

/* ---------------------------------------------------------------- tokens */

const SANS = "'Space Grotesk','Helvetica Neue',Arial,sans-serif";
const MONO = "'JetBrains Mono',ui-monospace,'SF Mono',Menlo,monospace";

const C = {
  ground: '#08090b',
  canvas: '#0d0f13',
  panel:  '#111419',
  bar:    '#0f1217',
  block:  '#191d24',
  field:  '#0b0d11',
  line:   '#242932',
  soft:   '#1a1e25',
  hi:     '#e8ebf0',
  mid:    '#98a2ae',
  low:    '#5f6875',
  faint:  '#39414c',
  accent: '#56c7d6',
  ok:     '#6fc98a',
  warn:   '#e0a458',
  err:    '#e0685f',
};

// port / data types -- the colour system every wire and port obeys
const T = {
  text:   '#56c7d6',
  tools:  '#e0a458',
  data:   '#a78bd0',
  stream: '#6fc98a',
  file:   '#7f93c9',
  image:  '#d77bd0',
  audio:  '#dcc65b',
  memory: '#7e9ff0',
  exec:   '#e8ebf0',
  any:    '#8a93a3',
};

const CAT = {
  models:'#56c7d6', capabilities:'#e0a458', runtimes:'#6fc98a',
  data:'#a78bd0', control:'#8a93a3', human:'#d97f8f',
  senses:'#dcc65b', memory:'#7e9ff0', actuators:'#e8865a', custom:'#c3ccd8',
};

const rgba = (hex, a) => {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${a})`;
};

/* ----------------------------------------------------------------- icons */

const ICONS = {
  llm:'<path d="M12 3l1.9 6.1L20 11l-6.1 1.9L12 19l-1.9-6.1L4 11l6.1-1.9z"/>',
  toolbox:'<rect x="3" y="8" width="18" height="12" rx="2"/><path d="M8 8V6.5A2.5 2.5 0 0 1 10.5 4h3A2.5 2.5 0 0 1 16 6.5V8"/><path d="M3 13h18"/>',
  terminal:'<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="M7.5 10l2.6 2-2.6 2"/><path d="M13 14h4"/>',
  python:'<path d="M9 8L5.5 12 9 16"/><path d="M15 8l3.5 4-3.5 4"/>',
  input:'<path d="M4 12h11"/><path d="M11.5 8.5L15 12l-3.5 3.5"/><path d="M19 4.5v15"/>',
  output:'<path d="M5 4.5v15"/><path d="M9 12h11"/><path d="M16.5 8.5L20 12l-3.5 3.5"/>',
  branch:'<circle cx="7" cy="5.5" r="2"/><circle cx="7" cy="18.5" r="2"/><circle cx="17" cy="9" r="2"/><path d="M7 7.5v9"/><path d="M9 5.5h3a3 3 0 0 1 3 3v.5"/>',
  search:'<circle cx="11" cy="11" r="6"/><path d="M15.5 15.5L20 20"/>',
  folder:'<path d="M3.5 6.5h6l2 2.2h9V19a1 1 0 0 1-1 1h-15a1 1 0 0 1-1-1z"/>',
  db:'<ellipse cx="12" cy="6" rx="7" ry="2.8"/><path d="M5 6v12c0 1.6 3.1 2.8 7 2.8s7-1.2 7-2.8V6"/><path d="M5 12c0 1.6 3.1 2.8 7 2.8s7-1.2 7-2.8"/>',
  http:'<circle cx="12" cy="12" r="8"/><path d="M4 12h16"/><ellipse cx="12" cy="12" rx="3.6" ry="8"/>',
  approve:'<circle cx="12" cy="12" r="8"/><path d="M8.5 12.2l2.4 2.4 4.6-5"/>',
  loop:'<path d="M4.5 10a7.5 7.5 0 0 1 12.8-3.5"/><path d="M19.5 14a7.5 7.5 0 0 1-12.8 3.5"/><path d="M17.5 3v4h-4"/><path d="M6.5 21v-4h4"/>',
  eye:'<path d="M2.5 12S6 6.5 12 6.5 21.5 12 21.5 12 18 17.5 12 17.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="2.5"/>',
  embed:'<circle cx="7.5" cy="7.5" r="1.7"/><circle cx="16.5" cy="7.5" r="1.7"/><circle cx="7.5" cy="16.5" r="1.7"/><circle cx="16.5" cy="16.5" r="1.7"/><path d="M9 8.5l6 7"/><path d="M15 8.5l-6 7"/>',
  braces:'<path d="M9.5 4c-2.5 0-2 5.5-4 8 2 2.5 1.5 8 4 8"/><path d="M14.5 4c2.5 0 2 5.5 4 8-2 2.5-1.5 8-4 8"/>',
  merge:'<circle cx="7" cy="19" r="2"/><circle cx="17" cy="5" r="2"/><path d="M7 17V11a4 4 0 0 1 4-4h4"/><path d="M13 4.5L15.5 7 13 9.5"/>',
  clock:'<circle cx="12" cy="12" r="8"/><path d="M12 7.5V12l3 1.8"/>',
  form:'<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M8 9h8"/><path d="M8 13h8"/><path d="M8 17h4"/>',
  shield:'<path d="M12 3.5l7.5 3v5.5c0 4.6-3.2 7.7-7.5 9.3-4.3-1.6-7.5-4.7-7.5-9.3V6.5z"/>',
  plug:'<path d="M8.5 3v5"/><path d="M15.5 3v5"/><path d="M6 8h12v3.5a6 6 0 0 1-12 0z"/><path d="M12 17.5V21"/>',
  bolt:'<path d="M13.5 3L5.5 13.5H11L10.5 21l8-10.5H13z"/>',
  chunk:'<rect x="3.5" y="5" width="17" height="4" rx="1.2"/><rect x="3.5" y="15" width="17" height="4" rx="1.2"/><path d="M8 11.5h8"/>',
  key:'<circle cx="8" cy="12" r="3.5"/><path d="M11.5 12H20"/><path d="M17 12v3.5"/><path d="M14 12v2.5"/>',
  note:'<rect x="4" y="3.5" width="16" height="17" rx="2"/><path d="M8 9h8"/><path d="M8 13h6"/>',
  chev:'<path d="M9 6l6 6-6 6"/>',
  plus:'<path d="M12 5v14"/><path d="M5 12h14"/>',
  play:'<path d="M7 4.5l12 7.5-12 7.5z"/>',
  stop:'<rect x="6" y="6" width="12" height="12" rx="1.5"/>',
  step:'<path d="M6 5l9 7-9 7z"/><path d="M18 5v14"/>',
  dots:'<circle cx="12" cy="5.5" r="1.3"/><circle cx="12" cy="12" r="1.3"/><circle cx="12" cy="18.5" r="1.3"/>',
  fit:'<path d="M4 9V5.5A1.5 1.5 0 0 1 5.5 4H9"/><path d="M15 4h3.5A1.5 1.5 0 0 1 20 5.5V9"/><path d="M20 15v3.5a1.5 1.5 0 0 1-1.5 1.5H15"/><path d="M9 20H5.5A1.5 1.5 0 0 1 4 18.5V15"/>',
  minus:'<path d="M6 12h12"/>',
  face:'<circle cx="12" cy="12" r="8.5"/><circle cx="9" cy="10.2" r="1.1" fill="currentColor" stroke="none"/><circle cx="15" cy="10.2" r="1.1" fill="currentColor" stroke="none"/><path d="M8.5 14.3c1 1.4 2.2 2.1 3.5 2.1s2.5-.7 3.5-2.1"/>',
  lamp:'<path d="M12 3a6 6 0 0 0-3.4 10.9c.6.5.9 1.2.9 2.1h5c0-.9.3-1.6.9-2.1A6 6 0 0 0 12 3z"/><path d="M9.5 19h5"/><path d="M10.5 21.5h3"/>',
  bell:'<path d="M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z"/><path d="M10 20.5a2 2 0 0 0 4 0"/>',
  code:'<path d="M9 8L5.5 12 9 16"/><path d="M15 8l3.5 4-3.5 4"/>',
  mark:'<circle cx="5.5" cy="6" r="2.2"/><circle cx="5.5" cy="18" r="2.2"/><circle cx="18.5" cy="12" r="2.6"/><path d="M7.7 6.9L16 11"/><path d="M7.7 17.1L16 13"/>',
};

const icon = (name, size = 14, color = 'currentColor', sw = 1.6) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="${color}" stroke-width="${sw}" stroke-linecap="round" stroke-linejoin="round" style="flex:none;display:block;">${ICONS[name] || ICONS.note}</svg>`;

/* -------------------------------------------------------------- document */

function doc(body, script = '') {
  return `<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;700&display=swap">
  <style>
    body { margin:0; background:${C.ground}; color:${C.hi}; font-family:${SANS}; -webkit-font-smoothing:antialiased; }
    a { color:${C.accent}; text-decoration:none; }
    a:hover { color:#93dde8; }
    @keyframes flow { to { stroke-dashoffset:-28; } }
    @keyframes breathe { 0%,100% { opacity:1; } 50% { opacity:.3; } }
  </style>
</helmet>
${body}
</x-dc>
${script}
</body>
</html>
`;
}

/* ------------------------------------------------------------- primitives */

function port({ kind, label, side, top, glow = false, dim = false }) {
  const c = T[kind];
  const ring = glow
    ? `box-shadow:0 0 0 4px ${rgba(c, 0.3)},0 0 16px ${rgba(c, 0.6)};`
    : `box-shadow:0 0 0 3px ${rgba(c, 0.12)};`;
  const pos = side === 'in'
    ? 'left:-5.5px;flex-direction:row;'
    : 'right:-5.5px;flex-direction:row-reverse;';
  return `<div style="position:absolute;${pos}top:${top}px;display:flex;align-items:center;gap:8px;opacity:${dim ? 0.3 : 1};">
<span style="width:11px;height:11px;border-radius:50%;background:${c};${ring}flex:none;"></span>
<span style="font-family:${MONO};font-size:9.5px;font-weight:500;color:${C.mid};letter-spacing:.03em;white-space:nowrap;">${label}</span>
</div>`;
}

function statusDot(state) {
  const map = {
    idle:  [C.faint, false],
    queued:[C.warn, false],
    running:[C.ok, true],
    ok:    [C.ok, false],
    error: [C.err, false],
    off:   [C.faint, false],
  };
  const [col, pulse] = map[state] || map.idle;
  return `<span style="width:7px;height:7px;border-radius:50%;background:${col};box-shadow:0 0 0 3px ${rgba(col, 0.16)};flex:none;${pulse ? 'animation:breathe 1.3s ease-in-out infinite;' : ''}"></span>`;
}

const viewToggle = (active, third = 'code') => `<div style="display:flex;gap:1px;padding:1px;background:${C.field};border:1px solid ${C.line};border-radius:4px;">${[['minus', 'compact'], ['form', 'summary'], ...(third ? [[third === 'stage' ? 'fit' : 'code', third]] : [])].map(([ic, v]) => `<div style="display:flex;align-items:center;justify-content:center;width:16px;height:14px;border-radius:3px;background:${v === active ? rgba(C.accent, 0.22) : 'transparent'};">${icon(ic, 10, v === active ? C.accent : C.low, 2)}</div>`).join('')}</div>`;
const grip = `<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 12 12" style="position:absolute;right:3px;bottom:3px;opacity:.7;"><path d="M11 1L1 11M11 6L6 11M11 10l-1 1" stroke="${C.mid}" stroke-width="1.4" stroke-linecap="round"/></svg>`;

function portRow(p) {
  const c = T[p.kind];
  const ring = p.glow
    ? `box-shadow:0 0 0 4px ${rgba(c, 0.3)},0 0 16px ${rgba(c, 0.6)};`
    : `box-shadow:0 0 0 3px ${rgba(c, 0.12)};`;
  const rev = p.side === 'out';
  return `<div style="display:flex;align-items:center;gap:8px;${rev ? 'flex-direction:row-reverse;margin-right:-6.5px;' : 'margin-left:-6.5px;'}opacity:${p.dim ? 0.3 : 1};">
<span style="width:11px;height:11px;border-radius:50%;background:${c};${ring}flex:none;"></span>
<span style="font-family:${MONO};font-size:9.5px;font-weight:500;color:${C.mid};letter-spacing:.03em;white-space:nowrap;">${p.label}</span>
</div>`;
}

// ports are rows: inputs down the left edge, outputs down the right, one row per index.
// row i's dot is centred at y = 51 + 24 * i from the block's top (see PY).
function portZone(ports = []) {
  const ins = ports.filter(p => p.side === 'in');
  const outs = ports.filter(p => p.side === 'out');
  const n = Math.max(ins.length, outs.length);
  const rows = [];
  for (let i = 0; i < n; i++) {
    const a = ins[i], b = outs[i];
    const hl = (a && a.hl) || (b && b.hl);
    rows.push(`<div style="display:flex;align-items:center;justify-content:space-between;height:24px;margin:0 -1px;padding:0 1px;${hl ? `background:${rgba(T.tools, 0.09)};border-top:1px solid ${rgba(T.tools, 0.35)};border-bottom:1px solid ${rgba(T.tools, 0.35)};` : ''}">${a ? portRow(a) : '<span></span>'}${b ? portRow(b) : '<span></span>'}</div>`);
  }
  return { n, html: n ? `<div style="padding:8px 0 4px;">${rows.join('')}</div>` : '' };
}
const PY = (b, i) => b.y + 51 + 24 * i;
const W = (B) => (from, oi, to, ii, kind, opt) => { const a = B[from], b = B[to]; return wire(a.x + a.w, PY(a, oi), b.x, PY(b, ii), kind, opt); };

function blockNode(o) {
  const c = o.color || T.any;
  const selected = !!o.selected;
  const running = o.state === 'running';
  const borderCol = selected ? C.accent : running ? rgba(C.ok, 0.5) : C.line;
  const shadow = selected
    ? `0 0 0 1px ${C.accent},0 0 0 5px ${rgba(C.accent, 0.15)},0 16px 38px rgba(0,0,0,.6)`
    : running
      ? `0 0 0 4px ${rgba(C.ok, 0.09)},0 12px 30px rgba(0,0,0,.5)`
      : `0 10px 26px rgba(0,0,0,.45)`;
  const ghost = o.ghost ? 'opacity:.5;' : '';
  const zone = portZone(o.ports);
  return `<div style="position:absolute;left:${o.x}px;top:${o.y}px;width:${o.w}px;background:${C.block};border:1px solid ${borderCol};border-radius:9px;box-shadow:${shadow};${ghost}">
  <div style="display:flex;align-items:center;gap:8px;height:31px;padding:0 10px;border-bottom:1px solid ${C.soft};border-radius:8px 8px 0 0;background:linear-gradient(180deg,${rgba(c, 0.13)},${rgba(c, 0.02)});">
    ${icon(o.icon, 13, c, 1.7)}
    <span style="font-size:12px;font-weight:600;letter-spacing:-.005em;color:${C.hi};white-space:nowrap;">${o.title}</span>
    <span style="flex:1;"></span>
    ${o.badge || ''}
    ${(selected || o.toggle) ? viewToggle(o.view || 'summary', o.third === undefined ? null : o.third) : ''}
    ${statusDot(o.state || 'idle')}
  </div>
  ${zone.html}
  ${o.body ? `<div style="position:relative;padding:${o.pad || (zone.n ? '4px 11px 12px' : '10px 11px 12px')};">${o.body}</div>` : ''}
  ${(selected || o.grip) ? grip : ''}
</div>`;
}

function wire(x1, y1, x2, y2, kind, opt = {}) {
  const c = T[kind] || kind;
  const dx = Math.max(48, Math.abs(x2 - x1) * 0.55, Math.abs(y2 - y1) * 0.22);
  const d = `M${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
  const halo = `<path d="${d}" fill="none" stroke="${c}" stroke-width="${opt.live ? 7 : 5}" stroke-linecap="round" opacity="${opt.live ? 0.16 : 0.09}"/>`;
  const core = `<path d="${d}" fill="none" stroke="${c}" stroke-width="${opt.width || 1.9}" stroke-linecap="round" opacity="${opt.opacity ?? 0.95}"${opt.dash ? ` stroke-dasharray="${opt.dash}"` : ''}${opt.live ? ' style="animation:flow .85s linear infinite;"' : ''}/>`;
  const mark = (kind === 'tools' || kind === 'memory') && !opt.nomark
    ? `<path d="M${x2 - 22} ${y2 - 3.5}l-3.5 3.5 3.5 3.5M${x2 - 13} ${y2 - 3.5}l3.5 3.5-3.5 3.5" fill="none" stroke="${c}" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" opacity=".95"/>`
    : '';
  return halo + core + mark;
}

/* ---------------------------------------------------------- form controls */

const sect = (title, inner, opt = {}) => `<div style="padding:14px 16px;border-bottom:1px solid ${C.soft};${opt.tint ? `background:${rgba(opt.tint, 0.04)};` : ''}">
  <div style="display:flex;align-items:center;gap:7px;margin-bottom:11px;">
    <span style="font-family:${MONO};font-size:9.5px;font-weight:700;letter-spacing:.13em;color:${opt.tint || C.low};text-transform:uppercase;">${title}</span>
    <span style="flex:1;height:1px;background:${C.soft};"></span>
    ${opt.right || ''}
  </div>
  ${inner}
</div>`;

const label = (t) => `<div style="font-size:10.5px;color:${C.low};margin-bottom:5px;letter-spacing:.01em;">${t}</div>`;

const field = (value, opt = {}) => `<div style="display:flex;align-items:center;gap:8px;height:30px;padding:0 9px;background:${C.field};border:1px solid ${C.line};border-radius:6px;">
  ${opt.icon ? icon(opt.icon, 12, C.low) : ''}
  <span style="flex:1;font-size:11.5px;${opt.mono ? `font-family:${MONO};font-size:10.5px;` : ''}color:${opt.muted ? C.low : C.hi};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${value}</span>
  ${opt.select ? `<span style="transform:rotate(90deg);opacity:.6;">${icon('chev', 11, C.low)}</span>` : ''}
  ${opt.suffix || ''}
</div>`;

const rowField = (l, v, opt = {}) => `<div style="margin-bottom:${opt.gap ?? 11}px;">${label(l)}${field(v, opt)}</div>`;

const slider = (l, v, pct, col = C.accent) => `<div style="margin-bottom:13px;">
  <div style="display:flex;align-items:baseline;justify-content:space-between;margin-bottom:7px;">
    <span style="font-size:10.5px;color:${C.low};">${l}</span>
    <span style="font-family:${MONO};font-size:10.5px;color:${C.hi};">${v}</span>
  </div>
  <div style="position:relative;height:3px;border-radius:2px;background:${C.line};">
    <div style="position:absolute;left:0;top:0;height:3px;width:${pct}%;border-radius:2px;background:${col};"></div>
    <div style="position:absolute;left:calc(${pct}% - 6px);top:-4.5px;width:12px;height:12px;border-radius:50%;background:${C.hi};box-shadow:0 1px 4px rgba(0,0,0,.6);"></div>
  </div>
</div>`;

const toggle = (on, col = C.accent) => `<div style="width:28px;height:16px;border-radius:8px;background:${on ? col : C.line};position:relative;flex:none;">
  <div style="position:absolute;top:2px;left:${on ? 14 : 2}px;width:12px;height:12px;border-radius:50%;background:${on ? '#0b0d11' : '#6c7583'};"></div>
</div>`;

const switchRow = (l, on, opt = {}) => `<div style="display:flex;align-items:center;gap:10px;padding:7px 0;">
  <div style="flex:1;">
    <div style="font-size:11.5px;color:${C.hi};">${l}</div>
    ${opt.hint ? `<div style="font-size:10px;color:${C.low};margin-top:2px;">${opt.hint}</div>` : ''}
  </div>
  ${toggle(on, opt.col)}
</div>`;

const chip = (t, col, opt = {}) => `<span style="display:inline-flex;align-items:center;gap:5px;height:20px;padding:0 8px;border-radius:5px;background:${rgba(col, opt.solid ? 0.9 : 0.12)};border:1px solid ${rgba(col, 0.3)};font-family:${MONO};font-size:9.5px;font-weight:600;letter-spacing:.04em;white-space:nowrap;color:${opt.solid ? '#0b0d11' : col};">${opt.dot ? `<span style="width:5px;height:5px;border-radius:50%;background:${col};"></span>` : ''}${t}</span>`;

const textBox = (text, h = 88) => `<div style="min-height:${h}px;padding:9px 10px;background:${C.field};border:1px solid ${C.line};border-radius:6px;font-family:${MONO};font-size:10.5px;line-height:1.6;color:#c3cad4;white-space:pre-wrap;">${text}</div>`;

const connRow = (ic, name, meta, kind, state) => `<div style="display:flex;align-items:center;gap:9px;padding:8px 9px;background:${C.field};border:1px solid ${state === 'pending' ? rgba(T.tools, 0.45) : C.line};${state === 'pending' ? 'border-style:dashed;' : ''}border-radius:6px;margin-bottom:6px;">
  ${icon(ic, 13, T[kind] || C.mid, 1.7)}
  <div style="flex:1;min-width:0;">
    <div style="font-size:11.5px;color:${C.hi};">${name}</div>
    <div style="font-family:${MONO};font-size:9.5px;color:${C.low};margin-top:1px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${meta}</div>
  </div>
  ${state === 'pending' ? chip('linking', T.tools) : statusDot(state)}
</div>`;

export { SANS, MONO, C, T, CAT, rgba, ICONS, icon, doc, port, statusDot, blockNode, wire, sect, label, field, rowField, slider, toggle, switchRow, chip, textBox, connRow };

/* ------------------------------------------------------------ shell chrome */

const SHELL_W = 1560, SHELL_H = 900;
const LIB_W = 264, INSP_W = 328, TOP_H = 46, BOT_H = 28;
const CW = SHELL_W - LIB_W - INSP_W;      // 968
const CH = SHELL_H - TOP_H - BOT_H;       // 826

const iconBtn = (name, opt = {}) => `<div style="display:flex;align-items:center;justify-content:center;width:26px;height:26px;border-radius:5px;${opt.on ? `background:${rgba(C.accent, 0.14)};` : ''}">${icon(name, 14, opt.on ? C.accent : C.mid, 1.6)}</div>`;

function topbar({ name = 'untitled.graph', saved = 'saved', running = false, elapsed = '', live = '', runtime = 'local &middot; ollama' } = {}) {
  const transport = live
    ? `<div style="display:flex;align-items:center;gap:8px;height:28px;padding:0 4px 0 10px;border-radius:6px;background:${rgba(C.ok, 0.13)};border:1px solid ${rgba(C.ok, 0.35)};">
        ${statusDot('running')}
        <span style="font-family:${MONO};font-size:10.5px;font-weight:600;color:${C.ok};letter-spacing:.03em;">live</span>
        <span style="font-family:${MONO};font-size:10.5px;color:${rgba(C.ok, 0.7)};">${live}</span>
        <div style="display:flex;align-items:center;justify-content:center;width:20px;height:20px;border-radius:4px;background:${rgba(C.err, 0.16)};margin-left:2px;">${icon('stop', 11, C.err, 0)}</div>
      </div>`
    : running
    ? `<div style="display:flex;align-items:center;gap:8px;height:28px;padding:0 4px 0 10px;border-radius:6px;background:${rgba(C.ok, 0.13)};border:1px solid ${rgba(C.ok, 0.35)};">
        ${statusDot('running')}
        <span style="font-family:${MONO};font-size:10.5px;font-weight:600;color:${C.ok};letter-spacing:.03em;">running</span>
        <span style="font-family:${MONO};font-size:10.5px;color:${rgba(C.ok, 0.7)};">${elapsed}</span>
        <div style="display:flex;align-items:center;justify-content:center;width:20px;height:20px;border-radius:4px;background:${rgba(C.err, 0.16)};margin-left:2px;">${icon('stop', 11, C.err, 0)}</div>
      </div>`
    : `<div style="display:flex;align-items:center;gap:7px;height:28px;padding:0 12px 0 10px;border-radius:6px;background:${C.accent};">
        ${icon('play', 11, '#08090b', 0)}
        <span style="font-size:11.5px;font-weight:600;color:#08090b;letter-spacing:.01em;">Run</span>
        <span style="font-family:${MONO};font-size:9.5px;color:${rgba('#08090b', 0.55)};margin-left:2px;">R</span>
      </div>`;
  return `<div style="display:flex;align-items:center;gap:10px;height:${TOP_H}px;flex:none;padding:0 12px;background:${C.bar};border-bottom:1px solid ${C.line};">
  <div style="display:flex;align-items:center;gap:9px;">
    ${icon('mark', 18, C.accent, 1.7)}
    <span style="width:1px;height:18px;background:${C.line};"></span>
    <span style="font-family:${MONO};font-size:11.5px;font-weight:500;color:${C.hi};">${name}</span>
    <span style="transform:rotate(90deg);opacity:.5;">${icon('chev', 11, C.low)}</span>
    <span style="font-size:10px;color:${C.low};">${saved}</span>
  </div>
  <span style="flex:1;"></span>
  <div style="display:flex;align-items:center;gap:8px;">
    ${transport}
    ${iconBtn('step')}
    <span style="width:1px;height:18px;background:${C.line};"></span>
    <div style="display:flex;align-items:center;gap:7px;height:26px;padding:0 9px;border-radius:6px;border:1px solid ${C.line};">
      <span style="width:6px;height:6px;border-radius:50%;background:${C.ok};"></span>
      <span style="font-family:${MONO};font-size:10px;color:${C.mid};">${runtime}</span>
    </div>
  </div>
  <span style="flex:1;"></span>
  <div style="display:flex;align-items:center;gap:4px;">
    <span style="font-family:${MONO};font-size:10px;color:${C.low};margin-right:6px;">100%</span>
    ${iconBtn('fit')}
    ${iconBtn('dots')}
    <div style="display:flex;align-items:center;gap:6px;height:26px;padding:0 11px;margin-left:6px;border-radius:6px;border:1px solid ${C.line};">
      <span style="font-size:11px;font-weight:500;color:${C.hi};">Deploy</span>
    </div>
  </div>
</div>`;
}

function statusbar(left, right) {
  return `<div style="display:flex;align-items:center;gap:14px;height:${BOT_H}px;flex:none;padding:0 12px;background:#0a0c10;border-top:1px solid ${C.line};font-family:${MONO};font-size:10px;color:${C.low};letter-spacing:.02em;">
  ${left}<span style="flex:1;"></span>${right}
</div>`;
}

/* ------------------------------------------------------------- block library */

const LIB = [
  { id:'models', name:'Models', blocks:[
    { n:'LLM', i:'llm', sig:'text, tools, mem &rarr; text', t:['text','tools','memory'] },
    { n:'Object detection', i:'eye', sig:'image &rarr; data', t:['image','data'] },
    { n:'Face recognition', i:'approve', sig:'image &rarr; data', t:['image','data'] },
    { n:'Speech to text', i:'note', sig:'audio &rarr; text', t:['audio','text'] },
    { n:'Text to speech', i:'form', sig:'text &rarr; audio', t:['text','audio'] },
    { n:'Embedding', i:'embed', sig:'text &rarr; data', t:['text','data'] },
    { n:'Classifier', i:'shield', sig:'text &rarr; data', t:['text','data'] },
    { n:'Affect', i:'face', sig:'text &rarr; data', t:['text','data'] },
  ]},
  { id:'capabilities', name:'Capabilities', blocks:[
    { n:'Toolbox', i:'toolbox', sig:'tools[] &rarr; tools', t:['tools'] },
    { n:'Web Search', i:'search', sig:'text &rarr; data', t:['text','data'] },
    { n:'File System', i:'folder', sig:'text &rarr; file', t:['text','file'] },
    { n:'MCP Server', i:'plug', sig:'&rarr; tools', t:['tools'] },
  ]},
  { id:'runtimes', name:'Runtimes', blocks:[
    { n:'Terminal', i:'terminal', sig:'&rarr; tools, stream', t:['tools','stream'] },
    { n:'Python', i:'python', sig:'&rarr; tools, data', t:['tools','data'] },
    { n:'Node', i:'bolt', sig:'&rarr; tools, data', t:['tools','data'] },
    { n:'SQL', i:'db', sig:'text &rarr; data', t:['text','data'] },
    { n:'HTTP Request', i:'http', sig:'text &rarr; data', t:['text','data'] },
  ]},
  { id:'senses', name:'Senses', blocks:[
    { n:'Webcam', i:'eye', sig:'&rarr; image', t:['image'] },
    { n:'Microphone', i:'note', sig:'&rarr; audio', t:['audio'] },
    { n:'Keyboard', i:'form', sig:'&rarr; text', t:['text'] },
    { n:'Schedule', i:'clock', sig:'&rarr; exec', t:['exec'] },
    { n:'Watch folder', i:'folder', sig:'&rarr; file', t:['file'] },
    { n:'Webhook', i:'http', sig:'&rarr; data', t:['data'] },
  ]},
  { id:'memory', name:'Memory', blocks:[
    { n:'Memory hub', i:'merge', sig:'memory[] &rarr; memory', t:['memory'] },
    { n:'Working memory', i:'braces', sig:'&rarr; memory', t:['memory'] },
    { n:'Long-term memory', i:'db', sig:'&rarr; memory', t:['memory'] },
    { n:'Episode log', i:'chunk', sig:'data &rarr; memory', t:['data','memory'] },
  ]},
  { id:'actuators', name:'Actuators', blocks:[
    { n:'Display', i:'form', sig:'text &rarr;', t:['text'] },
    { n:'Speaker', i:'note', sig:'audio &rarr;', t:['audio'] },
    { n:'USB device', i:'plug', sig:'&rarr; tools', t:['tools'] },
    { n:'Motors', i:'loop', sig:'&rarr; tools', t:['tools'] },
    { n:'GPIO', i:'bolt', sig:'&rarr; tools', t:['tools'] },
    { n:'Avatar', i:'face', sig:'audio, data &rarr; tools, stream', t:['audio','data','tools','stream'] },
    { n:'Status light', i:'lamp', sig:'data &rarr;', t:['data'] },
    { n:'Sound cue', i:'bell', sig:'data &rarr;', t:['data'] },
  ]},
  { id:'data', name:'Data', blocks:[
    { n:'Input', i:'input', sig:'&rarr; any', t:['any'] },
    { n:'Output', i:'output', sig:'any &rarr;', t:['any'] },
    { n:'Variable', i:'braces', sig:'any &rarr; any', t:['any'] },
    { n:'Chunker', i:'chunk', sig:'text &rarr; text', t:['text'] },
    { n:'Secret', i:'key', sig:'&rarr; text', t:['text'] },
  ]},
  { id:'control', name:'Control', blocks:[
    { n:'Loop', i:'loop', sig:'any &rarr; data, exec', t:['any','exec'] },
    { n:'Branch', i:'branch', sig:'any &rarr; a | b', t:['exec'] },
    { n:'Merge', i:'merge', sig:'any[] &rarr; any', t:['exec'] },
    { n:'Gate', i:'shield', sig:'any &rarr; any', t:['exec'] },
    { n:'Delay', i:'clock', sig:'any &rarr; any', t:['exec'] },
  ]},
  { id:'human', name:'Human', blocks:[
    { n:'Approval', i:'approve', sig:'any &rarr; any | halt', t:['exec'] },
    { n:'Form', i:'form', sig:'&rarr; data', t:['data'] },
    { n:'Notify', i:'note', sig:'text &rarr;', t:['text'] },
  ]},
  { id:'custom', name:'Custom', blocks:[
    { n:'door_check', i:'shield', sig:'image &rarr; data', t:['image','data'] },
  ]},
];

const typeDots = (ts) => `<span style="display:flex;align-items:center;gap:3px;flex:none;">${ts.map(k => `<span style="width:5px;height:5px;border-radius:50%;background:${T[k]};"></span>`).join('')}</span>`;

function libRow(b, catCol, opt = {}) {
  const bg = opt.state === 'placed' ? rgba(catCol, 0.1)
    : opt.state === 'drag' ? rgba(C.accent, 0.08) : 'transparent';
  const bd = opt.state === 'placed' ? rgba(catCol, 0.4)
    : opt.state === 'drag' ? rgba(C.accent, 0.4) : 'transparent';
  return `<div style="display:flex;align-items:center;gap:9px;height:28px;padding:0 8px;border-radius:6px;background:${bg};border:1px solid ${bd};">
  ${icon(b.i, 13, catCol, 1.7)}
  <span style="flex:1;font-size:11.5px;color:${C.hi};letter-spacing:-.005em;">${b.n}</span>
  ${opt.state === 'placed' ? chip('on canvas', catCol) : typeDots(b.t)}
</div>`;
}

function catHeader(cat, open, count) {
  const col = CAT[cat.id];
  return `<div style="display:flex;align-items:center;gap:7px;height:24px;padding:0 6px;">
  <span style="transform:rotate(${open ? 90 : 0}deg);opacity:.7;display:flex;">${icon('chev', 10, C.low, 2)}</span>
  <span style="width:6px;height:6px;border-radius:2px;background:${col};"></span>
  <span style="font-family:${MONO};font-size:9.5px;font-weight:700;letter-spacing:.12em;text-transform:uppercase;color:${C.mid};">${cat.name}</span>
  <span style="flex:1;"></span>
  <span style="font-family:${MONO};font-size:9.5px;color:${C.faint};">${count}</span>
</div>`;
}

function libraryPanel({ open = ['models','capabilities','runtimes'], placed = [], drag = null } = {}) {
  const groups = LIB.map(cat => {
    const isOpen = open.includes(cat.id);
    const rows = isOpen
      ? `<div style="display:flex;flex-direction:column;gap:1px;padding:2px 0 8px;">${cat.blocks.map(b => libRow(b, CAT[cat.id], {
          state: placed.includes(b.n) ? 'placed' : drag === b.n ? 'drag' : null,
        })).join('')}</div>`
      : '';
    return catHeader(cat, isOpen, cat.blocks.length) + rows;
  }).join('');
  return `<div style="width:${LIB_W}px;flex:none;display:flex;flex-direction:column;background:${C.panel};border-right:1px solid ${C.line};min-height:0;">
  <div style="padding:12px 12px 10px;border-bottom:1px solid ${C.soft};">
    <div style="display:flex;align-items:center;gap:8px;height:30px;padding:0 9px;background:${C.field};border:1px solid ${C.line};border-radius:6px;">
      ${icon('search', 13, C.low)}
      <span style="flex:1;font-size:11.5px;color:${C.faint};">Search blocks</span>
      <span style="font-family:${MONO};font-size:9.5px;color:${C.faint};border:1px solid ${C.line};border-radius:3px;padding:1px 4px;">&#8984;K</span>
    </div>
  </div>
  <div style="flex:1;overflow:hidden;padding:8px 10px;">${groups}</div>
  <div style="flex:none;padding:10px 12px;border-top:1px solid ${C.soft};display:flex;align-items:center;gap:7px;">
    ${icon('plus', 13, C.low, 1.8)}
    <span style="font-size:11px;color:${C.mid};">New custom block</span>
  </div>
</div>`;
}

/* -------------------------------------------------------------- inspector */

function inspector(inner, { title, sub, tabs, tab, icn, col } = {}) {
  const head = title
    ? `<div style="flex:none;padding:13px 16px 0;">
        <div style="display:flex;align-items:center;gap:9px;">
          <div style="display:flex;align-items:center;justify-content:center;width:26px;height:26px;border-radius:6px;background:${rgba(col || C.accent, 0.14)};">${icon(icn || 'note', 14, col || C.accent, 1.7)}</div>
          <div style="flex:1;min-width:0;">
            <div style="font-size:13px;font-weight:600;letter-spacing:-.01em;color:${C.hi};">${title}</div>
            <div style="font-family:${MONO};font-size:9.5px;color:${C.low};margin-top:2px;">${sub}</div>
          </div>
          ${icon('dots', 15, C.low, 1.6)}
        </div>
        ${tabs ? `<div style="display:flex;gap:16px;margin-top:13px;border-bottom:1px solid ${C.soft};">${tabs.map(t => `<div style="padding-bottom:8px;font-size:11.5px;color:${t === tab ? C.hi : C.low};border-bottom:1.5px solid ${t === tab ? C.accent : 'transparent'};margin-bottom:-1px;">${t}</div>`).join('')}</div>` : ''}
      </div>`
    : '';
  return `<div style="width:${INSP_W}px;flex:none;display:flex;flex-direction:column;background:${C.panel};border-left:1px solid ${C.line};min-height:0;">
  ${head}
  <div style="flex:1;overflow:hidden;">${inner}</div>
</div>`;
}

/* ------------------------------------------------------------------ shell */

function shell({ top, library, canvas, insp, status }) {
  return `<div style="width:${SHELL_W}px;height:${SHELL_H}px;display:flex;flex-direction:column;background:${C.ground};overflow:hidden;font-family:${SANS};">
  ${top}
  <div style="flex:1;display:flex;min-height:0;">
    ${library}
    ${canvas}
    ${insp}
  </div>
  ${status}
</div>`;
}

function stage({ svg = '', nodes = '', overlay = '', h = CH }) {
  return `<div style="flex:1;position:relative;overflow:hidden;background-color:${C.canvas};background-image:radial-gradient(circle at 1px 1px, rgba(255,255,255,.055) 1px, transparent 0);background-size:22px 22px;">
  <svg xmlns="http://www.w3.org/2000/svg" width="${CW}" height="${h}" viewBox="0 0 ${CW} ${h}" style="position:absolute;left:0;top:0;pointer-events:none;">${svg}</svg>
  ${nodes}
  ${overlay}
</div>`;
}

const zoomPill = `<div style="position:absolute;left:14px;bottom:14px;display:flex;align-items:center;gap:2px;height:30px;padding:0 6px;background:${rgba('#12161c', 0.92)};border:1px solid ${C.line};border-radius:8px;box-shadow:0 8px 24px rgba(0,0,0,.5);">
  <div style="display:flex;align-items:center;justify-content:center;width:22px;height:22px;color:${C.mid};font-size:14px;line-height:1;">&minus;</div>
  <span style="font-family:${MONO};font-size:10px;color:${C.hi};padding:0 4px;">100%</span>
  <div style="display:flex;align-items:center;justify-content:center;width:22px;height:22px;color:${C.mid};font-size:13px;line-height:1;">+</div>
  <span style="width:1px;height:14px;background:${C.line};margin:0 3px;"></span>
  ${icon('fit', 13, C.mid)}
</div>`;

function minimap(rects) {
  return `<div style="position:absolute;right:14px;bottom:14px;width:138px;height:88px;background:${rgba('#0a0c10', 0.92)};border:1px solid ${C.line};border-radius:8px;box-shadow:0 8px 24px rgba(0,0,0,.5);overflow:hidden;">
  ${rects.map(r => `<div style="position:absolute;left:${r[0]}px;top:${r[1]}px;width:${r[2]}px;height:${r[3]}px;border-radius:2px;background:${rgba(r[4], 0.65)};"></div>`).join('')}
  <div style="position:absolute;left:6px;top:6px;width:104px;height:66px;border:1px solid ${rgba(C.accent, 0.55)};border-radius:3px;"></div>
</div>`;
}

/* ------------------------------------------------- shared inspector bodies */

const dashedHint = (t, s, col = C.accent) => `<div style="margin:14px 16px 0;padding:11px 12px;border:1px dashed ${rgba(col, 0.4)};border-radius:8px;background:${rgba(col, 0.04)};">
  <div style="font-size:11.5px;font-weight:600;color:${C.mid};margin-bottom:4px;">${t}</div>
  <div style="font-size:10.5px;line-height:1.55;color:${C.low};">${s}</div>
</div>`;

const emptyRow = (t, s) => `<div style="padding:12px;border:1px dashed ${C.line};border-radius:7px;text-align:center;">
  <div style="font-size:11px;color:${C.mid};">${t}</div>
  <div style="font-size:10px;color:${C.low};margin-top:3px;line-height:1.5;">${s}</div>
</div>`;

const plusChip = `<div style="display:flex;align-items:center;justify-content:center;width:18px;height:18px;border-radius:4px;border:1px solid ${C.line};">${icon('plus', 10, C.mid, 2)}</div>`;

const btn = (t, opt = {}) => `<div style="display:flex;align-items:center;justify-content:center;gap:7px;height:30px;border-radius:6px;${opt.primary ? `background:${C.accent};color:#08090b;` : `border:1px solid ${C.line};color:${C.hi};`}${opt.danger ? `border-color:${rgba(C.err, 0.45)};color:${C.err};` : ''}font-size:11.5px;font-weight:${opt.primary ? 600 : 500};flex:1;">${opt.icon ? icon(opt.icon, 12, opt.primary ? '#08090b' : opt.danger ? C.err : C.mid, 1.7) : ''}${t}</div>`;

const GRAPH_BODY = [
  dashedHint('Nothing selected', 'The panel falls back to graph-wide settings. Select a block, a wire, or several blocks to change what appears here.'),
  sect('Graph', rowField('Name', 'customer-triage.graph', { mono: true }) + rowField('Description', 'What does this graph do?', { muted: true, gap: 0 })),
  sect('Execution', rowField('Runtime', 'Local machine', { select: true, icon: 'terminal' }) + rowField('Concurrency', '4 parallel', { select: true }) + rowField('Timeout', '120 s', { gap: 0 })),
  sect('Defaults', rowField('Model provider', 'Ollama &middot; local', { select: true, icon: 'llm' }) + rowField('Default model', 'llama3.2:3b', { select: true, gap: 0 })),
  sect('Env &amp; secrets', emptyRow('No secrets bound', 'Add one to expose it to Terminal and HTTP blocks'), { right: plusChip }),
].join('');

const LLM_BODY = [
  sect('Model', rowField('Provider', 'Ollama &middot; local', { select: true, icon: 'llm' }) + rowField('Model', 'llama3.2:3b', { select: true }) + rowField('Endpoint', 'http://127.0.0.1:11434', { mono: true, gap: 0 })),
  sect('Sampling', slider('Temperature', '0.70', 70) + slider('Top-p', '0.90', 90) + rowField('Max tokens', '2048', { mono: true, gap: 0 })),
  sect('System prompt', textBox('You triage build failures. Read the\nerror, run the smallest command that\nconfirms the cause, then answer.', 62)),
  sect('Tools', connRow('terminal', 'Terminal', 'shell.exec &middot; sandboxed', 'tools', 'ok')
    + connRow('python', 'Python', 'python.exec &middot; 3.12', 'tools', 'ok')
    + connRow('toolbox', 'Toolbox', 'tools &rarr; llm.tools', 'tools', 'pending')
    + `<div style="height:5px;"></div>` + rowField('Tool choice', 'auto', { select: true, gap: 0 }),
    { tint: T.tools, right: chip('2 + 1', T.tools) }),
].join('');

const TERMINAL_BODY = [
  sect('Command', textBox('cargo build --target \\\n  aarch64-unknown-linux-gnu', 46) + `<div style="height:11px;"></div>` + rowField('Shell', '/bin/bash', { select: true, mono: true, gap: 0 })),
  sect('Working directory', field('~/projects/tandem', { mono: true, icon: 'folder' })),
  sect('Safety', switchRow('Sandbox filesystem', true, { hint: 'read-only outside the working directory', col: C.err })
    + switchRow('Warn before run', true, { hint: 'shows the command first &mdash; never blocks', col: C.warn })
    + `<div style="height:9px;"></div>${label('Allowed commands')}<div style="display:flex;flex-wrap:wrap;gap:5px;">${chip('cargo', C.err)}${chip('rg', C.err)}${chip('git', C.err)}${plusChip}</div>`,
    { tint: C.err, right: chip('elevated', C.err, { dot: true }) }),
  sect('Limits', rowField('Timeout', '90 s') + rowField('Max output', '64 KB') + switchRow('Capture stderr', true)),
  `<div style="padding:12px 16px;display:flex;gap:9px;align-items:flex-start;">
    ${icon('shield', 14, C.err, 1.7)}
    <div style="font-size:10.5px;line-height:1.55;color:${C.low};">This block runs commands on the host machine. Every setting above is yours to loosen &mdash; the panel shows them first, it never enforces them.</div>
  </div>`,
].join('');

const WIRE_BODY = [
  sect('Endpoints', `<div style="display:flex;flex-direction:column;gap:7px;">
    ${connRow('toolbox', 'Toolbox', 'out &middot; tools', 'tools', 'ok')}
    <div style="display:flex;align-items:center;gap:8px;padding-left:5px;">${icon('chev', 12, T.tools, 2)}<span style="flex:1;height:1px;background:${rgba(T.tools, 0.35)};"></span></div>
    ${connRow('llm', 'LLM', 'in &middot; tools', 'tools', 'ok')}
  </div>`),
  sect('Type', `<div style="display:flex;align-items:center;gap:8px;margin-bottom:9px;">${chip('tools', T.tools, { dot: true })}<span style="font-size:10.5px;color:${C.low};">exact match &mdash; no cast needed</span></div>${rowField('Transform', 'None', { select: true, gap: 0 })}`),
  sect('Debug', switchRow('Watch value', true) + `<div style="height:7px;"></div>` + textBox('[\n  { "name": "terminal.run", "args": 1 },\n  { "name": "python.exec", "args": 2 }\n]', 74), { right: chip('live', C.ok, { dot: true }) }),
  `<div style="padding:14px 16px;display:flex;gap:8px;">${btn('Insert block', { icon: 'plus' })}${btn('Delete', { danger: true })}</div>`,
].join('');

const alignBtn = (paths) => `<div style="display:flex;align-items:center;justify-content:center;height:30px;border:1px solid ${C.line};border-radius:6px;"><svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="${C.mid}" stroke-width="1.6" stroke-linecap="round">${paths}</svg></div>`;

const MULTI_BODY = [
  sect('Selection', `<div style="display:flex;flex-direction:column;gap:6px;">
    ${connRow('terminal', 'Terminal', 'runtimes &middot; shell.exec', 'stream', 'ok')}
    ${connRow('python', 'Python', 'runtimes &middot; python.exec', 'data', 'ok')}
    ${connRow('toolbox', 'Toolbox', 'capabilities &middot; tools', 'tools', 'ok')}
  </div>`, { right: chip('3 blocks', C.accent) }),
  sect('Arrange', `<div style="display:grid;grid-template-columns:repeat(4, minmax(0, 1fr));gap:6px;">
    ${alignBtn('<path d="M4 4v16"/><path d="M8 8h9"/><path d="M8 16h5"/>')}
    ${alignBtn('<path d="M12 4v16"/><path d="M7.5 8h9"/><path d="M9.5 16h5"/>')}
    ${alignBtn('<path d="M20 4v16"/><path d="M7 8h9"/><path d="M11 16h5"/>')}
    ${alignBtn('<path d="M4 12h16"/><path d="M8 7v10"/><path d="M16 9v6"/>')}
  </div><div style="height:9px;"></div>${rowField('Spacing', '24 px between blocks', { select: true, gap: 0 })}`),
  sect('Group', `<div style="display:flex;align-items:center;gap:10px;padding:11px 12px;border:1px solid ${C.line};border-radius:7px;">
    ${icon('merge', 15, C.accent, 1.7)}
    <div style="flex:1;"><div style="font-size:11.5px;color:${C.hi};">Collapse into subgraph</div><div style="font-size:10px;color:${C.low};margin-top:2px;">one Toolbox block, ports preserved</div></div>
    <span style="font-family:${MONO};font-size:9.5px;color:${C.faint};border:1px solid ${C.line};border-radius:3px;padding:1px 4px;">&#8984;G</span>
  </div>`),
  sect('Shared settings', switchRow('Enabled', true) + rowField('Retry policy', '3 attempts, backoff', { select: true, gap: 0 })
    + `<div style="margin-top:10px;font-size:10px;color:${C.low};line-height:1.5;">2 of 11 settings are common to this selection. Everything else stays per-block.</div>`),
].join('');

/* ============================================================ 1. EmptyShell */

const emptyOverlay = `<div style="position:absolute;left:50%;top:46%;transform:translate(-50%,-50%);width:430px;display:flex;flex-direction:column;align-items:center;gap:20px;">
  <div style="width:100%;height:158px;border:1.5px dashed #2b323d;border-radius:14px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:11px;background:${rgba(C.accent, 0.018)};">
    <div style="display:flex;align-items:center;justify-content:center;width:38px;height:38px;border-radius:10px;border:1px solid ${C.line};background:${C.block};">${icon('plus', 17, C.mid, 1.8)}</div>
    <div style="font-size:13px;color:${C.mid};">Drag a block from the library</div>
    <div style="font-family:${MONO};font-size:10.5px;color:${C.faint};">or press &#8984;K to search all 27</div>
  </div>
  <div style="display:flex;align-items:center;gap:8px;">
    <span style="font-size:10.5px;color:${C.faint};">Start from</span>
    ${['Blank agent', 'Terminal assistant', 'Python data run'].map(t => `<div style="height:26px;display:flex;align-items:center;padding:0 11px;border:1px solid ${C.line};border-radius:13px;font-size:11px;color:${C.mid};">${t}</div>`).join('')}
  </div>
</div>`;

const EMPTY = doc(shell({
  top: topbar({ name: 'untitled.graph', saved: 'new' }),
  library: libraryPanel({ open: ['models', 'capabilities', 'runtimes'] }),
  canvas: stage({ overlay: emptyOverlay + zoomPill }),
  insp: inspector(GRAPH_BODY, { title: 'Graph', sub: 'untitled.graph', icn: 'mark', col: C.accent, tabs: ['Settings', 'Variables', 'Runs'], tab: 'Settings' }),
  status: statusbar('0 blocks &middot; 0 wires', 'ready &middot; local runtime'),
}));

/* =================================================================== 2. Main */

const MB = {
  input: { x: 28, y: 110, w: 178 },
  terminal: { x: 26, y: 418, w: 186 },
  python: { x: 26, y: 570, w: 186 },
  toolbox: { x: 250, y: 452, w: 196 },
  llm: { x: 560, y: 104, w: 236 },
};
const mw = W(MB);
const toolRow = (ic, t, extra = '') => `<div style="display:flex;align-items:center;gap:7px;height:21px;padding:0 7px;border-radius:5px;background:${C.field};border:1px solid ${C.soft};">${icon(ic, 11, CAT.runtimes, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${C.mid};">${t}</span>${extra}</div>`;

const mainNodes = [
  blockNode({ ...MB.input, icon: 'input', color: CAT.data, title: 'Input', state: 'idle',
    body: field('"triage ticket #4192"', { mono: true }),
    ports: [{ kind: 'text', label: 'text', side: 'out', dim: true }] }),
  blockNode({ ...MB.terminal, icon: 'terminal', color: CAT.runtimes, title: 'Terminal', state: 'idle',
    body: label('command') + field('cargo build', { mono: true }),
    ports: [{ kind: 'tools', label: 'tool', side: 'out' }] }),
  blockNode({ ...MB.python, icon: 'python', color: CAT.runtimes, title: 'Python', state: 'idle',
    body: label('source') + field('analyse.py', { mono: true }),
    ports: [{ kind: 'tools', label: 'tool', side: 'out' }] }),
  blockNode({ ...MB.toolbox, icon: 'toolbox', color: CAT.capabilities, title: 'Toolbox', state: 'idle',
    badge: chip('2', T.tools),
    body: `<div style="display:flex;flex-direction:column;gap:5px;">${toolRow('terminal', 'terminal.run')}${toolRow('python', 'python.exec')}<div style="font-family:${MONO};font-size:9px;color:${C.faint};padding-left:2px;">exposes 4 functions</div></div>`,
    ports: [
      { kind: 'tools', label: 'terminal', side: 'in' },
      { kind: 'tools', label: 'python', side: 'in' },
      { kind: 'tools', label: 'tools', side: 'out', glow: true },
    ] }),
  blockNode({ ...MB.llm, icon: 'llm', color: CAT.models, title: 'LLM', state: 'idle', selected: true,
    body: label('model') + field('llama3.2:3b', { mono: true, select: true })
      + `<div style="margin-top:9px;font-family:${MONO};font-size:9.5px;line-height:1.6;color:${C.faint};">You triage build failures. Read<br>the error, run the smallest&#8230;</div>`,
    ports: [
      { kind: 'text', label: 'prompt', side: 'in', dim: true },
      { kind: 'text', label: 'context', side: 'in', dim: true },
      { kind: 'tools', label: 'tools', side: 'in', glow: true, hl: true },
      { kind: 'text', label: 'text', side: 'out', dim: true },
      { kind: 'data', label: 'calls', side: 'out', dim: true },
    ] }),
].join('\n');

const snapX = MB.llm.x, snapY = PY(MB.llm, 2);
const mainSvg = [
  mw('input', 0, 'llm', 0, 'text', { opacity: 0.5 }),
  mw('terminal', 0, 'toolbox', 0, 'tools'),
  mw('python', 0, 'toolbox', 1, 'tools'),
  mw('toolbox', 0, 'llm', 2, 'tools', { live: true, dash: '7 6', width: 2.3, nomark: true }),
  `<circle cx="${snapX}" cy="${snapY}" r="10" fill="none" stroke="${T.tools}" stroke-width="1.6" opacity=".95"/>`,
  `<circle cx="${snapX}" cy="${snapY}" r="16" fill="none" stroke="${T.tools}" stroke-width="1" opacity=".35"/>`,
].join('');

const dragCursor = `<div style="position:absolute;left:${snapX + 4}px;top:${snapY + 4}px;">
  <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" style="filter:drop-shadow(0 2px 4px rgba(0,0,0,.8));"><path d="M5.5 3l12 8.2-5.4 1.1 2.6 5.6-2.4 1.1-2.6-5.6-4.2 3.6z" fill="${C.hi}" stroke="#0b0d11" stroke-width="1.2"/></svg>
  <div style="position:absolute;right:22px;top:-4px;display:flex;align-items:center;gap:6px;height:24px;padding:0 9px;border-radius:6px;background:${rgba('#12161c', 0.96)};border:1px solid ${rgba(T.tools, 0.5)};box-shadow:0 8px 20px rgba(0,0,0,.6);white-space:nowrap;">
    <span style="width:6px;height:6px;border-radius:50%;background:${T.tools};"></span>
    <span style="font-family:${MONO};font-size:10px;color:${C.hi};">toolbox.tools &rarr; llm.tools</span>
  </div>
</div>`;

const MAIN = doc(shell({
  top: topbar({ name: 'customer-triage.graph', saved: 'edited &middot; 2m ago' }),
  library: libraryPanel({ open: ['models', 'capabilities', 'runtimes'], placed: ['LLM', 'Toolbox', 'Terminal', 'Python'] }),
  canvas: stage({ svg: mainSvg, nodes: mainNodes, overlay: dragCursor + zoomPill + minimap([
    [9, 15, 19, 6, CAT.data], [9, 39, 20, 8, CAT.runtimes], [9, 51, 20, 8, CAT.runtimes],
    [33, 42, 21, 10, CAT.capabilities], [66, 14, 25, 12, CAT.models],
  ]) }),
  insp: inspector(LLM_BODY, { title: 'LLM', sub: 'models &middot; llm.chat', icn: 'llm', col: CAT.models, tabs: ['Settings', 'Ports', 'Runs'], tab: 'Settings' }),
  status: statusbar('5 blocks &middot; 3 wires &middot; 1 linking', 'release to connect &mdash; esc cancels'),
}));

/* ================================================================ 3. Running */

const RUN_STAGE_H = CH - 176;

const RB = {
  input: { x: 24, y: 110, w: 168 },
  terminal: { x: 24, y: 300, w: 168 },
  python: { x: 24, y: 450, w: 168 },
  toolbox: { x: 236, y: 392, w: 180 },
  llm: { x: 470, y: 56, w: 240 },
  report: { x: 770, y: 96, w: 180 },
};
const rw = W(RB);
const runNodes = [
  blockNode({ ...RB.input, icon: 'input', color: CAT.data, title: 'Input', state: 'ok',
    body: field('ticket #4192', { mono: true }),
    ports: [{ kind: 'text', label: 'text', side: 'out' }] }),
  blockNode({ ...RB.terminal, icon: 'terminal', color: CAT.runtimes, title: 'Terminal', state: 'ok',
    badge: chip('42 lines', C.ok),
    body: label('last run') + field('exit 101', { mono: true, suffix: `<span style="font-family:${MONO};font-size:9px;color:${C.err};">err</span>` }),
    ports: [{ kind: 'tools', label: 'tool', side: 'out' }] }),
  blockNode({ ...RB.python, icon: 'python', color: CAT.runtimes, title: 'Python', state: 'queued',
    body: label('source') + field('analyse.py', { mono: true, muted: true }),
    ports: [{ kind: 'tools', label: 'tool', side: 'out' }] }),
  blockNode({ ...RB.toolbox, icon: 'toolbox', color: CAT.capabilities, title: 'Toolbox', state: 'ok',
    body: `<div style="display:flex;flex-direction:column;gap:5px;">${toolRow('terminal', 'terminal.run', `<span style="flex:1;"></span>${statusDot('ok')}`)}${toolRow('python', 'python.exec', `<span style="flex:1;"></span>${statusDot('queued')}`)}<div style="font-family:${MONO};font-size:9px;color:${C.faint};padding-left:2px;">1 call &middot; 1 queued</div></div>`,
    ports: [
      { kind: 'tools', label: 'terminal', side: 'in' },
      { kind: 'tools', label: 'python', side: 'in' },
      { kind: 'tools', label: 'tools', side: 'out' },
    ] }),
  blockNode({ ...RB.llm, icon: 'llm', color: CAT.models, title: 'LLM', state: 'running',
    badge: chip('streaming', C.ok, { dot: true }),
    body: `<div style="display:flex;align-items:baseline;justify-content:space-between;margin-bottom:7px;">
      <span style="font-family:${MONO};font-size:10px;color:${C.mid};">412 tok</span>
      <span style="font-family:${MONO};font-size:10px;color:${C.ok};">38 tok/s</span>
    </div>
    <div style="height:3px;border-radius:2px;background:${C.line};margin-bottom:10px;"><div style="width:64%;height:3px;border-radius:2px;background:${C.ok};"></div></div>
    <div style="display:flex;align-items:center;gap:7px;height:23px;padding:0 8px;border-radius:5px;background:${rgba(T.tools, 0.1)};border:1px solid ${rgba(T.tools, 0.32)};">
      ${icon('toolbox', 11, T.tools, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${T.tools};">calling terminal.run</span>
    </div>`,
    ports: [
      { kind: 'text', label: 'prompt', side: 'in' },
      { kind: 'tools', label: 'tools', side: 'in' },
      { kind: 'text', label: 'text', side: 'out' },
      { kind: 'data', label: 'calls', side: 'out' },
    ] }),
  blockNode({ ...RB.report, icon: 'output', color: CAT.data, title: 'Report', state: 'queued',
    body: `<div style="font-family:${MONO};font-size:9.5px;line-height:1.65;color:${C.faint};">waiting for llm.text<br>&#8230;</div>`,
    ports: [{ kind: 'text', label: 'text', side: 'in' }] }),
].join('\n');

const runSvg = [
  rw('input', 0, 'llm', 0, 'text', { live: true, dash: '7 7' }),
  rw('terminal', 0, 'toolbox', 0, 'tools', { live: true, dash: '6 6' }),
  rw('python', 0, 'toolbox', 1, 'tools', { opacity: 0.35 }),
  rw('toolbox', 0, 'llm', 1, 'tools', { live: true, dash: '7 7', width: 2.2 }),
  rw('llm', 0, 'report', 0, 'text', { opacity: 0.28, dash: '4 5' }),
].join('');

const consoleLine = (t, src, srcCol, msg) => `<div style="display:flex;gap:12px;">
  <span style="color:${C.faint};width:52px;flex:none;">${t}</span>
  <span style="color:${srcCol};width:52px;flex:none;">${src}</span>
  <span style="color:#b3bcc7;flex:1;">${msg}</span>
</div>`;

const consoleDrawer = `<div style="height:176px;flex:none;display:flex;flex-direction:column;background:#0a0c10;border-top:1px solid ${C.line};">
  <div style="display:flex;align-items:center;gap:18px;height:33px;flex:none;padding:0 14px;border-bottom:1px solid ${C.soft};">
    ${['Console', 'Trace', 'Variables'].map((t, i) => `<span style="font-size:11.5px;color:${i === 0 ? C.hi : C.low};border-bottom:1.5px solid ${i === 0 ? C.accent : 'transparent'};height:33px;display:flex;align-items:center;">${t}</span>`).join('')}
    <span style="flex:1;"></span>
    ${chip('1 warning', C.warn, { dot: true })}
    <span style="font-family:${MONO};font-size:10px;color:${C.low};">clear</span>
    <span style="transform:rotate(-90deg);display:flex;">${icon('chev', 12, C.low, 2)}</span>
  </div>
  <div style="flex:1;padding:9px 14px;font-family:${MONO};font-size:10.5px;line-height:1.85;overflow:hidden;">
    ${consoleLine('00:00.2', 'graph', C.accent, 'run started &middot; 5 blocks &middot; seed 41827')}
    ${consoleLine('00:00.3', 'input', CAT.data, '&rarr; "triage ticket #4192: build fails on arm64"')}
    ${consoleLine('00:00.4', 'llm', CAT.models, 'llama3.2:3b &middot; 3 tools bound from toolbox')}
    ${consoleLine('00:01.8', 'llm', CAT.models, 'tool_call terminal.run { cmd: "cargo build --target aarch64&#8230;" }')}
    ${consoleLine('00:03.1', 'term', C.err, 'exit 101 &middot; 42 lines captured &middot; ld: cannot find -lssl')}
    ${consoleLine('00:03.2', 'llm', CAT.models, 'resumed &middot; streaming<span style="color:' + C.ok + ';">&#9612;</span>')}
  </div>
</div>`;

const RUNNING = doc(shell({
  top: topbar({ name: 'customer-triage.graph', saved: 'run #18', running: true, elapsed: '00:04.2' }),
  library: libraryPanel({ open: ['models', 'capabilities', 'runtimes'], placed: ['LLM', 'Toolbox', 'Terminal', 'Python'] }),
  canvas: `<div style="width:${CW}px;flex:none;display:flex;flex-direction:column;min-height:0;">
    ${stage({ svg: runSvg, nodes: runNodes, overlay: zoomPill + minimap([
      [9, 15, 18, 6, CAT.data], [9, 34, 18, 8, CAT.runtimes], [9, 46, 18, 8, CAT.runtimes],
      [31, 38, 19, 10, CAT.capabilities], [56, 10, 26, 11, CAT.models], [88, 13, 19, 7, CAT.data],
    ]), h: RUN_STAGE_H })}
    ${consoleDrawer}
  </div>`,
  insp: inspector([
    sect('Progress', `<div style="display:flex;flex-direction:column;gap:1px;">
      ${[['Input', 'input', CAT.data, 'ok', '4 ms'], ['Terminal', 'terminal', CAT.runtimes, 'ok', '1.9 s'], ['Toolbox', 'toolbox', CAT.capabilities, 'ok', '2 ms'], ['LLM', 'llm', CAT.models, 'running', '2.4 s'], ['Report', 'output', CAT.data, 'queued', '&mdash;']].map(([n, i, c, s, t]) => `<div style="display:flex;align-items:center;gap:9px;height:30px;padding:0 8px;border-radius:6px;${s === 'running' ? `background:${rgba(C.ok, 0.07)};` : ''}">
        ${icon(i, 12, c, 1.7)}
        <span style="flex:1;font-size:11.5px;color:${s === 'queued' ? C.low : C.hi};">${n}</span>
        <span style="font-family:${MONO};font-size:10px;color:${C.low};">${t}</span>
        ${statusDot(s)}
      </div>`).join('')}
    </div>`, { right: chip('4 / 5', C.ok) }),
    sect('Live output', textBox('The arm64 build fails at link time:\nld cannot find -lssl. The target\nsysroot has no OpenSSL. Install&#8230;<span style="color:' + C.ok + ';">&#9612;</span>', 74)),
    sect('Usage', rowField('Tokens in', '1,208', { mono: true }) + rowField('Tokens out', '412', { mono: true }) + rowField('Cost', 'local &middot; no charge', { muted: true, gap: 0 })),
    `<div style="padding:14px 16px;display:flex;gap:8px;">${btn('Pause', { icon: 'clock' })}${btn('Stop run', { danger: true, icon: 'stop' })}</div>`,
  ].join(''), { title: 'Run #18', sub: 'started 4.2 s ago &middot; local', icn: 'play', col: C.ok, tabs: ['Run', 'Settings', 'Ports'], tab: 'Run' }),
  status: statusbar('5 blocks &middot; 4 wires &middot; 1 pending', '00:04.2 &middot; 412 tok &middot; 38 tok/s &middot; 0 errors'),
}));

/* ------------------------------------------------------------ sheet helpers */

function sheet({ w, h, kicker, title, body }) {
  return `<div style="width:${w}px;height:${h}px;box-sizing:border-box;padding:30px 32px;background:${C.ground};font-family:${SANS};display:flex;flex-direction:column;gap:22px;overflow:hidden;">
  <div>
    <div style="font-family:${MONO};font-size:10px;font-weight:700;letter-spacing:.17em;text-transform:uppercase;color:${C.accent};margin-bottom:7px;">${kicker}</div>
    <div style="font-size:19px;font-weight:600;letter-spacing:-.02em;color:${C.hi};">${title}</div>
  </div>
  ${body}
</div>`;
}

function panelInner(inner, m) {
  return `<div style="display:flex;flex-direction:column;height:100%;min-height:0;">
  <div style="flex:none;padding:13px 16px 0;">
    <div style="display:flex;align-items:center;gap:9px;">
      <div style="display:flex;align-items:center;justify-content:center;width:26px;height:26px;border-radius:6px;background:${rgba(m.col, 0.14)};">${icon(m.icn, 14, m.col, 1.7)}</div>
      <div style="flex:1;min-width:0;">
        <div style="font-size:13px;font-weight:600;letter-spacing:-.01em;color:${C.hi};">${m.title}</div>
        <div style="font-family:${MONO};font-size:9.5px;color:${C.low};margin-top:2px;">${m.sub}</div>
      </div>
      ${icon('dots', 15, C.low, 1.6)}
    </div>
    <div style="display:flex;gap:16px;margin-top:13px;border-bottom:1px solid ${C.soft};">${m.tabs.map(t => `<div style="padding-bottom:8px;font-size:11.5px;color:${t === m.tab ? C.hi : C.low};border-bottom:1.5px solid ${t === m.tab ? C.accent : 'transparent'};margin-bottom:-1px;">${t}</div>`).join('')}</div>
  </div>
  <div style="flex:1;overflow:hidden;">${inner}</div>
</div>`;
}

const META = {
  graph: { title: 'Graph', sub: 'customer-triage.graph', icn: 'mark', col: C.accent, tabs: ['Settings', 'Variables', 'Runs'], tab: 'Settings' },
  llm: { title: 'LLM', sub: 'models &middot; llm.chat', icn: 'llm', col: CAT.models, tabs: ['Settings', 'Ports', 'Runs'], tab: 'Settings' },
  term: { title: 'Terminal', sub: 'runtimes &middot; shell.exec', icn: 'terminal', col: CAT.runtimes, tabs: ['Settings', 'Ports', 'Runs'], tab: 'Settings' },
  wire: { title: 'Connection', sub: 'toolbox.tools &rarr; llm.tools', icn: 'merge', col: T.tools, tabs: ['Settings', 'Debug'], tab: 'Settings' },
  multi: { title: '3 blocks', sub: 'terminal, python, toolbox', icn: 'dots', col: C.accent, tabs: ['Common', 'Arrange'], tab: 'Common' },
  tool: { title: 'Toolbox', sub: 'capabilities &middot; tools.bundle', icn: 'toolbox', col: CAT.capabilities, tabs: ['Settings', 'Ports', 'Runs'], tab: 'Settings' },
  input: { title: 'Input', sub: 'data &middot; graph.input', icn: 'input', col: CAT.data, tabs: ['Settings', 'Ports', 'Runs'], tab: 'Settings' },
};

/* ============================================================= 4. Inspector */

const INSP_COLS = [
  ['Nothing selected', 'graph-wide settings', GRAPH_BODY, META.graph, C.accent],
  ['LLM selected', 'a model and its tools', LLM_BODY, META.llm, CAT.models],
  ['Terminal selected', 'safety first, then limits', TERMINAL_BODY, META.term, CAT.runtimes],
  ['Wire selected', 'types and live payload', WIRE_BODY, META.wire, T.tools],
  ['3 blocks selected', 'only what they share', MULTI_BODY, META.multi, C.accent],
];

const INSPECTOR_SHEET = doc(sheet({
  w: 1800, h: 980, kicker: 'Right panel',
  title: 'One column, five selections &mdash; the panel is the state of the canvas',
  body: `<div style="display:flex;gap:22px;">
  ${INSP_COLS.map(([cap, note, body, meta, col]) => `<div style="width:328px;flex:none;display:flex;flex-direction:column;gap:10px;">
    <div style="display:flex;align-items:baseline;gap:8px;">
      <span style="width:6px;height:6px;border-radius:50%;background:${col};flex:none;transform:translateY(-1px);"></span>
      <span style="font-size:12px;font-weight:600;color:${C.hi};">${cap}</span>
      <span style="font-size:10.5px;color:${C.low};">${note}</span>
    </div>
    <div style="height:806px;background:${C.panel};border:1px solid ${C.line};border-radius:10px;overflow:hidden;">${panelInner(body, meta)}</div>
  </div>`).join('')}
</div>`,
}));

/* =============================================================== 5. Library */

const catCard = (cat) => `<div style="background:${C.panel};border:1px solid ${C.line};border-radius:10px;overflow:hidden;">
  <div style="display:flex;align-items:center;gap:8px;height:37px;padding:0 13px;border-bottom:1px solid ${C.soft};background:${rgba(CAT[cat.id], 0.07)};">
    <span style="width:7px;height:7px;border-radius:2px;background:${CAT[cat.id]};"></span>
    <span style="font-family:${MONO};font-size:10px;font-weight:700;letter-spacing:.13em;text-transform:uppercase;color:${CAT[cat.id]};">${cat.name}</span>
    <span style="flex:1;"></span>
    <span style="font-family:${MONO};font-size:9.5px;color:${C.faint};">${cat.blocks.length}</span>
  </div>
  <div style="padding:5px 8px 8px;">
    ${cat.blocks.map(b => `<div style="display:flex;align-items:center;gap:10px;height:33px;padding:0 5px;">
      ${icon(b.i, 14, CAT[cat.id], 1.7)}
      <span style="width:118px;flex:none;font-size:11.5px;color:${C.hi};">${b.n}</span>
      <span style="flex:1;font-family:${MONO};font-size:10px;color:${C.low};">${b.sig}</span>
      ${typeDots(b.t)}
    </div>`).join('')}
  </div>
</div>`;

const TYPE_DOC = [
  ['text', 'prompts, stdout, any string', 'text &middot; any'],
  ['tools', 'one callable, or a bundle of them', 'llm.tools &middot; toolbox'],
  ['memory', 'a store the model reads and writes', 'llm.memory &middot; hub'],
  ['data', 'structured json or a record', 'data &middot; any'],
  ['stream', 'output arriving incrementally', 'text &middot; data'],
  ['image', 'frames from a camera or a file', 'image &middot; any'],
  ['audio', 'samples from a microphone', 'audio &middot; any'],
  ['file', 'a path or blob on disk', 'file &middot; any'],
  ['exec', 'a trigger or control flow, never a value', 'exec'],
  ['any', 'accepts every type', 'everything'],
];
const byId = (id) => LIB.find(c => c.id === id);

const LIBRARY_SHEET = doc(sheet({
  w: 1470, h: 1250, kicker: 'Left panel',
  title: 'Nine categories plus yours, one type system',
  body: `<div style="display:flex;gap:26px;align-items:flex-start;">
  <div style="width:${LIB_W}px;flex:none;height:748px;border:1px solid ${C.line};border-radius:10px;overflow:hidden;display:flex;">${libraryPanel({ open: ['models', 'capabilities', 'runtimes'] })}</div>
  <div style="flex:1;display:grid;grid-template-columns:repeat(3, minmax(0, 1fr));gap:18px;align-content:start;">
    <div style="display:flex;flex-direction:column;gap:18px;">${catCard(byId('models'))}${catCard(byId('senses'))}${catCard(byId('human'))}${catCard(byId('custom'))}</div>
    <div style="display:flex;flex-direction:column;gap:18px;">${catCard(byId('capabilities'))}${catCard(byId('memory'))}${catCard(byId('control'))}</div>
    <div style="display:flex;flex-direction:column;gap:18px;">${catCard(byId('runtimes'))}${catCard(byId('actuators'))}${catCard(byId('data'))}</div>
  </div>
</div>
<div style="background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:15px 18px;">
  <div style="display:flex;align-items:center;gap:9px;margin-bottom:13px;">
    <span style="font-family:${MONO};font-size:10px;font-weight:700;letter-spacing:.13em;text-transform:uppercase;color:${C.mid};">Port types</span>
    <span style="flex:1;height:1px;background:${C.soft};"></span>
    <span style="font-size:10.5px;color:${C.low};">a wire is legal when its source type is accepted by the target port</span>
  </div>
  <div style="display:grid;grid-template-columns:repeat(5, minmax(0, 1fr));gap:16px 14px;">
    ${TYPE_DOC.map(([k, d, c]) => `<div>
      <div style="display:flex;align-items:center;gap:7px;margin-bottom:6px;">
        <span style="width:10px;height:10px;border-radius:50%;background:${T[k]};box-shadow:0 0 0 3px ${rgba(T[k], 0.14)};flex:none;"></span>
        <span style="font-family:${MONO};font-size:10.5px;font-weight:600;color:${T[k]};">${k}</span>
      </div>
      <div style="font-size:10.5px;line-height:1.5;color:${C.mid};margin-bottom:5px;">${d}</div>
      <div style="font-family:${MONO};font-size:9.5px;color:${C.faint};">&rarr; ${c}</div>
    </div>`).join('')}
  </div>
</div>`,
}));

/* ========================================================== 6. BlockAnatomy */

const callout = (x, y, w, t, align) => `<div style="position:absolute;left:${x}px;top:${y}px;width:${w}px;text-align:${align};font-size:10.5px;line-height:1.45;color:${C.mid};">${t}</div>`;

const AB = { x: 170, y: 60, w: 280 };
const anatomyBlock = blockNode({
  ...AB, toggle: true, grip: true, icon: 'llm', color: CAT.models, title: 'LLM', state: 'running',
  badge: chip('streaming', C.ok, { dot: true }),
  body: label('model') + field('llama3.2:3b', { mono: true, select: true })
    + `<div style="margin-top:9px;font-family:${MONO};font-size:9.5px;line-height:1.6;color:${C.faint};">The arm64 build fails at link<br>time: ld cannot find -lssl&#8230;</div>`,
  ports: [
    { kind: 'text', label: 'prompt', side: 'in' },
    { kind: 'text', label: 'context', side: 'in' },
    { kind: 'tools', label: 'tools', side: 'in' },
    { kind: 'text', label: 'text', side: 'out' },
    { kind: 'data', label: 'calls', side: 'out' },
  ],
});
const aHead = AB.y + 15, aIn = PY(AB, 1), aOut = PY(AB, 1), aBody = AB.y + 31 + 84 + 4 + 50, aGrip = AB.y + 31 + 84 + 4 + 109 - 8;

const STATES = [
  ['idle', 'idle', C.line, 'placed, never run'],
  ['queued', 'queued', C.line, 'waiting on an upstream block'],
  ['running', 'running', rgba(C.ok, 0.5), 'executing now &mdash; wires animate'],
  ['ok', 'done', C.line, 'produced a value this run'],
  ['error', 'error', rgba(C.err, 0.55), 'threw; the console holds the trace'],
  ['off', 'disabled', C.line, 'skipped, wires kept'],
  ['queued', 'breakpoint', rgba(C.warn, 0.55), 'the run pauses before this block'],
];

const ruleCard = (title, note, svg, col) => `<div style="flex:1;background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:13px 15px;">
  <div style="height:34px;margin-bottom:9px;">${svg}</div>
  <div style="font-size:11.5px;font-weight:600;color:${col};margin-bottom:4px;">${title}</div>
  <div style="font-size:10.5px;line-height:1.5;color:${C.low};">${note}</div>
</div>`;

const ruleSvg = (c1, c2, stroke, dash) => `<svg xmlns="http://www.w3.org/2000/svg" width="200" height="34" viewBox="0 0 200 34" style="display:block;">
  <circle cx="10" cy="17" r="5.5" fill="${c1}"/>
  <path d="M18 17 C 60 17, 120 17, 162 17" fill="none" stroke="${stroke}" stroke-width="2" stroke-linecap="round"${dash ? ` stroke-dasharray="${dash}"` : ''}/>
  <circle cx="170" cy="17" r="5.5" fill="${c2}"/>
</svg>`;

const ANATOMY = doc(sheet({
  w: 1500, h: 760, kicker: 'Vocabulary',
  title: 'A block, its ports, and the states it moves through',
  body: `<div style="display:flex;gap:28px;align-items:flex-start;">
  <div style="position:relative;width:620px;height:310px;flex:none;">
    <svg xmlns="http://www.w3.org/2000/svg" width="620" height="300" viewBox="0 0 620 300" style="position:absolute;left:0;top:0;">
      ${[[154, aHead, AB.x - 6], [154, aIn, AB.x - 8]].map(([x1, y, x2]) => `<path d="M${x1} ${y}H${x2}" stroke="${C.faint}" stroke-width="1"/>`).join('')}
      ${[[466, aHead, AB.x + AB.w - 2], [466, aOut, AB.x + AB.w + 8], [466, aBody, AB.x + AB.w + 2], [466, aGrip, AB.x + AB.w - 4]].map(([x1, y, x2]) => `<path d="M${x1} ${y}H${x2}" stroke="${C.faint}" stroke-width="1"/>`).join('')}
    </svg>
    ${anatomyBlock}
    ${callout(0, aHead - 22, 148, 'Category colour and icon &mdash; the block looks like its shelf in the library', 'right')}
    ${callout(0, aIn - 14, 148, 'Typed input ports &mdash; the label is the name the graph API uses', 'right')}
    ${callout(472, aHead - 8, 148, 'Run status; before it the view toggle (compact, summary, plus code or stage)', 'left')}
    ${callout(472, aOut - 8, 148, 'Typed output ports', 'left')}
    ${callout(472, aBody - 16, 148, 'Inline preview of the current value &mdash; no need to open the inspector', 'left')}
    ${callout(472, aGrip - 8, 148, 'Resize grip', 'left')}
  </div>
  <div style="flex:1;">
    <div style="font-family:${MONO};font-size:10px;font-weight:700;letter-spacing:.13em;text-transform:uppercase;color:${C.mid};margin-bottom:12px;">States</div>
    <div style="display:flex;flex-direction:column;gap:9px;">
      ${STATES.map(([s, name, bd, note]) => `<div style="display:flex;align-items:center;gap:14px;${name === 'disabled' ? 'opacity:.45;' : ''}">
        <div style="width:154px;flex:none;display:flex;align-items:center;gap:7px;height:30px;padding:0 9px;background:${C.block};border:1px solid ${bd};${name === 'disabled' ? 'border-style:dashed;' : ''}border-radius:7px;${name === 'breakpoint' ? `box-shadow:inset 3px 0 0 ${C.warn};` : ''}">
          ${icon('llm', 12, CAT.models, 1.7)}
          <span style="font-size:11px;color:${C.hi};">LLM</span>
          <span style="flex:1;"></span>
          ${statusDot(s)}
        </div>
        <span style="font-family:${MONO};font-size:10px;color:${C.hi};width:74px;flex:none;">${name}</span>
        <span style="flex:1;font-size:10.5px;color:${C.low};line-height:1.45;">${note}</span>
      </div>`).join('')}
    </div>
  </div>
</div>
<div style="display:flex;gap:18px;">
  ${ruleCard('Same type connects', 'Drop anywhere on the target block and it snaps to the first port that accepts the type.', ruleSvg(T.tools, T.tools, T.tools), C.ok)}
  ${ruleCard('Mismatched type is refused', 'Incompatible ports dim during the drag, so a wrong wire cannot be drawn in the first place.', ruleSvg(T.text, T.tools, rgba(C.err, 0.7), '5 5'), C.err)}
  ${ruleCard('any accepts everything', 'Input, Output and Variable take any type and pass it through unchanged.', ruleSvg(T.data, T.any, T.any), C.mid)}
  ${ruleCard('Tools reply, senses report', 'A handle wire is two-way: the call goes out, the reply comes back. Telemetry and faults leave on ports of their own.', `<svg xmlns="http://www.w3.org/2000/svg" width="200" height="34" viewBox="0 0 200 34" style="display:block;"><circle cx="10" cy="17" r="5.5" fill="${T.tools}"/><path d="M17 17H78" fill="none" stroke="${T.tools}" stroke-width="2" stroke-linecap="round"/><path d="M60 13.5l-3.5 3.5 3.5 3.5M69 13.5l3.5 3.5-3.5 3.5" fill="none" stroke="${T.tools}" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/><rect x="80" y="8" width="40" height="18" rx="4" fill="none" stroke="${C.mid}" stroke-width="1.5"/><path d="M122 12H162" fill="none" stroke="${T.stream}" stroke-width="2" stroke-linecap="round"/><path d="M122 22H150" fill="none" stroke="${T.exec}" stroke-width="2" stroke-linecap="round" stroke-dasharray="3 3"/><circle cx="170" cy="12" r="5.5" fill="${T.stream}"/><circle cx="158" cy="22" r="4" fill="${T.exec}"/></svg>`, T.tools)}
  ${ruleCard('Direct or bundled', 'A runtime wires straight into llm.tools for a simple run; a Toolbox bundles several and adds guards.', `<svg xmlns="http://www.w3.org/2000/svg" width="200" height="34" viewBox="0 0 200 34" style="display:block;"><circle cx="10" cy="9" r="5" fill="${T.tools}"/><path d="M17 9 C 70 9, 110 17, 162 17" fill="none" stroke="${T.tools}" stroke-width="2" stroke-linecap="round"/><circle cx="10" cy="26" r="5" fill="${T.tools}"/><path d="M17 26 C 40 26, 50 26, 70 26" fill="none" stroke="${T.tools}" stroke-width="2" stroke-linecap="round"/><rect x="72" y="20" width="26" height="12" rx="3" fill="none" stroke="${T.tools}" stroke-width="1.5"/><path d="M99 26 C 120 26, 140 17, 162 17" fill="none" stroke="${T.tools}" stroke-width="2" stroke-linecap="round"/><circle cx="170" cy="17" r="5.5" fill="${T.tools}"/></svg>`, T.tools)}
</div>`,
}));

/* =========================================================== 7. Interactive */

const ISTAGE = CH - 34;

function panelFlow(inner, m) {
  return `<div>
  <div style="padding:13px 16px 0;">
    <div style="display:flex;align-items:center;gap:9px;">
      <div style="display:flex;align-items:center;justify-content:center;width:26px;height:26px;border-radius:6px;background:${rgba(m.col, 0.14)};">${icon(m.icn, 14, m.col, 1.7)}</div>
      <div style="flex:1;min-width:0;">
        <div style="font-size:13px;font-weight:600;letter-spacing:-.01em;color:${C.hi};">${m.title}</div>
        <div style="font-family:${MONO};font-size:9.5px;color:${C.low};margin-top:2px;">${m.sub}</div>
      </div>
      ${icon('dots', 15, C.low, 1.6)}
    </div>
    <div style="display:flex;gap:16px;margin-top:13px;border-bottom:1px solid ${C.soft};">${m.tabs.map(t => `<div style="padding-bottom:8px;font-size:11.5px;color:${t === m.tab ? C.hi : C.low};border-bottom:1.5px solid ${t === m.tab ? C.accent : 'transparent'};margin-bottom:-1px;">${t}</div>`).join('')}</div>
  </div>
  ${inner}
</div>`;
}

const TOOL_BODY = [
  sect('Exposed functions', connRow('terminal', 'terminal.run', '1 required arg &middot; cmd', 'stream', 'ok') + connRow('python', 'python.exec', '2 args &middot; src, timeout', 'data', 'ok'), { right: chip('2', T.tools), tint: T.tools }),
  sect('Binding', rowField('Presented to', 'LLM &middot; llm.tools', { select: true, icon: 'llm' }) + switchRow('Describe from docstrings', true)),
  sect('Guards', switchRow('Confirm before each call', false) + switchRow('Log arguments', true, { hint: 'appears in the run trace' })),
].join('');

const IN_BODY = [
  dashedHint('Graph entry point', 'Whatever this block holds is what the run starts with.', CAT.data),
  sect('Value', textBox('triage ticket #4192: build fails\non arm64 since the ssl bump', 50) + `<div style="height:11px;"></div>` + rowField('Type', 'text', { select: true, gap: 0 })),
  sect('Source', rowField('Provided by', 'Manual run', { select: true }) + switchRow('Prompt on run', true, { hint: 'ask for the value each time' })),
].join('');

function iblock(o) {
  const c = o.color;
  const zone = portZone(o.ports);
  return `<div onClick="{{pick${o.h}}}" style="position:absolute;left:${o.x}px;top:${o.y}px;width:${o.w}px;background:${C.block};border:1px solid {{bd${o.h}}};border-radius:9px;box-shadow:{{sh${o.h}}};cursor:pointer;">
  <div style="display:flex;align-items:center;gap:8px;height:31px;padding:0 10px;border-bottom:1px solid ${C.soft};border-radius:8px 8px 0 0;background:linear-gradient(180deg,${rgba(c, 0.13)},${rgba(c, 0.02)});">
    ${icon(o.icon, 13, c, 1.7)}
    <span style="font-size:12px;font-weight:600;color:${C.hi};">${o.title}</span>
    <span style="flex:1;"></span>
    <span style="width:7px;height:7px;border-radius:50%;background:{{dot${o.h}}};box-shadow:0 0 0 3px {{ring${o.h}}};flex:none;"></span>
  </div>
  ${zone.html}
  <div style="position:relative;padding:4px 11px 12px;">${o.body}</div>
</div>`;
}

const IB = {
  input: { x: 40, y: 120, w: 178 },
  terminal: { x: 40, y: 430, w: 186 },
  toolbox: { x: 300, y: 470, w: 196 },
  llm: { x: 560, y: 110, w: 236 },
};
const iw = W(IB);
const iNodes = [
  iblock({ h: 'In', ...IB.input, icon: 'input', color: CAT.data, title: 'Input',
    body: field('"triage ticket #4192"', { mono: true }),
    ports: [{ kind: 'text', label: 'text', side: 'out' }] }),
  iblock({ h: 'Term', ...IB.terminal, icon: 'terminal', color: CAT.runtimes, title: 'Terminal',
    body: label('command') + field('cargo build', { mono: true }),
    ports: [{ kind: 'tools', label: 'tool', side: 'out' }] }),
  iblock({ h: 'Tool', ...IB.toolbox, icon: 'toolbox', color: CAT.capabilities, title: 'Toolbox',
    body: `<div style="display:flex;flex-direction:column;gap:5px;">${toolRow('terminal', 'terminal.run')}${toolRow('python', 'python.exec')}</div>`,
    ports: [
      { kind: 'tools', label: 'terminal', side: 'in' },
      { kind: 'tools', label: 'tools', side: 'out' },
    ] }),
  iblock({ h: 'Llm', ...IB.llm, icon: 'llm', color: CAT.models, title: 'LLM',
    body: label('model') + field('llama3.2:3b', { mono: true, select: true })
      + `<div style="margin-top:9px;font-family:${MONO};font-size:9.5px;line-height:1.6;color:${C.faint};">You triage build failures. Read<br>the error, run the smallest&#8230;</div>`,
    ports: [
      { kind: 'text', label: 'prompt', side: 'in' },
      { kind: 'text', label: 'context', side: 'in' },
      { kind: 'tools', label: 'tools', side: 'in' },
      { kind: 'text', label: 'text', side: 'out' },
      { kind: 'data', label: 'calls', side: 'out' },
    ] }),
].join('\n');

const iWires = (live) => [
  iw('input', 0, 'llm', 0, 'text', live ? { live: true, dash: '7 7' } : {}),
  iw('terminal', 0, 'toolbox', 0, 'tools', live ? { live: true, dash: '6 6' } : {}),
  iw('toolbox', 0, 'llm', 2, 'tools', live ? { live: true, dash: '7 7', width: 2.2 } : { width: 2.1 }),
].join('');

const hintPill = `<div style="position:absolute;left:16px;top:16px;display:flex;align-items:center;gap:9px;height:30px;padding:0 13px;border-radius:15px;background:${rgba('#12161c', 0.9)};border:1px solid ${C.line};">
  <span style="width:6px;height:6px;border-radius:50%;background:${C.accent};"></span>
  <span style="font-size:11px;color:${C.mid};">Click a block to change the panel &middot; press Run to put the graph in flight</span>
</div>`;

const INTERACTIVE = doc(`<div onClick="{{clearSel}}" style="width:${SHELL_W}px;height:${SHELL_H}px;display:flex;flex-direction:column;background:${C.ground};overflow:hidden;font-family:${SANS};">
  <div style="display:flex;align-items:center;gap:10px;height:${TOP_H}px;flex:none;padding:0 12px;background:${C.bar};border-bottom:1px solid ${C.line};">
    <div style="display:flex;align-items:center;gap:9px;">
      ${icon('mark', 18, C.accent, 1.7)}
      <span style="width:1px;height:18px;background:${C.line};"></span>
      <span style="font-family:${MONO};font-size:11.5px;color:${C.hi};">customer-triage.graph</span>
    </div>
    <span style="flex:1;"></span>
    <div onClick="{{toggleRun}}" style="display:flex;align-items:center;gap:8px;height:28px;padding:0 13px;border-radius:6px;background:{{runBg}};border:1px solid {{runBd}};cursor:pointer;">
      <span style="width:7px;height:7px;border-radius:50%;background:{{runDot}};"></span>
      <span style="font-size:11.5px;font-weight:600;color:{{runFg}};">{{runLabel}}</span>
    </div>
    <span style="flex:1;"></span>
    <div style="display:flex;align-items:center;gap:7px;height:26px;padding:0 9px;border-radius:6px;border:1px solid ${C.line};">
      <span style="width:6px;height:6px;border-radius:50%;background:${C.ok};"></span>
      <span style="font-family:${MONO};font-size:10px;color:${C.mid};">local &middot; ollama</span>
    </div>
  </div>
  <div style="flex:1;display:flex;min-height:0;">
    ${libraryPanel({ open: ['models', 'capabilities', 'runtimes'], placed: ['LLM', 'Toolbox', 'Terminal'] })}
    <div style="width:${CW}px;flex:none;display:flex;flex-direction:column;min-height:0;">
      <div style="flex:1;position:relative;overflow:hidden;background-color:${C.canvas};background-image:radial-gradient(circle at 1px 1px, rgba(255,255,255,.055) 1px, transparent 0);background-size:22px 22px;">
        <svg xmlns="http://www.w3.org/2000/svg" width="${CW}" height="${ISTAGE}" viewBox="0 0 ${CW} ${ISTAGE}" style="position:absolute;left:0;top:0;pointer-events:none;opacity:{{opStatic}};">${iWires(false)}</svg>
        <svg xmlns="http://www.w3.org/2000/svg" width="${CW}" height="${ISTAGE}" viewBox="0 0 ${CW} ${ISTAGE}" style="position:absolute;left:0;top:0;pointer-events:none;opacity:{{opLive}};">${iWires(true)}</svg>
        ${iNodes}
        ${hintPill}
        ${zoomPill}
      </div>
      <div style="height:34px;flex:none;display:flex;align-items:center;gap:14px;padding:0 14px;background:#0a0c10;border-top:1px solid ${C.line};font-family:${MONO};font-size:10px;">
        <span style="color:${C.faint};">console</span>
        <span style="color:{{logCol}};">{{logLine}}</span>
      </div>
    </div>
    <div style="width:${INSP_W}px;flex:none;background:${C.panel};border-left:1px solid ${C.line};overflow:hidden;">
      <sc-if value="{{isNone}}" hint-placeholder-val="{{ true }}">${panelFlow(GRAPH_BODY, META.graph)}</sc-if>
      <sc-if value="{{isLlm}}" hint-placeholder-val="{{ false }}">${panelFlow(LLM_BODY, META.llm)}</sc-if>
      <sc-if value="{{isTerm}}" hint-placeholder-val="{{ false }}">${panelFlow(TERMINAL_BODY, META.term)}</sc-if>
      <sc-if value="{{isTool}}" hint-placeholder-val="{{ false }}">${panelFlow(TOOL_BODY, META.tool)}</sc-if>
      <sc-if value="{{isIn}}" hint-placeholder-val="{{ false }}">${panelFlow(IN_BODY, META.input)}</sc-if>
    </div>
  </div>
  <div style="display:flex;align-items:center;gap:14px;height:${BOT_H}px;flex:none;padding:0 12px;background:#0a0c10;border-top:1px solid ${C.line};font-family:${MONO};font-size:10px;color:${C.low};">
    <span>4 blocks &middot; 3 wires</span><span style="flex:1;"></span><span>{{statusRight}}</span>
  </div>
</div>`, `<script data-dc-script data-props='{"$preview":{"width":1560,"height":900}}'>
class Component extends DCLogic {
  renderVals() {
    var self = this;
    var st = this.state || {};
    var sel = st.sel || null;
    var running = !!st.running;
    var SEL = '0 0 0 1px #56c7d6,0 0 0 5px rgba(86,199,214,.15),0 16px 38px rgba(0,0,0,.6)';
    var OFF = '0 10px 26px rgba(0,0,0,.45)';
    var out = {
      isNone: sel === null,
      isLlm: sel === 'llm',
      isTerm: sel === 'term',
      isTool: sel === 'tool',
      isIn: sel === 'in',
      opStatic: running ? 0 : 1,
      opLive: running ? 1 : 0,
      runBg: running ? 'rgba(111,201,138,0.13)' : '#56c7d6',
      runBd: running ? 'rgba(111,201,138,0.35)' : '#56c7d6',
      runDot: running ? '#6fc98a' : '#08090b',
      runFg: running ? '#6fc98a' : '#08090b',
      runLabel: running ? 'Running 00:03.1' : 'Run',
      logCol: running ? '#e0685f' : '#5f6875',
      logLine: running ? '00:03.1  term   exit 101 - ld: cannot find -lssl' : 'idle - press Run to execute the graph',
      statusRight: running ? '412 tok - 38 tok/s - 0 errors' : 'ready - local runtime',
      toggleRun: function (e) { if (e && e.stopPropagation) { e.stopPropagation(); } self.setState({ running: !running }); },
      clearSel: function () { self.setState({ sel: null }); }
    };
    var ids = { In: 'in', Llm: 'llm', Tool: 'tool', Term: 'term' };
    Object.keys(ids).forEach(function (K) {
      var id = ids[K];
      out['pick' + K] = function (e) { if (e && e.stopPropagation) { e.stopPropagation(); } self.setState({ sel: id }); };
      out['sh' + K] = sel === id ? SEL : OFF;
      out['bd' + K] = sel === id ? '#56c7d6' : (running && id === 'llm' ? 'rgba(111,201,138,0.5)' : '#242932');
      out['dot' + K] = running ? '#6fc98a' : '#39414c';
      out['ring' + K] = running ? 'rgba(111,201,138,0.16)' : 'rgba(57,65,76,0.16)';
    });
    return out;
  }
}
</script>`);


/* ============================================================ 8. Continuous */

function loopFrame({ x, y, w, h, title, counter, inner, ports = [] }) {
  return `<div style="position:absolute;left:${x}px;top:${y}px;width:${w}px;height:${h}px;border:1.5px dashed ${rgba(CAT.control, 0.55)};border-radius:12px;background:${rgba('#ffffff', 0.018)};">
  <div style="display:flex;align-items:center;gap:8px;height:30px;padding:0 12px;border-bottom:1px dashed ${rgba(CAT.control, 0.3)};">
    ${icon('loop', 13, CAT.control, 1.8)}
    <span style="font-size:12px;font-weight:600;color:${C.hi};">${title}</span>
    <span style="font-family:${MONO};font-size:9.5px;color:${C.low};">as item</span>
    <span style="flex:1;"></span>
    ${counter}
  </div>
  ${inner}
  ${ports.map(port).join('')}
</div>`;
}

const CB = {
  webhook: { x: 24, y: 60, w: 186 },
  watch: { x: 24, y: 200, w: 186 },
  schedule: { x: 24, y: 420, w: 186 },
  frame: { x: 250, y: 50, w: 480, h: 300 },
  classify: { x: 300, y: 100, w: 200 },
  branch: { x: 530, y: 112, w: 186 },
  notify: { x: 770, y: 90, w: 176 },
  archive: { x: 770, y: 220, w: 176 },
  digest: { x: 480, y: 420, w: 200 },
  notify2: { x: 770, y: 400, w: 176 },
};
const cw = W(CB);
const rel = (b, f) => ({ ...b, x: b.x - f.x, y: b.y - f.y });
const frameInY = PY(CB.classify, 0);

const contNodes = [
  blockNode({ ...CB.webhook, icon: 'http', color: CAT.senses, title: 'Webhook', state: 'running',
    badge: chip('listening', C.ok, { dot: true }),
    body: field('POST /hooks/ticket', { mono: true }) + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">:8787 &middot; 0 events today</div>`,
    ports: [{ kind: 'data', label: 'event', side: 'out' }] }),
  blockNode({ ...CB.watch, icon: 'folder', color: CAT.senses, title: 'Watch folder', state: 'running',
    badge: chip('3/min', C.ok, { dot: true }),
    body: field('~/inbox/*.eml', { mono: true }) + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">1,204 events &middot; last 12 s ago</div>`,
    ports: [{ kind: 'file', label: 'file', side: 'out' }] }),
  blockNode({ ...CB.schedule, icon: 'clock', color: CAT.senses, title: 'Schedule', state: 'queued',
    badge: chip('armed', C.warn),
    body: field('every 15 min', { mono: true, select: true }) + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">next in 4:12 &middot; jitter &plusmn;2 min</div>`,
    ports: [{ kind: 'exec', label: 'tick', side: 'out' }] }),
  loopFrame({ ...CB.frame, title: 'For each',
    counter: `<div style="display:flex;align-items:center;gap:6px;">${chip('3 / 7', CAT.control)}${chip('queue 4', C.warn)}${chip('parallel 2', C.mid)}</div>`,
    ports: [{ kind: 'any', label: 'items', side: 'in', top: frameInY - CB.frame.y - 7 }],
    inner: blockNode({ ...rel(CB.classify, CB.frame), icon: 'llm', color: CAT.models, title: 'Classify', state: 'running',
        body: label('model') + field('llama3.2:3b', { mono: true, select: true }) + `<div style="margin-top:8px;font-family:${MONO};font-size:9.5px;line-height:1.55;color:${C.faint};">urgent | routine | spam</div>`,
        ports: [{ kind: 'text', label: 'item', side: 'in' }, { kind: 'data', label: 'label', side: 'out' }] })
      + blockNode({ ...rel(CB.branch, CB.frame), icon: 'branch', color: CAT.control, title: 'Branch', state: 'ok',
        body: `<div style="font-family:${MONO};font-size:9.5px;line-height:1.7;color:${C.faint};">label == "urgent"</div>`,
        ports: [{ kind: 'any', label: 'in', side: 'in' }, { kind: 'exec', label: 'urgent', side: 'out' }, { kind: 'exec', label: 'else', side: 'out' }] })
      + `<div style="position:absolute;left:14px;right:14px;bottom:12px;display:flex;align-items:center;gap:10px;font-family:${MONO};font-size:9.5px;color:${C.faint};">
          ${statusDot('running')}<span>iteration 3 &middot; re-arm64-build.eml &middot; 1.2 s</span><span style="flex:1;"></span><span>done 2 &middot; failed 0</span>
        </div>` }),
  blockNode({ ...CB.notify, icon: 'note', color: CAT.human, title: 'Notify', state: 'ok',
    body: field('slack #oncall', { mono: true, select: true }),
    ports: [{ kind: 'exec', label: 'send', side: 'in' }] }),
  blockNode({ ...CB.archive, icon: 'output', color: CAT.data, title: 'Archive', state: 'ok',
    body: field('~/inbox/done/', { mono: true }),
    ports: [{ kind: 'exec', label: 'move', side: 'in' }] }),
  blockNode({ ...CB.digest, icon: 'llm', color: CAT.models, title: 'Digest', state: 'idle',
    body: label('context') + field('results.last(15 min)', { mono: true }) + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">summarise what happened</div>`,
    ports: [{ kind: 'exec', label: 'trigger', side: 'in' }, { kind: 'text', label: 'text', side: 'out' }] }),
  blockNode({ ...CB.notify2, icon: 'note', color: CAT.human, title: 'Notify', state: 'idle',
    body: field('email &middot; me', { mono: true, select: true }),
    ports: [{ kind: 'text', label: 'text', side: 'in' }] }),
].join('\n');

const contSvg = [
  wire(CB.webhook.x + CB.webhook.w, PY(CB.webhook, 0), CB.frame.x, frameInY, 'data', { opacity: 0.5 }),
  wire(CB.watch.x + CB.watch.w, PY(CB.watch, 0), CB.frame.x, frameInY, 'file', { live: true, dash: '6 6' }),
  wire(CB.frame.x, frameInY, CB.classify.x, frameInY, 'any', { opacity: 0.7 }),
  cw('classify', 0, 'branch', 0, 'data', { live: true, dash: '6 6' }),
  cw('branch', 0, 'notify', 0, 'exec'),
  cw('branch', 1, 'archive', 0, 'exec'),
  cw('schedule', 0, 'digest', 0, 'exec', { opacity: 0.45, dash: '4 5' }),
  cw('digest', 0, 'notify2', 0, 'text', { opacity: 0.45, dash: '4 5' }),
].join('');

const segmented = (opts, on, col = C.ok) => `<div style="display:flex;gap:3px;padding:3px;background:${C.field};border:1px solid ${C.line};border-radius:7px;">${opts.map(o => `<div style="flex:1;display:flex;align-items:center;justify-content:center;gap:6px;height:24px;border-radius:5px;font-size:11px;font-weight:${o === on ? 600 : 500};color:${o === on ? '#08090b' : C.mid};background:${o === on ? col : 'transparent'};">${o === on ? `<span style="width:5px;height:5px;border-radius:50%;background:#08090b;"></span>` : ''}${o}</div>`).join('')}</div>`;

const srcRow = (ic, name, meta, state, count) => `<div style="display:flex;align-items:center;gap:9px;height:32px;padding:0 8px;border-radius:6px;">
  ${icon(ic, 12, CAT.senses, 1.7)}
  <div style="flex:1;min-width:0;"><div style="font-size:11.5px;color:${C.hi};">${name}</div></div>
  <span style="font-family:${MONO};font-size:9.5px;color:${C.low};">${meta}</span>
  ${statusDot(state)}
</div>`;

const RUNMODE_BODY = [
  sect('Run mode', segmented(['Once', 'Live', 'Schedule'], 'Live')
    + `<div style="margin-top:10px;font-size:10.5px;line-height:1.55;color:${C.low};">Sources keep the graph armed. Each event runs everything downstream of it, and the graph stays up until you stop it.</div>`,
    { tint: C.ok, right: chip('4 h 12 m', C.ok, { dot: true }) }),
  sect('Sources armed', srcRow('http', 'Webhook', '0 today', 'running') + srcRow('folder', 'Watch folder', '3 / min', 'running') + srcRow('clock', 'Schedule', 'next 4:12', 'queued'), { right: chip('3', C.ok) }),
  sect('When events overlap', rowField('Policy', 'Queue &middot; max 50', { select: true }) + switchRow('Coalesce bursts', true, { hint: 'merge events arriving within 500 ms' }) + rowField('Loop concurrency', '2', { mono: true, gap: 0 })),
  sect('Between events', switchRow('Keep block state', true, { hint: 'variables and memory persist across events' }) + switchRow('Restart on crash', true, { hint: 'back off 5 s, 30 s, 2 min' })),
  sect('Recent events', `<div style="font-family:${MONO};font-size:10px;line-height:1.9;">
    ${[['12:41:07', 'folder', 're-arm64-build.eml', C.ok], ['12:41:02', 'folder', 'invoice-0912.eml', C.ok], ['12:40:58', 'folder', 'weekly-digest.eml', C.ok], ['12:30:00', 'clock', 'tick &rarr; digest', C.mid]].map(([t, s, m, c]) => `<div style="display:flex;gap:10px;"><span style="color:${C.faint};">${t}</span><span style="color:${CAT.senses};width:44px;">${s}</span><span style="color:${c};flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${m}</span></div>`).join('')}
  </div>`, { right: chip('1,204', C.mid) }),
].join('');

const CONTINUOUS = doc(shell({
  top: topbar({ name: 'inbox-triage.graph', saved: 'saved', live: '4 h 12 m &middot; 3.1 / min' }),
  library: libraryPanel({ open: ['senses', 'control'], placed: ['Webhook', 'Watch folder', 'Schedule', 'Loop', 'Branch'] }),
  canvas: stage({ svg: contSvg, nodes: contNodes, overlay: zoomPill + minimap([
    [9, 11, 20, 7, CAT.senses], [9, 22, 20, 7, CAT.senses], [9, 40, 20, 7, CAT.senses],
    [33, 10, 51, 24, CAT.control], [88, 13, 19, 6, CAT.human], [88, 24, 19, 6, CAT.data], [57, 40, 21, 7, CAT.models], [88, 38, 19, 6, CAT.human],
  ]) }),
  insp: inspector(RUNMODE_BODY, { title: 'Graph', sub: 'inbox-triage.graph &middot; live', icn: 'mark', col: C.accent, tabs: ['Settings', 'Variables', 'Runs'], tab: 'Settings' }),
  status: statusbar('8 blocks &middot; 8 wires &middot; 1 loop', 'live &middot; 1,204 events &middot; queue 4 &middot; 0 errors'),
}));

/* ============================================================== 9. RunModes */

const transportCard = (chipHtml, name, note) => `<div style="flex:1;background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:14px 16px;">
  <div style="display:flex;align-items:center;height:32px;margin-bottom:10px;">${chipHtml}</div>
  <div style="font-size:12px;font-weight:600;color:${C.hi};margin-bottom:4px;">${name}</div>
  <div style="font-size:10.5px;line-height:1.5;color:${C.low};">${note}</div>
</div>`;

const tChip = (bg, bd, dotCol, textCol, label, extra = '', stop = true) => `<div style="display:flex;align-items:center;gap:8px;height:28px;padding:0 ${stop ? 4 : 12}px 0 10px;border-radius:6px;background:${bg};border:1px solid ${bd};">
  <span style="width:7px;height:7px;border-radius:50%;background:${dotCol};flex:none;"></span>
  <span style="font-family:${MONO};font-size:10.5px;font-weight:600;color:${textCol};letter-spacing:.03em;">${label}</span>
  ${extra ? `<span style="font-family:${MONO};font-size:10.5px;color:${rgba(textCol, 0.7)};">${extra}</span>` : ''}
  ${stop ? `<div style="display:flex;align-items:center;justify-content:center;width:20px;height:20px;border-radius:4px;background:${rgba(C.err, 0.16)};margin-left:2px;">${icon('stop', 11, C.err, 0)}</div>` : ''}
</div>`;

const WATCH_BODY = [
  sect('Source', rowField('Path', '~/inbox', { mono: true, icon: 'folder' }) + rowField('Pattern', '*.eml', { mono: true }) + rowField('On', 'create, modify', { select: true, gap: 0 })),
  sect('Rate', rowField('Debounce', '500 ms', { mono: true }) + rowField('When busy', 'Queue &middot; max 50', { select: true }) + switchRow('Drop duplicates', true, { hint: 'same path within 5 s' })),
  sect('Emits', `<div style="display:flex;align-items:center;gap:8px;">${chip('file', T.file, { dot: true })}<span style="font-size:10.5px;color:${C.low};">one event per matching file</span></div>`),
  sect('Live', rowField('Rate', '3 / min', { mono: true }) + rowField('Total', '1,204 &middot; last 12 s ago', { mono: true, gap: 0 }), { tint: C.ok, right: chip('watching', C.ok, { dot: true }) }),
].join('');

const SCHEDULE_BODY = [
  sect('Interval', segmented(['Every', 'Cron', 'Once at'], 'Every', C.accent) + `<div style="height:11px;"></div>` + rowField('Every', '15 minutes', { select: true }) + rowField('Jitter', '&plusmn; 2 min', { mono: true, gap: 0 })),
  sect('Catch-up', switchRow('Run missed ticks', false, { hint: 'after sleep or a crash' }) + switchRow('Skip if still running', true)),
  sect('Emits', `<div style="display:flex;align-items:center;gap:8px;">${chip('exec', T.exec, { dot: true })}<span style="font-size:10.5px;color:${C.low};">a tick &mdash; wire it to any trigger port</span></div>`),
  sect('Next', rowField('Fires in', '4:12', { mono: true }) + rowField('This hour', '3 ticks &middot; 0 skipped', { mono: true, gap: 0 }), { tint: C.warn, right: chip('armed', C.warn) }),
].join('');

const LOOP_BODY = [
  sect('Iterate', rowField('Over', 'items &middot; any', { select: true }) + rowField('As', 'item', { mono: true }) + rowField('Parallel', '2', { mono: true }) + rowField('Max iterations', '500', { mono: true, gap: 0 })),
  sect('Stop when', field('branch.urgent fires', { select: true, icon: 'branch' }) + switchRow('Continue on error', true, { hint: 'failed items go to the errors port' })),
  sect('Ports', `<div style="display:flex;flex-direction:column;gap:6px;">
    ${[['in', 'items', 'any'], ['out', 'results', 'data'], ['out', 'done', 'exec'], ['out', 'errors', 'data']].map(([d, n, k]) => `<div style="display:flex;align-items:center;gap:9px;height:28px;padding:0 9px;background:${C.field};border:1px solid ${C.line};border-radius:6px;"><span style="font-family:${MONO};font-size:9.5px;color:${C.faint};width:22px;">${d}</span><span style="flex:1;font-family:${MONO};font-size:10.5px;color:${C.hi};">${n}</span>${chip(k, T[k], { dot: true })}</div>`).join('')}
  </div>`),
  sect('Live', rowField('Iteration', '3 of 7 &middot; queue 4', { mono: true, gap: 0 }), { tint: C.ok, right: chip('running', C.ok, { dot: true }) }),
].join('');

const RUNMODES_SHEET = doc(sheet({
  w: 1120, h: 980, kicker: 'Continuous running',
  title: 'The transport says how the graph runs; sources and loops say when',
  body: `<div style="display:flex;gap:16px;">
  ${transportCard(`<div style="display:flex;align-items:center;gap:7px;height:28px;padding:0 12px 0 10px;border-radius:6px;background:${C.accent};">${icon('play', 11, '#08090b', 0)}<span style="font-size:11.5px;font-weight:600;color:#08090b;">Run</span></div>`, 'Once', 'Runs the graph top to bottom, then stops. What every graph does until it has a source.')}
  ${transportCard(tChip(rgba(C.ok, 0.13), rgba(C.ok, 0.35), C.ok, C.ok, 'live', '4h 12m'), 'Live', 'Sources stay armed and every event runs downstream. Stop tears the whole graph down.')}
  ${transportCard(tChip(rgba(C.warn, 0.13), rgba(C.warn, 0.35), C.warn, C.warn, 'next in', '4:12'), 'Scheduled', 'Only Schedule blocks are armed. Between ticks the graph sleeps and costs nothing.')}
  ${transportCard(tChip(rgba(C.mid, 0.13), rgba(C.mid, 0.35), C.mid, C.hi, 'paused', 'queue 12', false), 'Paused', 'Events keep queueing; nothing runs until you resume. Useful while you rewire a live graph.')}
</div>
<div style="display:flex;gap:22px;">
  ${[['Watch folder', 'a source block', WATCH_BODY, { title: 'Watch folder', sub: 'senses &middot; fs.watch', icn: 'folder', col: CAT.senses, tabs: ['Settings', 'Ports', 'Events'], tab: 'Settings' }, CAT.senses],
     ['Schedule', 'periodic trigger', SCHEDULE_BODY, { title: 'Schedule', sub: 'senses &middot; clock.tick', icn: 'clock', col: CAT.senses, tabs: ['Settings', 'Ports', 'Events'], tab: 'Settings' }, CAT.senses],
     ['Loop frame', 'repeat a region of the canvas', LOOP_BODY, { title: 'For each', sub: 'control &middot; loop.frame', icn: 'loop', col: CAT.control, tabs: ['Settings', 'Ports', 'Runs'], tab: 'Settings' }, CAT.control],
    ].map(([cap, note, body, meta, col]) => `<div style="width:328px;flex:none;display:flex;flex-direction:column;gap:10px;">
    <div style="display:flex;align-items:baseline;gap:8px;">
      <span style="width:6px;height:6px;border-radius:50%;background:${col};flex:none;transform:translateY(-1px);"></span>
      <span style="font-size:12px;font-weight:600;color:${C.hi};">${cap}</span>
      <span style="font-size:10.5px;color:${C.low};">${note}</span>
    </div>
    <div style="height:680px;background:${C.panel};border:1px solid ${C.line};border-radius:10px;overflow:hidden;">${panelInner(body, meta)}</div>
  </div>`).join('')}
</div>`,
}));

/* ============================================================= 10. Assistant */

const RAIL_W = 48;
const AW = 1920, AH = 1080;
const ACW = AW - RAIL_W - INSP_W;   // 1544
const ACH = AH - TOP_H - BOT_H;     // 1006

function libraryRail() {
  const cats = [['models', 'llm'], ['capabilities', 'toolbox'], ['runtimes', 'terminal'], ['senses', 'eye'], ['memory', 'db'], ['actuators', 'bolt'], ['data', 'braces'], ['control', 'branch'], ['human', 'approve'], ['custom', 'braces']];
  return `<div style="width:${RAIL_W}px;flex:none;display:flex;flex-direction:column;align-items:center;gap:6px;padding:10px 0;background:${C.panel};border-right:1px solid ${C.line};">
  <div style="display:flex;align-items:center;justify-content:center;width:30px;height:30px;border-radius:7px;background:${C.field};border:1px solid ${C.line};margin-bottom:6px;">${icon('search', 13, C.low)}</div>
  ${cats.map(([id, ic]) => `<div style="display:flex;align-items:center;justify-content:center;width:30px;height:30px;border-radius:7px;background:${rgba(CAT[id], 0.1)};">${icon(ic, 14, CAT[id], 1.7)}</div>`).join('')}
  <span style="flex:1;"></span>
  <div style="display:flex;align-items:center;justify-content:center;width:30px;height:30px;">${icon('chev', 13, C.low, 2)}</div>
</div>`;
}

const meter = (heights, col) => `<div style="display:flex;align-items:flex-end;gap:2px;height:22px;">${heights.map(h => `<span style="width:5px;height:${h}px;border-radius:1px;background:${col};opacity:${0.45 + h / 40};"></span>`).join('')}</div>`;

const camPreview = `<div style="position:relative;height:46px;border-radius:5px;background:linear-gradient(135deg,#1c2027,#0e1116);border:1px solid ${C.soft};overflow:hidden;">
  <div style="position:absolute;left:52px;top:9px;width:26px;height:30px;border:1px solid ${T.image};border-radius:2px;"></div>
  <div style="position:absolute;left:52px;top:2px;font-family:${MONO};font-size:7px;color:${T.image};">person .97</div>
  <div style="position:absolute;left:108px;top:14px;width:34px;height:26px;border:1px solid ${rgba(T.image, 0.6)};border-radius:2px;"></div>
  <div style="position:absolute;left:108px;top:7px;font-family:${MONO};font-size:7px;color:${rgba(T.image, 0.8)};">door .88</div>
</div>`;

const fnChip = (t) => chip(t, T.tools);


// Stage view: a slim header, port dots only on the edges at the same y as
// every other view (switching views never moves a port), content fills the rest.
function stageBlock(o) {
  const c = o.color || T.any;
  const selected = !!o.selected;
  const borderCol = selected ? C.accent : o.state === 'running' ? rgba(C.ok, 0.5) : C.line;
  const shadow = selected ? `0 0 0 1px ${C.accent},0 0 0 5px ${rgba(C.accent, 0.15)},0 16px 38px rgba(0,0,0,.6)` : `0 12px 30px rgba(0,0,0,.5)`;
  const ins = (o.ports || []).filter(p => p.side === 'in'), outs = (o.ports || []).filter(p => p.side === 'out');
  const dot = (p, i, side) => `<span title="${p.label}" style="position:absolute;${side === 'in' ? 'left:-5.5px;' : 'right:-5.5px;'}top:${51 + 24 * i - 5.5}px;width:11px;height:11px;border-radius:50%;background:${T[p.kind]};box-shadow:0 0 0 3px ${rgba(T[p.kind], 0.12)};opacity:${p.dim ? 0.3 : 1};"></span>`;
  return `<div style="position:absolute;left:${o.x}px;top:${o.y}px;width:${o.w}px;background:${C.block};border:1px solid ${borderCol};border-radius:9px;box-shadow:${shadow};">
  <div style="display:flex;align-items:center;gap:7px;height:24px;padding:0 8px;border-bottom:1px solid ${C.soft};border-radius:8px 8px 0 0;background:linear-gradient(180deg,${rgba(c, 0.13)},${rgba(c, 0.02)});">
    ${icon(o.icon, 12, c, 1.7)}
    <span style="font-size:11px;font-weight:600;color:${C.hi};white-space:nowrap;">${o.title}</span>
    <span style="flex:1;"></span>
    ${o.badge || ''}
    ${viewToggle('stage', 'stage')}
    ${statusDot(o.state || 'idle')}
  </div>
  <div style="position:relative;height:${o.h}px;overflow:hidden;border-radius:0 0 8px 8px;background:${o.bg || C.field};">${o.content}${grip}</div>
  ${ins.map((p, i) => dot(p, i, 'in')).join('')}${outs.map((p, i) => dot(p, i, 'out')).join('')}
</div>`;
}


/* --------------------------------------------------------------- rigs */
// Four base aesthetics, one expression vocabulary. Each returns a 64-unit SVG.
const RIG_EXPR = ['neutral', 'smile', 'frown', 'surprised', 'thinking', 'speaking', 'love'];
const heart = (cx, cy, r, fill) => `<path d="M${cx} ${cy + r * 0.95} C${cx - r * 1.5} ${cy - r * 0.05}, ${cx - r * 1.05} ${cy - r * 1.25}, ${cx} ${cy - r * 0.45} C${cx + r * 1.05} ${cy - r * 1.25}, ${cx + r * 1.5} ${cy - r * 0.05}, ${cx} ${cy + r * 0.95}Z" fill="${fill}"/>`;
const ROSE = CAT.human;
function rigFace(rig, expr, size = 64) {
  const W = C.hi, cy = C.accent, am = T.tools, vi = T.data;
  let g = '';
  if (rig === 'line') {
    const eyes = {
      neutral: `<circle cx="22" cy="26" r="2.6" fill="${W}"/><circle cx="42" cy="26" r="2.6" fill="${W}"/>`,
      smile: `<path d="M17 27q5-6 10 0" /><path d="M37 27q5-6 10 0"/>`,
      frown: `<circle cx="22" cy="27" r="2.6" fill="${W}"/><circle cx="42" cy="27" r="2.6" fill="${W}"/><path d="M16 19l9 3"/><path d="M48 19l-9 3"/>`,
      surprised: `<circle cx="22" cy="25" r="4.2"/><circle cx="42" cy="25" r="4.2"/>`,
      thinking: `<circle cx="22" cy="26" r="2.6" fill="${W}"/><circle cx="43" cy="22" r="2.6" fill="${W}"/><path d="M37 16q5-3 10 0"/>`,
      speaking: `<circle cx="22" cy="26" r="2.6" fill="${W}"/><circle cx="42" cy="26" r="2.6" fill="${W}"/>`,
      love: heart(22, 26, 5, ROSE) + heart(42, 26, 5, ROSE),
    }[expr];
    const mouth = {
      neutral: `<path d="M25 43h14"/>`,
      smile: `<path d="M21 40q11 11 22 0"/>`,
      frown: `<path d="M21 46q11-9 22 0"/>`,
      surprised: `<ellipse cx="32" cy="44" rx="4.5" ry="6.5"/>`,
      thinking: `<path d="M25 44q5-4 10 0"/><circle cx="43" cy="45" r="1.4" fill="${W}"/><circle cx="48" cy="45" r="1.4" fill="${W}"/>`,
      speaking: `<ellipse cx="32" cy="43" rx="7" ry="4"/>`,
      love: `<path d="M21 40q11 11 22 0"/>`,
    }[expr];
    g = `<g fill="none" stroke="${W}" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round">${eyes}${mouth}</g>`;
  } else if (rig === 'robot') {
    const eye = (x, h, skew = 0) => `<rect x="${x}" y="${28 - h / 2}" width="11" height="${h}" rx="2" fill="${cy}" transform="skewY(${skew}) translate(0 ${-skew * 0.2})"/>`;
    const eyes = {
      neutral: eye(17, 6) + eye(36, 6),
      smile: `<path d="M17 30q5.5-7 11 0" fill="none" stroke="${cy}" stroke-width="3.5" stroke-linecap="round"/><path d="M36 30q5.5-7 11 0" fill="none" stroke="${cy}" stroke-width="3.5" stroke-linecap="round"/>`,
      frown: `<rect x="17" y="24" width="11" height="6" rx="2" fill="${cy}" transform="rotate(12 22.5 27)"/><rect x="36" y="24" width="11" height="6" rx="2" fill="${cy}" transform="rotate(-12 41.5 27)"/>`,
      surprised: eye(17, 11) + eye(36, 11),
      thinking: eye(17, 6) + `<rect x="36" y="22" width="11" height="4" rx="2" fill="${cy}"/>`,
      speaking: eye(17, 6) + eye(36, 6),
      love: heart(22.5, 28, 5.5, ROSE) + heart(41.5, 28, 5.5, ROSE),
    }[expr];
    const bars = {
      neutral: [[0, 3], [0, 3], [0, 3], [0, 3], [0, 3]],
      smile: [[2, 3], [0.5, 3], [-1.5, 3], [0.5, 3], [2, 3]],
      frown: [[-2.5, 3], [-0.5, 3], [1.5, 3], [-0.5, 3], [-2.5, 3]],
      surprised: null,
      thinking: [[0, 3], [0, 3], [0, 3], null, null],
      speaking: [[0, 3], [0, 7], [0, 5], [0, 8], [0, 4]],
      love: [[2, 3], [0.5, 3], [-1.5, 3], [0.5, 3], [2, 3]],
    }[expr];
    const mouth = bars === null
      ? `<rect x="28" y="38" width="8" height="8" rx="2" fill="${cy}"/>`
      : bars.map((b, i) => b ? `<rect x="${20 + i * 5.2}" y="${42 - b[0] - b[1] / 2}" width="3.6" height="${b[1]}" rx="1" fill="${cy}"/>` : '').join('');
    g = `<rect x="10" y="12" width="44" height="42" rx="9" fill="${C.block}" stroke="${C.mid}" stroke-width="2"/><path d="M32 12V6" stroke="${C.mid}" stroke-width="2" stroke-linecap="round"/><circle cx="32" cy="5" r="2" fill="${C.mid}"/>${eyes}${mouth}`;
  } else if (rig === 'orb') {
    const col = { neutral: cy, smile: am, frown: '#4e6392', surprised: W, thinking: vi, speaking: cy, love: ROSE }[expr];
    const r = expr === 'surprised' ? 23 : 20;
    const ink = '#0b0d11';
    const eyes = {
      neutral: `<circle cx="26" cy="30" r="2.2" fill="${ink}"/><circle cx="38" cy="30" r="2.2" fill="${ink}"/>`,
      smile: `<path d="M22 31q4-5 8 0M34 31q4-5 8 0" fill="none" stroke="${ink}" stroke-width="2.4" stroke-linecap="round"/>`,
      frown: `<path d="M22 28l7 3M42 28l-7 3" fill="none" stroke="${W}" stroke-width="2.2" stroke-linecap="round"/>`,
      surprised: `<circle cx="25" cy="29" r="3.4" fill="${ink}"/><circle cx="39" cy="29" r="3.4" fill="${ink}"/>`,
      thinking: `<circle cx="26" cy="31" r="2.2" fill="${ink}"/><circle cx="39" cy="27" r="2.2" fill="${ink}"/><circle cx="50" cy="16" r="2.4" fill="${vi}"/>`,
      speaking: `<circle cx="26" cy="30" r="2.2" fill="${ink}"/><circle cx="38" cy="30" r="2.2" fill="${ink}"/><circle cx="32" cy="32" r="26" fill="none" stroke="${cy}" stroke-width="1.2" opacity=".55"/><circle cx="32" cy="32" r="30" fill="none" stroke="${cy}" stroke-width="1" opacity=".25"/>`,
      love: heart(26, 30, 3.2, ink) + heart(38, 30, 3.2, ink) + heart(51, 13, 3.4, ROSE),
    }[expr];
    g = `<circle cx="32" cy="32" r="${r + 6}" fill="${col}" opacity=".16"/><circle cx="32" cy="32" r="${r}" fill="${col}"/><circle cx="26" cy="24" r="6" fill="#ffffff" opacity=".16"/>${eyes}`;
  } else if (rig === 'pixel') {
    const led = T.tools;
    const eyesN = [[2, 2], [5, 2]];
    const cells = {
      neutral: [...eyesN, [2, 5], [3, 5], [4, 5], [5, 5]],
      smile: [...eyesN, [1, 5], [2, 6], [3, 6], [4, 6], [5, 6], [6, 5]],
      frown: [...eyesN, [1, 6], [2, 5], [3, 5], [4, 5], [5, 5], [6, 6]],
      surprised: [[2, 1], [2, 2], [5, 1], [5, 2], [3, 5], [4, 5], [3, 6], [4, 6]],
      thinking: [[2, 2], [5, 1], [2, 5], [3, 5], [4, 5], [7, 3]],
      speaking: [...eyesN, [2, 5], [3, 5], [4, 5], [5, 5], [3, 6], [4, 6]],
      love: [[1, 1], [2, 1], [5, 1], [6, 1], [0, 2], [1, 2], [2, 2], [3, 2], [4, 2], [5, 2], [6, 2], [7, 2], [0, 3], [1, 3], [2, 3], [3, 3], [4, 3], [5, 3], [6, 3], [7, 3], [1, 4], [2, 4], [3, 4], [4, 4], [5, 4], [6, 4], [2, 5], [3, 5], [4, 5], [5, 5], [3, 6], [4, 6]],
    }[expr];
    let grid = '';
    for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) grid += `<rect x="${8 + x * 6}" y="${8 + y * 6}" width="5" height="5" rx="1" fill="${C.soft}"/>`;
    const lit = expr === 'love' ? ROSE : led;
    g = `<rect x="4" y="4" width="56" height="56" rx="4" fill="#07080a"/>${grid}${cells.map(([x, y]) => `<rect x="${8 + x * 6}" y="${8 + y * 6}" width="5" height="5" rx="1" fill="${lit}"/>`).join('')}`;
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 64 64" style="display:block;flex:none;">${g}</svg>`;
}

function assistant(stage) {
const XB = {
  webcam: { x: 24, y: 50, w: 180 },
  mic: { x: 24, y: 250, w: 180 },
  keyboard: { x: 24, y: 394, w: 180 },
  objdet: { x: 260, y: 40, w: 200 },
  facerec: { x: 260, y: 182, w: 200 },
  stt: { x: 260, y: 304, w: 200 },
  motors: { x: 260, y: 458, w: 200 },
  wm: { x: 260, y: 656, w: 200 },
  ltm: { x: 260, y: 789, w: 200 },
  toolbox: { x: 520, y: 610, w: 200 },
  hub: { x: 520, y: 806, w: 210 },
  llm: { x: 780, y: 300, w: 260 },
  affect: { x: 1100, y: 30, w: 200 },
  avatar: { x: 1340, y: 40, w: 200 },
  display: { x: 1100, y: 200, w: 200 },
  tts: { x: 1100, y: 360, w: 200 },
  term: { x: 1100, y: 490, w: 200 },
  speaker: { x: 1340, y: 360, w: 180 },
};
if (stage) Object.assign(XB, { affect: { x: 1100, y: 30, w: 180 }, avatar: { x: 1300, y: 40, w: 240 }, display: { x: 1100, y: 200, w: 180 }, tts: { x: 1100, y: 360, w: 180 }, term: { x: 1100, y: 490, w: 180 }, speaker: { x: 1330, y: 420, w: 180 } });
const xw = W(XB);
const hubRow = (ic, t, col, extra = '') => `<div style="display:flex;align-items:center;gap:7px;height:21px;padding:0 7px;border-radius:5px;background:${C.field};border:1px solid ${C.soft};">${icon(ic, 11, col, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${C.mid};white-space:nowrap;">${t}</span>${extra}</div>`;

const asstNodes = [
  blockNode({ ...XB.webcam, icon: 'eye', color: CAT.senses, title: 'Webcam', state: 'running',
    body: camPreview + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">1280&times;720 &middot; 15 fps</div>`,
    ports: [{ kind: 'image', label: 'frames', side: 'out' }] }),
  blockNode({ ...XB.mic, icon: 'note', color: CAT.senses, title: 'Microphone', state: 'running',
    body: meter([6, 12, 18, 22, 16, 20, 10, 14, 19, 8, 12, 17, 9, 5], T.audio) + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">&minus;18 dB &middot; vad: speech</div>`,
    ports: [{ kind: 'audio', label: 'audio', side: 'out' }] }),
  blockNode({ ...XB.keyboard, icon: 'form', color: CAT.senses, title: 'Keyboard', state: 'ok',
    body: field('&gt; check the front door_', { mono: true }),
    ports: [{ kind: 'text', label: 'text', side: 'out' }] }),
  blockNode({ ...XB.objdet, icon: 'eye', color: CAT.models, title: 'Object detection', state: 'running',
    body: `<div style="display:flex;gap:4px;">${chip('person .97', T.image)}${chip('door .88', T.image)}</div><div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">yolo-v8n &middot; 22 ms &middot; 5 fps</div>`,
    ports: [{ kind: 'image', label: 'image', side: 'in' }, { kind: 'data', label: 'objects', side: 'out' }] }),
  blockNode({ ...XB.facerec, icon: 'approve', color: CAT.models, title: 'Face recognition', state: 'ok',
    body: `<div style="display:flex;align-items:center;gap:8px;"><span style="width:22px;height:22px;border-radius:50%;background:${rgba(CAT.human, 0.25)};border:1px solid ${rgba(CAT.human, 0.5)};display:flex;align-items:center;justify-content:center;font-size:10px;font-weight:600;color:${CAT.human};flex:none;">M</span><div><div style="font-size:11px;color:${C.hi};">Mykl &middot; 0.93</div><div style="font-family:${MONO};font-size:9px;color:${C.faint};">known &middot; seen 2 min ago</div></div></div>`,
    ports: [{ kind: 'image', label: 'image', side: 'in' }, { kind: 'data', label: 'person', side: 'out' }] }),
  blockNode({ ...XB.stt, icon: 'note', color: CAT.models, title: 'Speech to text', state: 'running',
    body: `<div style="font-family:${MONO};font-size:9.5px;line-height:1.55;color:#c3cad4;white-space:nowrap;overflow:hidden;">&#8230;check the front door<span style="color:${C.ok};">&#9612;</span></div>`,
    ports: [{ kind: 'audio', label: 'audio', side: 'in' }, { kind: 'text', label: 'text', side: 'out' }] }),
  blockNode({ ...XB.motors, icon: 'loop', color: CAT.actuators, title: 'Motors', state: 'ok',
    badge: chip('warns', C.warn, { dot: true }),
    body: field('2 servos &middot; pan / tilt', { mono: true }) + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">pan &minus;40&deg; &middot; tilt 5&deg; &middot; load 12%</div>`,
    ports: [
      { kind: 'tools', label: 'tool', side: 'out' },
      { kind: 'stream', label: 'state', side: 'out' },
      { kind: 'exec', label: 'fault', side: 'out' },
    ] }),
  blockNode({ ...XB.wm, icon: 'braces', color: CAT.memory, title: 'Working memory', state: 'ok',
    body: `<div style="font-family:${MONO};font-size:9.5px;line-height:1.7;color:${C.faint};white-space:nowrap;"><span style="color:${C.mid};">14:02</span> Mykl at desk<br><span style="color:${C.mid};">14:03</span> asked about the door</div>`,
    ports: [{ kind: 'memory', label: 'memory', side: 'out' }] }),
  blockNode({ ...XB.ltm, icon: 'db', color: CAT.memory, title: 'Long-term memory', state: 'ok',
    body: `<div style="font-family:${MONO};font-size:9.5px;line-height:1.6;color:${C.faint};">Mykl &mdash; terse answers, works late</div><div style="margin-top:4px;font-family:${MONO};font-size:9.5px;color:${C.faint};">12 people &middot; 2,140 episodes</div>`,
    ports: [{ kind: 'memory', label: 'memory', side: 'out' }] }),
  blockNode({ ...XB.toolbox, icon: 'toolbox', color: CAT.capabilities, title: 'Toolbox', state: 'ok',
    badge: chip('2 fns', T.tools),
    body: `<div style="display:flex;flex-direction:column;gap:5px;">${hubRow('loop', 'motor.move', CAT.actuators, `<span style="flex:1;"></span><span style="width:6px;height:6px;border-radius:50%;background:${C.warn};"></span>`)}${hubRow('loop', 'motor.home', CAT.actuators)}<div style="font-family:${MONO};font-size:9px;color:${C.faint};padding-left:2px;">pauses on fault &middot; resume anytime</div></div>`,
    ports: [{ kind: 'tools', label: 'motors', side: 'in' }, { kind: 'exec', label: 'pause', side: 'in' }, { kind: 'tools', label: 'tools', side: 'out' }] }),
  blockNode({ ...XB.hub, icon: 'merge', color: CAT.memory, title: 'Memory hub', state: 'ok',
    badge: chip('2', T.memory),
    body: `<div style="display:flex;flex-direction:column;gap:5px;">${hubRow('braces', 'working &middot; fast', CAT.memory)}${hubRow('db', 'long-term &middot; vectors', CAT.memory)}<div style="font-family:${MONO};font-size:9px;color:${C.faint};padding-left:2px;">consolidate every 10 min</div></div>`,
    ports: [{ kind: 'memory', label: 'working', side: 'in' }, { kind: 'memory', label: 'long-term', side: 'in' }, { kind: 'memory', label: 'memory', side: 'out' }] }),
  blockNode({ ...XB.llm, icon: 'llm', color: CAT.models, title: 'Orchestrator', state: 'running', selected: !stage,
    badge: chip('thinking', C.ok, { dot: true }),
    body: label('model') + field('qwen2.5:14b &middot; local', { mono: true, select: true })
      + `<div style="margin-top:9px;font-family:${MONO};font-size:9.5px;line-height:1.55;color:${C.faint};">Mykl asked about the front door. Camera shows it closed, no one near it for 2 min. Pan the camera to confirm&#8230;</div>`
      + `<div style="margin-top:8px;display:flex;align-items:center;gap:7px;height:23px;padding:0 8px;border-radius:5px;background:${rgba(T.tools, 0.1)};border:1px solid ${rgba(T.tools, 0.32)};">${icon('loop', 11, T.tools, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${T.tools};white-space:nowrap;">motor.move(pan: &minus;40)</span><span style="flex:1;"></span>${chip('warned', C.warn)}</div>`,
    ports: [
      { kind: 'text', label: 'prompt', side: 'in' },
      { kind: 'data', label: 'context', side: 'in' },
      { kind: 'tools', label: 'tools', side: 'in' },
      { kind: 'memory', label: 'memory', side: 'in' },
      { kind: 'text', label: 'text', side: 'out' },
      { kind: 'text', label: 'thoughts', side: 'out' },
      { kind: 'data', label: 'calls', side: 'out' },
    ] }),
  blockNode({ ...XB.affect, icon: 'face', color: CAT.models, title: 'Affect', state: 'running',
    body: `<div style="display:flex;gap:4px;">${chip('calm .72', CAT.models)}${chip('curious .21', C.mid)}</div><div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">affect-small &middot; 4 ms</div>`,
    ports: [{ kind: 'text', label: 'text', side: 'in' }, { kind: 'data', label: 'affect', side: 'out' }] }),
  stage
    ? stageBlock({ ...XB.avatar, h: 240, icon: 'face', color: CAT.actuators, title: 'Avatar', state: 'running', selected: true,
        content: `<div style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center;">${rigFace('line', 'smile', 200)}</div><div style="position:absolute;left:9px;bottom:7px;font-family:${MONO};font-size:9px;color:${C.faint};">smile &middot; speaking &middot; 240 &times; 240</div>`,
        ports: [
          { kind: 'audio', label: 'speech', side: 'in' },
          { kind: 'data', label: 'express', side: 'in' },
          { kind: 'data', label: 'look', side: 'in' },
          { kind: 'tools', label: 'tool', side: 'out', dim: true },
          { kind: 'stream', label: 'state', side: 'out' },
        ] })
    : blockNode({ ...XB.avatar, icon: 'face', color: CAT.actuators, title: 'Avatar', state: 'running',
    badge: chip('line', CAT.actuators),
    body: `<div style="display:flex;align-items:center;gap:10px;"><div style="width:46px;height:46px;flex:none;border-radius:6px;background:${C.field};border:1px solid ${C.soft};display:flex;align-items:center;justify-content:center;">${rigFace('line', 'smile', 40)}</div><div style="font-family:${MONO};font-size:9.5px;line-height:1.6;color:${C.faint};">smile &middot; speaking<br>looking at Mykl</div></div>`,
    ports: [
      { kind: 'audio', label: 'speech', side: 'in' },
      { kind: 'data', label: 'express', side: 'in' },
      { kind: 'data', label: 'look', side: 'in' },
      { kind: 'tools', label: 'tool', side: 'out', dim: true },
      { kind: 'stream', label: 'state', side: 'out' },
    ] }),
  blockNode({ ...XB.display, icon: 'form', color: CAT.actuators, title: 'Display', state: 'ok',
    body: `<div style="padding:7px 9px;border-radius:5px;background:${C.field};border:1px solid ${C.soft};font-size:10.5px;line-height:1.5;color:#c3cad4;">Front door is closed. Last motion 14:01.</div><div style="margin-top:6px;font-family:${MONO};font-size:9.5px;color:${C.faint};">HDMI-1 &middot; overlay</div>`,
    ports: [{ kind: 'text', label: 'text', side: 'in' }] }),
  blockNode({ ...XB.tts, icon: 'note', color: CAT.models, title: 'Text to speech', state: 'idle',
    body: field('piper &middot; en_GB-alan', { mono: true, select: true }),
    ports: [{ kind: 'text', label: 'text', side: 'in' }, { kind: 'audio', label: 'audio', side: 'out' }] }),
  blockNode({ ...XB.term, icon: 'terminal', color: CAT.runtimes, title: 'Terminal', state: 'running',
    badge: chip('thoughts', C.mid),
    body: `<div style="padding:6px 8px;border-radius:5px;background:#07080a;border:1px solid ${C.soft};font-family:${MONO};font-size:9px;line-height:1.65;color:#9aa4b0;white-space:nowrap;overflow:hidden;">[14:03:12] look: door closed<br>[14:03:12] recall: asked yesterday<br>[14:03:13] act: pan camera &minus;40&deg;<span style="color:${C.ok};">&#9612;</span></div>`,
    ports: [{ kind: 'text', label: 'text', side: 'in' }] }),
  blockNode({ ...XB.speaker, icon: 'note', color: CAT.actuators, title: 'Speaker', state: 'idle',
    body: field('default &middot; 48 kHz', { mono: true, muted: true }),
    ports: [{ kind: 'audio', label: 'audio', side: 'in' }] }),
].join('\n');

const asstSvg = [
  xw('webcam', 0, 'objdet', 0, 'image', { live: true, dash: '6 6' }),
  xw('webcam', 0, 'facerec', 0, 'image', { live: true, dash: '6 6' }),
  xw('mic', 0, 'stt', 0, 'audio', { live: true, dash: '6 6' }),
  xw('keyboard', 0, 'llm', 0, 'text'),
  xw('stt', 0, 'llm', 0, 'text', { live: true, dash: '6 6' }),
  xw('objdet', 0, 'llm', 1, 'data', { live: true, dash: '6 6' }),
  xw('facerec', 0, 'llm', 1, 'data'),
  xw('motors', 0, 'toolbox', 0, 'tools'),
  xw('motors', 1, 'llm', 1, 'stream', { live: true, dash: '6 6' }),
  xw('motors', 2, 'toolbox', 1, 'exec', { opacity: 0.5, dash: '4 5' }),
  xw('toolbox', 0, 'llm', 2, 'tools', { width: 2.2 }),
  xw('wm', 0, 'hub', 0, 'memory'),
  xw('ltm', 0, 'hub', 1, 'memory'),
  xw('hub', 0, 'llm', 3, 'memory', { width: 2.2 }),
  xw('llm', 0, 'display', 0, 'text', { live: true, dash: '6 6' }),
  xw('llm', 0, 'tts', 0, 'text', { opacity: 0.5 }),
  xw('tts', 0, 'speaker', 0, 'audio', { opacity: 0.5 }),
  xw('llm', 1, 'term', 0, 'text', { live: true, dash: '6 6' }),
  xw('llm', 0, 'affect', 0, 'text', { live: true, dash: '6 6' }),
  xw('affect', 0, 'avatar', 1, 'data', { live: true, dash: '6 6' }),
  xw('tts', 0, 'avatar', 0, 'audio', { opacity: 0.5 }),
  xw('facerec', 0, 'avatar', 2, 'data'),
].join('');

const ORCH_BODY = [
  sect('Model', rowField('Provider', 'Ollama &middot; local &middot; cuda', { select: true, icon: 'llm' }) + rowField('Model', 'qwen2.5:14b', { select: true }) + rowField('Role', 'Orchestrator', { select: true, gap: 0 })
    + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Coordinates the specialists below. It never sees raw frames or audio &mdash; only what they report.</div>`),
  sect('Specialists', connRow('eye', 'Object detection', 'yolo-v8n &rarr; context', 'data', 'running') + connRow('approve', 'Face recognition', 'insightface &rarr; context', 'data', 'ok') + connRow('note', 'Speech to text', 'whisper-small &rarr; prompt', 'text', 'running') + connRow('face', 'Affect', 'text &rarr; avatar.express', 'data', 'running'), { tint: CAT.models, right: chip('4', CAT.models) }),
  sect('Memory', connRow('merge', 'Memory hub', 'working + long-term', 'memory', 'ok') + switchRow('May write memories', true, { hint: 'remember() and forget() offered as tools', col: T.memory }), { tint: T.memory, right: chip('hub', T.memory) }),
  sect('Tools', connRow('toolbox', 'Toolbox', 'motor.move, motor.home &middot; pauses on fault', 'tools', 'ok') + switchRow('Warn on physical actions', true, { hint: 'a warning, never a block &mdash; you own your tools', col: C.warn }), { tint: T.tools, right: chip('3', T.tools) }),
  sect('Thoughts', switchRow('Print thoughts', true, { hint: '&rarr; Terminal, one line per step' }) + rowField('Detail', 'look / recall / act', { select: true, gap: 0 })),
].join('');

return doc(`<div style="width:${AW}px;height:${AH}px;display:flex;flex-direction:column;background:${C.ground};overflow:hidden;font-family:${SANS};">
  ${topbar({ name: 'home-assistant.graph', saved: 'saved', live: '4 h 12 m', runtime: 'local &middot; ollama + cuda' })}
  <div style="flex:1;display:flex;min-height:0;">
    ${libraryRail()}
    <div style="flex:1;position:relative;overflow:hidden;background-color:${C.canvas};background-image:radial-gradient(circle at 1px 1px, rgba(255,255,255,.055) 1px, transparent 0);background-size:22px 22px;">
      <svg xmlns="http://www.w3.org/2000/svg" width="${ACW}" height="${ACH}" viewBox="0 0 ${ACW} ${ACH}" style="position:absolute;left:0;top:0;pointer-events:none;">${asstSvg}</svg>
      ${asstNodes}
      <div style="position:absolute;left:24px;top:${ACH - 136}px;display:flex;flex-direction:column;gap:5px;font-family:${MONO};font-size:9.5px;color:${C.faint};">
        <div style="display:flex;align-items:center;gap:8px;"><span style="width:18px;height:2px;background:${T.image};"></span>image</div>
        <div style="display:flex;align-items:center;gap:8px;"><span style="width:18px;height:2px;background:${T.audio};"></span>audio</div>
        <div style="display:flex;align-items:center;gap:8px;"><span style="width:18px;height:2px;background:${T.memory};"></span>memory</div>
        <div style="display:flex;align-items:center;gap:8px;"><span style="width:18px;height:2px;background:${T.tools};"></span>tools &middot; two-way</div>
        <div style="display:flex;align-items:center;gap:8px;"><span style="width:18px;height:2px;background:${T.exec};"></span>exec &middot; fault</div>
      </div>
      ${minimap([[8, 9, 12, 8, CAT.senses], [8, 22, 12, 7, CAT.senses], [8, 32, 12, 6, CAT.senses], [24, 8, 13, 8, CAT.models], [24, 18, 13, 8, CAT.models], [24, 27, 13, 7, CAT.models], [24, 37, 13, 12, CAT.actuators], [24, 51, 13, 8, CAT.memory], [24, 60, 13, 9, CAT.memory], [41, 46, 13, 11, CAT.capabilities], [41, 59, 13, 11, CAT.memory], [59, 26, 17, 13, CAT.models], [80, 8, 13, 6, CAT.models], [96, 9, 13, 10, CAT.actuators], [80, 19, 13, 7, CAT.actuators], [80, 28, 13, 6, CAT.models], [80, 37, 13, 7, CAT.runtimes], [96, 28, 12, 6, CAT.actuators]])}
    </div>
    ${stage ? inspector(avatarBody(true), { title: 'Avatar', sub: 'actuators &middot; avatar.rig &middot; stage view', icn: 'face', col: CAT.actuators, tabs: ['Settings', 'Ports', 'Runs', 'Rigs'], tab: 'Settings' }) : inspector(ORCH_BODY, { title: 'Orchestrator', sub: 'models &middot; llm.chat &middot; role: orchestrator', icn: 'llm', col: CAT.models, tabs: ['Settings', 'Ports', 'Runs'], tab: 'Settings' })}
  </div>
  ${statusbar('18 blocks &middot; 22 wires &middot; 4 specialists', 'live &middot; 4 h 12 m &middot; cam 15 fps &middot; mic vad &middot; 1 warning')}
</div>`);
}

/* =========================================================== 11. SensePanels */

const personRow = (initial, name, meta, col, action = '') => `<div style="display:flex;align-items:center;gap:9px;padding:7px 9px;background:${C.field};border:1px solid ${C.line};border-radius:6px;margin-bottom:6px;">
  <span style="width:22px;height:22px;border-radius:50%;background:${rgba(col, 0.22)};border:1px solid ${rgba(col, 0.5)};display:flex;align-items:center;justify-content:center;font-size:10px;font-weight:600;color:${col};flex:none;">${initial}</span>
  <div style="flex:1;min-width:0;"><div style="font-size:11.5px;color:${C.hi};">${name}</div><div style="font-family:${MONO};font-size:9.5px;color:${C.low};margin-top:1px;">${meta}</div></div>
  ${action}
</div>`;

const WEBCAM_BODY = [
  sect('Device', rowField('Camera', 'Logitech C920 &middot; /dev/video0', { select: true, icon: 'eye' }) + rowField('Resolution', '1280 &times; 720', { select: true }) + rowField('Frame rate', '15 fps', { mono: true, gap: 0 })
    + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Downstream blocks sample what they need; detection runs at 5 fps.</div>`),
  sect('Emits', `<div style="display:flex;align-items:center;gap:8px;">${chip('image', T.image, { dot: true })}<span style="font-size:10.5px;color:${C.low};">every frame &middot; 2 subscribers</span></div>`),
  sect('View', segmented(['Compact', 'Summary', 'Stage'], 'Summary', C.accent) + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Stage fills the block with the live frame.</div>`),
  sect('Privacy', switchRow('Frames never leave this machine', true, { col: C.err }) + switchRow('Record to disk', false) + rowField('Retention', 'none &middot; frames are dropped after use', { muted: true, gap: 0 }), { tint: C.err, right: chip('local only', C.err, { dot: true }) }),
  sect('Live', camPreview + `<div style="margin-top:8px;font-family:${MONO};font-size:10px;color:${C.mid};">15 fps &middot; 22 ms &middot; 2 subscribers</div>`, { tint: C.ok, right: chip('capturing', C.ok, { dot: true }) }),
].join('');

const FACE_BODY = [
  sect('Model', rowField('Model', 'insightface &middot; buffalo_l', { select: true }) + slider('Match threshold', '0.80', 80, CAT.models) + rowField('Runs on', 'GPU &middot; 9 ms', { mono: true, gap: 0 })),
  sect('Known people', personRow('M', 'Mykl', 'you &middot; 412 sightings', CAT.human) + personRow('S', 'Sam', 'partner &middot; 96 sightings', CAT.human) + personRow('?', 'Unknown-3', 'seen 4&times; this week', C.mid, chip('name', C.accent)), { right: chip('12', CAT.human) }),
  sect('Emits', `<div style="display:flex;align-items:center;gap:8px;margin-bottom:9px;">${chip('data', T.data, { dot: true })}<span style="font-family:${MONO};font-size:10px;color:${C.low};">{ name, confidence, bbox }</span></div>` + switchRow('Enrol new faces automatically', false, { hint: 'ask before anyone new is stored' })),
  sect('Stored as', `<div style="font-size:10.5px;line-height:1.55;color:${C.low};">Embeddings only &mdash; 512 floats per person, never an image. Delete a person and every sighting goes with them.</div>`, { tint: C.err }),
].join('');

const HUB_BODY = [
  sect('Stores', connRow('braces', 'Working memory', 'in-process &middot; 128 items &middot; 5 min', 'memory', 'ok') + connRow('db', 'Long-term memory', 'sqlite + vectors &middot; 2,140 episodes', 'memory', 'ok'), { right: chip('2', T.memory), tint: T.memory }),
  sect('Recall', rowField('Order', 'working first, then long-term', { select: true }) + rowField('Max recalled', '12 items', { mono: true }) + slider('Relevance cutoff', '0.62', 62, T.memory)),
  sect('Consolidation', rowField('Every', '10 min &middot; or when working is full', { select: true }) + switchRow('Summarise before storing', true, { hint: 'the orchestrator writes one line per episode' }) + switchRow('Forget after', false, { hint: 'off &mdash; episodes are kept until deleted' })),
  sect('What is kept', `<div style="display:flex;flex-direction:column;gap:5px;font-family:${MONO};font-size:10px;">
    <div style="display:flex;gap:10px;"><span style="color:${C.ok};width:52px;">yes</span><span style="color:${C.mid};">transcripts, who was seen, where, when</span></div>
    <div style="display:flex;gap:10px;"><span style="color:${C.ok};width:52px;">as vectors</span><span style="color:${C.mid};">faces</span></div>
    <div style="display:flex;gap:10px;"><span style="color:${C.err};width:52px;">never</span><span style="color:${C.mid};">frames, audio</span></div>
  </div>`, { tint: C.err }),
].join('');

const MOTOR_BODY = [
  sect('Device', rowField('Controller', 'Arduino Nano &middot; /dev/ttyACM0', { select: true, icon: 'plug' }) + rowField('Channels', '2 &middot; pan, tilt', { mono: true, gap: 0 })),
  sect('Limits', slider('Pan range', '&plusmn; 90&deg;', 100, CAT.actuators) + slider('Tilt range', '&plusmn; 30&deg;', 33, CAT.actuators) + rowField('Max speed', '60&deg; / s', { mono: true }) + switchRow('Warn before move', true, { hint: 'a prompt you can always continue through', col: C.warn }), { tint: C.err, right: chip('physical', C.err, { dot: true }) }),
  sect('Exposes', `<div style="display:flex;gap:5px;">${fnChip('motor.move')}${fnChip('motor.home')}</div><div style="margin:8px 0 10px;font-size:10px;line-height:1.5;color:${C.low};">Called by the model; each call returns the final position or a fault.</div><div style="display:flex;gap:5px;">${chip('state &middot; stream', T.stream, { dot: true })}${chip('fault &middot; exec', T.exec, { dot: true })}</div><div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Reported on its own ports, 20&times; a second, whether or not anyone asked.</div>`),
  sect('Live', rowField('Position', 'pan &minus;40&deg; &middot; tilt 5&deg;', { mono: true }) + rowField('Last', 'move(pan: &minus;40) &middot; warned, ran', { mono: true, gap: 0 }), { tint: C.warn, right: chip('1 warning', C.warn, { dot: true }) }),
].join('');

const SENSE_SHEET = doc(sheet({
  w: 1460, h: 1020, kicker: 'Embodied blocks',
  title: 'Senses, memory and actuators &mdash; the panel leads with the boundary that matters',
  body: `<div style="display:flex;gap:22px;">
  ${[['Webcam', 'a sense', WEBCAM_BODY, { title: 'Webcam', sub: 'senses &middot; video.capture', icn: 'eye', col: CAT.senses, tabs: ['Settings', 'Ports', 'Events'], tab: 'Settings' }, CAT.senses],
     ['Face recognition', 'a specialist model', FACE_BODY, { title: 'Face recognition', sub: 'models &middot; face.identify', icn: 'approve', col: CAT.models, tabs: ['Settings', 'Ports', 'Runs', 'People'], tab: 'Settings' }, CAT.models],
     ['Memory hub', 'short and long term, one handle', HUB_BODY, { title: 'Memory hub', sub: 'memory &middot; recall', icn: 'merge', col: CAT.memory, tabs: ['Settings', 'Ports', 'Browse'], tab: 'Settings' }, CAT.memory],
     ['Motors', 'an actuator, offered as a tool', MOTOR_BODY, { title: 'Motors', sub: 'actuators &middot; motor.servo', icn: 'loop', col: CAT.actuators, tabs: ['Settings', 'Ports', 'Runs'], tab: 'Settings' }, CAT.actuators],
    ].map(([cap, note, body, meta, col]) => `<div style="width:328px;flex:none;display:flex;flex-direction:column;gap:10px;">
    <div style="display:flex;align-items:baseline;gap:8px;">
      <span style="width:6px;height:6px;border-radius:50%;background:${col};flex:none;transform:translateY(-1px);"></span>
      <span style="font-size:12px;font-weight:600;color:${C.hi};">${cap}</span>
      <span style="font-size:10.5px;color:${C.low};">${note}</span>
    </div>
    <div style="height:850px;background:${C.panel};border:1px solid ${C.line};border-radius:10px;overflow:hidden;">${panelInner(body, meta)}</div>
  </div>`).join('')}
</div>`,
}));

/* ============================================================ 12. CustomBlock */

const kw = (t) => `<span style="color:${T.data};">${t}</span>`;
const st = (t) => `<span style="color:${C.ok};">${t}</span>`;
const dc = (t) => `<span style="color:${C.accent};">${t}</span>`;
const ty = (t) => `<span style="color:${T.image};">${t}</span>`;
const nm = (t) => `<span style="color:${C.warn};">${t}</span>`;
const cm = (t) => `<span style="color:${C.faint};">${t}</span>`;
const fn = (t) => `<span style="color:${C.hi};">${t}</span>`;
const SP = '&nbsp;&nbsp;&nbsp;&nbsp;';

const CODE = [
  `${kw('from')} canvas ${kw('import')} block, Image, Data`,
  ``,
  `${dc('@block')}(icon=${st('"shield"')}, category=${st('"senses"')})`,
  `${kw('def')} ${fn('door_check')}(frame: ${ty('Image')}, threshold: ${ty('float')} = ${nm('0.8')}) -> ${ty('Data')}:`,
  `${SP}${st('"""Is the front door open in this frame?"""')}`,
  `${SP}boxes = detect(frame, classes=[${st('"door"')}])`,
  `${SP}door = max(boxes, key=${kw('lambda')} b: b.score, default=${kw('None')})`,
  `${SP}${kw('if')} door ${kw('is')} ${kw('None')} ${kw('or')} door.score < threshold:`,
  `${SP}${SP}${kw('return')} {${st('"open"')}: ${kw('None')}, ${st('"confidence"')}: ${nm('0.0')}}`,
  `${SP}${kw('return')} {${st('"open"')}: door.aspect > ${nm('1.4')}, ${st('"confidence"')}: door.score}`,
];

function codeBlock(lines, { h = 214, marks = {}, fs = 10.5 } = {}) {
  return `<div style="height:${h}px;padding:9px 0;background:#07080a;border:1px solid ${C.soft};border-radius:6px;overflow:hidden;font-family:${MONO};font-size:${fs}px;line-height:19px;">
  ${lines.map((l, i) => `<div style="display:flex;align-items:center;gap:12px;padding:0 10px;${marks[i] ? `background:${rgba(C.accent, 0.06)};` : ''}">
    <span style="width:14px;flex:none;text-align:right;color:${C.faint};font-size:9.5px;">${i + 1}</span>
    <span style="flex:1;white-space:nowrap;overflow:hidden;color:#c3cad4;">${l || '&nbsp;'}</span>
    ${marks[i] ? `<span style="width:16px;height:16px;border-radius:50%;background:${C.accent};color:#08090b;font-size:9.5px;font-weight:700;display:flex;align-items:center;justify-content:center;flex:none;">${marks[i]}</span>` : ''}
  </div>`).join('')}
</div>`;
}

const ifaceChip = (dir, name, kind) => `<div style="display:flex;align-items:center;gap:6px;height:22px;padding:0 8px;border-radius:5px;background:${C.field};border:1px solid ${C.line};">
  <span style="font-family:${MONO};font-size:9px;color:${C.faint};">${dir}</span>
  <span style="font-family:${MONO};font-size:10px;color:${C.hi};">${name}</span>
  <span style="width:6px;height:6px;border-radius:50%;background:${T[kind] || C.mid};"></span>
  <span style="font-family:${MONO};font-size:9.5px;color:${C.mid};">${kind}</span>
</div>`;

const KB = {
  webcam: { x: 24, y: 140, w: 180 },
  custom: { x: 260, y: 70, w: 470 },
  notify: { x: 780, y: 90, w: 176 },
};
const kbw = W(KB);

const customEditorBody = `<div style="display:flex;align-items:center;gap:8px;margin-bottom:8px;">
  <div style="display:flex;gap:2px;padding:2px;background:${C.field};border:1px solid ${C.line};border-radius:6px;">
    <div style="display:flex;align-items:center;gap:5px;height:20px;padding:0 9px;border-radius:4px;background:${C.accent};font-size:10.5px;font-weight:600;color:#08090b;"><span style="width:5px;height:5px;border-radius:50%;background:#08090b;"></span>Inline</div>
    <div style="display:flex;align-items:center;height:20px;padding:0 9px;font-size:10.5px;color:${C.mid};">File</div>
  </div>
  ${chip('python 3.12', CAT.runtimes)}
  <span style="flex:1;"></span>
  <span style="font-family:${MONO};font-size:9.5px;color:${C.low};">Format</span>
  <span style="font-family:${MONO};font-size:9.5px;color:${C.low};">Test</span>
</div>
${codeBlock(CODE, { fs: 10 })}
<div style="display:flex;align-items:center;gap:6px;margin-top:9px;">
  <span style="font-family:${MONO};font-size:9px;font-weight:700;letter-spacing:.12em;text-transform:uppercase;color:${C.low};margin-right:2px;">interface</span>
  ${ifaceChip('in', 'frame', 'image')}${ifaceChip('out', 'result', 'data')}${ifaceChip('set', 'threshold', 'float')}
  <span style="flex:1;"></span>
  <span style="font-family:${MONO};font-size:9.5px;color:${C.ok};white-space:nowrap;">updated 0.2 s ago</span>
</div>`;

const customNodes = [
  blockNode({ ...KB.webcam, icon: 'eye', color: CAT.senses, title: 'Webcam', state: 'running',
    body: camPreview + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">1280&times;720 &middot; 15 fps</div>`,
    ports: [{ kind: 'image', label: 'frames', side: 'out' }] }),
  blockNode({ ...KB.custom, icon: 'shield', color: CAT.senses, title: 'door_check', state: 'ok', selected: true,
    badge: `<div style="display:flex;gap:6px;align-items:center;">${chip('reloaded', C.ok, { dot: true })}${chip('py', CAT.runtimes)}</div>`, view: 'code', third: 'code',
    body: customEditorBody,
    ports: [{ kind: 'image', label: 'frame', side: 'in' }, { kind: 'data', label: 'result', side: 'out' }] }),
  blockNode({ ...KB.notify, icon: 'note', color: CAT.human, title: 'Notify', state: 'idle',
    body: field('slack #home', { mono: true, select: true }) + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">when result.open</div>`,
    ports: [{ kind: 'data', label: 'data', side: 'in' }] }),
].join('\n');

const customSvg = [
  kbw('webcam', 0, 'custom', 0, 'image', { live: true, dash: '6 6' }),
  kbw('custom', 0, 'notify', 0, 'data'),
].join('');

const ifaceRow = (dir, name, kind, from) => `<div style="display:flex;align-items:center;gap:9px;height:34px;padding:0 9px;background:${C.field};border:1px solid ${C.line};border-radius:6px;margin-bottom:6px;">
  <span style="font-family:${MONO};font-size:9px;color:${C.faint};width:22px;">${dir}</span>
  <span style="font-family:${MONO};font-size:10.5px;color:${C.hi};width:64px;">${name}</span>
  ${chip(kind, T[kind] || C.mid, { dot: true })}
  <span style="flex:1;"></span>
  <span style="font-family:${MONO};font-size:9px;color:${C.faint};white-space:nowrap;">${from}</span>
</div>`;

const CUSTOM_BODY = [
  sect('Source', segmented(['Inline', 'File'], 'Inline', C.accent) + `<div style="height:11px;"></div>` + rowField('File', '~/blocks/door_check.py', { mono: true, icon: 'folder', suffix: chip('watching', C.ok, { dot: true }) }) + rowField('Runtime', 'Python 3.12 &middot; .venv', { select: true, gap: 0 })
    + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Switch to File to edit in your own editor. The block reloads on every save and keeps its wires.</div>`),
  sect('Interface', ifaceRow('in', 'frame', 'image', 'frame: Image') + ifaceRow('out', 'result', 'data', '-&gt; Data') + ifaceRow('set', 'threshold', 'float', '= 0.8')
    + `<div style="margin-top:4px;font-family:${MONO};font-size:9.5px;color:${C.faint};">parsed from the signature &middot; 0.2 s ago &middot; no errors</div>`,
    { tint: C.accent, right: chip('live', C.ok, { dot: true }) }),
  sect('Settings', slider('threshold', '0.80', 80) + `<div style="font-size:10px;line-height:1.5;color:${C.low};">Generated from the default argument. Change it here or in the code &mdash; they stay in sync.</div>`),
  sect('View', segmented(['Compact', 'Summary', 'Code'], 'Code', C.accent) + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">480 wide &middot; drag the corner. Remembered for this block on this graph.</div>`),
  sect('Library', rowField('Category', 'Senses', { select: true, icon: 'eye' }) + rowField('Icon', 'shield', { select: true, icon: 'shield', gap: 0 })
    + `<div style="margin-top:12px;display:flex;gap:8px;">${btn('Save to library', { primary: true })}${btn('Export .py')}</div>`),
].join('');

const CUSTOMBLOCK = doc(shell({
  top: topbar({ name: 'door-watch.graph', saved: 'edited &middot; just now' }),
  library: libraryPanel({ open: ['senses', 'custom'], placed: ['Webcam', 'door_check'] }),
  canvas: stage({ svg: customSvg, nodes: customNodes, overlay: zoomPill + minimap([[9, 15, 19, 12, CAT.senses], [37, 10, 47, 38, CAT.senses], [88, 12, 19, 8, CAT.human]]) }),
  insp: inspector(CUSTOM_BODY, { title: 'door_check', sub: 'custom &middot; python.block', icn: 'shield', col: CAT.senses, tabs: ['Settings', 'Ports', 'Source', 'Tests'], tab: 'Settings' }),
  status: statusbar('3 blocks &middot; 2 wires &middot; 1 custom', 'door_check reloaded 0.2 s ago &middot; interface unchanged'),
}));

/* ========================================================== 13. CustomRules */

const RB2 = { x: 40, y: 56, w: 260 };
const numBadge = (n, x, y) => `<span style="position:absolute;left:${x}px;top:${y}px;width:18px;height:18px;border-radius:50%;background:${C.accent};color:#08090b;font-family:${MONO};font-size:10px;font-weight:700;display:flex;align-items:center;justify-content:center;">${n}</span>`;

const resultBlock = blockNode({ ...RB2, icon: 'shield', color: CAT.senses, title: 'door_check', state: 'ok',
  badge: chip('py', CAT.runtimes),
  body: label('threshold') + field('0.80', { mono: true }) + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Is the front door open in this frame?</div>`,
  ports: [{ kind: 'image', label: 'frame', side: 'in' }, { kind: 'data', label: 'result', side: 'out' }] });

const ruleText = (title, note) => `<div style="flex:1;background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:13px 15px;">
  <div style="font-size:11.5px;font-weight:600;color:${C.hi};margin-bottom:5px;">${title}</div>
  <div style="font-size:10.5px;line-height:1.5;color:${C.low};">${note}</div>
</div>`;

const langCard = (name, col, lines) => `<div style="flex:1;background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:12px 14px;">
  <div style="display:flex;align-items:center;gap:7px;margin-bottom:8px;"><span style="width:6px;height:6px;border-radius:2px;background:${col};"></span><span style="font-family:${MONO};font-size:10px;font-weight:700;letter-spacing:.12em;text-transform:uppercase;color:${C.mid};">${name}</span></div>
  <div style="font-family:${MONO};font-size:10px;line-height:17px;color:#c3cad4;white-space:nowrap;overflow:hidden;">${lines.join('<br>')}</div>
</div>`;

const CUSTOMRULES = doc(sheet({
  w: 1400, h: 820, kicker: 'Custom blocks',
  title: 'The signature is the block &mdash; write code, get ports',
  body: `<div style="display:flex;gap:28px;align-items:flex-start;">
  <div style="width:700px;flex:none;">${codeBlock(CODE, { h: 214, marks: { 2: 1, 3: 2, 4: 5 } })}
    <div style="display:flex;gap:16px;margin-top:10px;font-family:${MONO};font-size:9.5px;color:${C.low};">
      <span>${dc('2')}&nbsp; frame: Image &rarr; input port</span><span>${dc('3')}&nbsp; threshold = 0.8 &rarr; setting</span><span>${dc('4')}&nbsp; -&gt; Data &rarr; output port</span>
    </div>
  </div>
  <div style="position:relative;flex:1;height:250px;">
    ${resultBlock}
    ${numBadge(1, RB2.x + RB2.w - 118, RB2.y - 9)}
    ${numBadge(2, RB2.x - 26, PY(RB2, 0) - 9)}
    ${numBadge(4, RB2.x + RB2.w + 8, PY(RB2, 0) - 9)}
    ${numBadge(3, RB2.x + RB2.w + 8, RB2.y + 31 + 36 + 4 + 20 + 6)}
    ${numBadge(5, RB2.x + RB2.w + 8, RB2.y + 31 + 36 + 4 + 20 + 30 + 8 + 2)}
    <div style="position:absolute;left:${RB2.x + RB2.w + 40}px;top:${RB2.y}px;width:230px;font-size:10.5px;line-height:1.55;color:${C.mid};">
      <div style="margin-bottom:8px;"><b style="color:${C.hi};">1</b>&nbsp; the decorator names the block and picks its shelf and icon</div>
      <div style="margin-bottom:8px;"><b style="color:${C.hi};">2</b>&nbsp; a parameter without a default is an input port, typed by its annotation</div>
      <div style="margin-bottom:8px;"><b style="color:${C.hi};">3</b>&nbsp; a parameter with a default is a setting in the inspector</div>
      <div style="margin-bottom:8px;"><b style="color:${C.hi};">4</b>&nbsp; the return annotation is the output port; a tuple makes several</div>
      <div><b style="color:${C.hi};">5</b>&nbsp; the docstring is the description</div>
    </div>
  </div>
</div>
<div style="display:flex;gap:16px;">
  ${ruleText('Reload keeps the wires', 'Save the file or leave the editor and the block re-parses. Ports that still exist keep their connections; a removed port drops its wire and says so in the console.')}
  ${ruleText('Errors keep the last good interface', 'A syntax error turns the block red with the line number. The previous ports stay so the graph keeps running around it.')}
  ${ruleText('Any type in the system', 'Annotate with Image, Audio, Text, Data, File, Tools or Memory. Untyped parameters become any. A parameter typed Tools makes the block a tool host of its own.')}
  ${ruleText('Save to library', 'A saved block lands under Custom, or under the category the decorator names, and can be dropped into any graph like a built-in.')}
</div>
<div style="display:flex;gap:16px;">
  ${langCard('Python', CAT.runtimes, [`${dc('@block')}(icon=${st('"shield"')})`, `${kw('def')} door_check(frame: ${ty('Image')}) -> ${ty('Data')}:`])}
  ${langCard('TypeScript', CAT.runtimes, [`${kw('export default')} block({ icon: ${st('"shield"')} },`, `${SP}(frame: ${ty('Image')}): ${ty('Data')} => { ... })`])}
  ${langCard('Shell', CAT.runtimes, [`${cm('# @block icon=shield')}`, `${cm('# @in frame:image  @out result:data')}`, `door_check() { ... }`])}
</div>`,
}));

/* ============================================================ 14. BlockViews */

const VB = { x: 0, y: 0, w: 250 };
const viewCompact = blockNode({ ...VB, grip: true, icon: 'shield', color: CAT.senses, title: 'door_check', state: 'ok',
  badge: `<div style="display:flex;gap:6px;align-items:center;">${chip('py', CAT.runtimes)}${viewToggle('compact')}</div>`,
  ports: [{ kind: 'image', label: 'frame', side: 'in' }, { kind: 'data', label: 'result', side: 'out' }] });
const viewSummary = blockNode({ ...VB, grip: true, icon: 'shield', color: CAT.senses, title: 'door_check', state: 'ok',
  badge: `<div style="display:flex;gap:6px;align-items:center;">${chip('py', CAT.runtimes)}${viewToggle('summary')}</div>`,
  body: label('threshold') + field('0.80', { mono: true }) + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Is the front door open in this frame?</div><div style="margin-top:7px;font-family:${MONO};font-size:9px;color:${C.faint};">12 lines &middot; reloaded 2 m ago</div>`,
  ports: [{ kind: 'image', label: 'frame', side: 'in' }, { kind: 'data', label: 'result', side: 'out' }] });
const viewCode = blockNode({ ...VB, w: 330, grip: true, icon: 'shield', color: CAT.senses, title: 'door_check', state: 'ok',
  badge: `<div style="display:flex;gap:6px;align-items:center;">${chip('py', CAT.runtimes)}${viewToggle('code')}</div>`,
  body: codeBlock(CODE.slice(0, 6), { h: 128 }) + `<div style="display:flex;align-items:center;margin-top:6px;font-family:${MONO};font-size:9px;color:${C.faint};"><span>&#8230; 4 more lines</span><span style="flex:1;"></span></div>`,
  ports: [{ kind: 'image', label: 'frame', side: 'in' }, { kind: 'data', label: 'result', side: 'out' }] });

const viewCol = (cap, note, block, w, h = 240) => `<div style="flex:none;width:${w}px;display:flex;flex-direction:column;gap:12px;">
  <div><div style="font-size:12px;font-weight:600;color:${C.hi};margin-bottom:3px;">${cap}</div><div style="font-size:10.5px;line-height:1.5;color:${C.low};">${note}</div></div>
  <div style="position:relative;height:${h}px;">${block}</div>
</div>`;

const AV_PORTS = [{ kind: 'audio', label: 'speech', side: 'in' }, { kind: 'data', label: 'express', side: 'in' }, { kind: 'data', label: 'look', side: 'in' }, { kind: 'tools', label: 'tool', side: 'out' }, { kind: 'stream', label: 'state', side: 'out' }];
const avCompact = blockNode({ ...VB, grip: true, icon: 'face', color: CAT.actuators, title: 'Avatar', state: 'running', badge: viewToggle('compact', 'stage'), ports: AV_PORTS });
const avSummary = blockNode({ ...VB, grip: true, icon: 'face', color: CAT.actuators, title: 'Avatar', state: 'running', badge: viewToggle('summary', 'stage'),
  body: `<div style="display:flex;align-items:center;gap:10px;"><div style="width:46px;height:46px;flex:none;border-radius:6px;background:${C.field};border:1px solid ${C.soft};display:flex;align-items:center;justify-content:center;">${rigFace('line', 'smile', 40)}</div><div style="font-family:${MONO};font-size:9.5px;line-height:1.6;color:${C.faint};">line &middot; smile<br>speaking &middot; looking at Mykl</div></div>`,
  ports: AV_PORTS });
const avStage = stageBlock({ ...VB, w: 260, h: 236, icon: 'face', color: CAT.actuators, title: 'Avatar', state: 'running',
  content: `<div style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center;">${rigFace('line', 'smile', 190)}</div><div style="position:absolute;left:9px;bottom:7px;font-family:${MONO};font-size:9px;color:${C.faint};">260 &times; 260</div>`,
  ports: AV_PORTS });

const shortcut = (k) => `<span style="font-family:${MONO};font-size:9.5px;color:${C.mid};border:1px solid ${C.line};border-radius:3px;padding:1px 5px;white-space:nowrap;">${k}</span>`;

const BLOCKVIEWS = doc(sheet({
  w: 1400, h: 1040, kicker: 'Block views',
  title: 'Compact and Summary for every block; Code or Stage for the ones that earn it',
  body: `<div style="display:flex;gap:40px;">
  ${viewCol('Compact', 'Name and ports only &mdash; what every block looks like from a distance, and the default once a block is saved to the library.', viewCompact, 250)}
  ${viewCol('Summary', 'Settings and description, no code. The default while the block is being used on a graph. Change a setting here or in the code; they stay in sync.', viewSummary, 250)}
  ${viewCol('Code', 'The editor, inline. Sized by you: drag the corner, it scrolls inside. Fine up to a screenful; past that the drawer below is the better place.', viewCode, 330)}
  <div style="flex:1;"></div>
</div>
<div style="display:flex;gap:40px;">
  ${viewCol('Compact', 'A block with a live picture has the same first two views as a custom block.', avCompact, 250, 300)}
  ${viewCol('Summary', 'A thumbnail and the current state. The default for a visual block on a graph.', avSummary, 250, 300)}
  ${viewCol('Stage', 'The picture fills the block. The header shrinks to a strip, port labels hide and only the dots stay on the edges, at exactly the y they had before: switching views never moves a wire.', avStage, 260, 300)}
  <div style="flex:1;display:flex;flex-direction:column;gap:14px;">
    <div style="background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:13px 15px;">
      <div style="font-size:11.5px;font-weight:600;color:${C.hi};margin-bottom:8px;">Switching</div>
      <div style="display:flex;flex-direction:column;gap:7px;font-size:10.5px;line-height:1.5;color:${C.low};">
        <div style="display:flex;align-items:center;gap:8px;">${viewToggle('summary')}<span>the toggle in a block's header, shown while hovered or selected; two positions when there is no third view</span></div>
        <div style="display:flex;align-items:center;gap:8px;">${shortcut('dbl-click')}<span>on the header cycles the views</span></div>
        <div style="display:flex;align-items:center;gap:8px;">${shortcut('&#8984;E')}<span>opens the third view of the selected block</span></div>
        <div>The third view is <b style="color:${C.mid};">Code</b> for a custom block and <b style="color:${C.mid};">Stage</b> for a block with a live picture: Avatar, Webcam, Display, Terminal, Object detection. The view is remembered per block, per graph.</div>
      </div>
    </div>
    <div style="background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:13px 15px;">
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px;"><span style="font-size:11.5px;font-weight:600;color:${C.hi};">Resizing</span><span style="position:relative;width:18px;height:18px;border:1px solid ${C.line};border-radius:3px;">${grip}</span></div>
      <div style="display:flex;flex-direction:column;gap:7px;font-size:10.5px;line-height:1.5;color:${C.low};">
        <div><b style="color:${C.mid};">Drag the corner.</b> In Summary it sets width and the body reflows. In Stage or Code it sets width and height; the picture scales, the code scrolls.</div>
        <div>Snaps to the 22 px grid. Minimum is the compact size. Remembered per block, per graph. An Avatar keeps its aspect unless you unlock it in the inspector.</div>
      </div>
    </div>
    <div style="background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:13px 15px;">
      <div style="font-size:11.5px;font-weight:600;color:${C.hi};margin-bottom:8px;">Big programs</div>
      <div style="display:flex;flex-direction:column;gap:7px;font-size:10.5px;line-height:1.5;color:${C.low};">
        <div><b style="color:${C.mid};">Code drawer</b> &mdash; full width under the canvas, beside Console and Trace; the block stays compact above it.</div>
        <div><b style="color:${C.mid};">File mode</b> &mdash; point the block at a file and edit in your own editor; the block reloads on save.</div>
        <div><b style="color:${C.mid};">Split into blocks</b> &mdash; a file with several <span style="font-family:${MONO};">@block</span> functions makes several blocks, one per function.</div>
      </div>
    </div>
  </div>
</div>`,
}));

/* ========================================================= 15. CustomDrawer */

const DRAWER_H = 300;
const DB = {
  webcam: { x: 24, y: 120, w: 180 },
  custom: { x: 300, y: 96, w: 250 },
  hub: { x: 300, y: 330, w: 200 },
  notify: { x: 640, y: 110, w: 176 },
};
const dbw = W(DB);

const CODE_LONG = [
  `${kw('from')} canvas ${kw('import')} block, Image, Data, Memory`,
  `${kw('from')} .vision ${kw('import')} detect, track`,
  `${kw('from')} .config ${kw('import')} DOOR_CLASSES, MIN_ASPECT`,
  ``,
  `_history: list[dict] = []`,
  ``,
  `${kw('def')} ${fn('_aspect')}(box) -> ${ty('float')}:`,
  `${SP}${kw('return')} box.w / max(box.h, ${nm('1')})`,
  ``,
  `${kw('def')} ${fn('_stable')}(open_: ${ty('bool')}, n: ${ty('int')} = ${nm('3')}) -> ${ty('bool')}:`,
  `${SP}_history.append({${st('"open"')}: open_})`,
  `${SP}recent = [h[${st('"open"')}] ${kw('for')} h ${kw('in')} _history[-n:]]`,
  `${SP}${kw('return')} len(recent) == n ${kw('and')} all(r == open_ ${kw('for')} r ${kw('in')} recent)`,
  ``,
  `${dc('@block')}(icon=${st('"shield"')}, category=${st('"senses"')})`,
  `${kw('def')} ${fn('door_check')}(frame: ${ty('Image')}, memory: ${ty('Memory')}, threshold: ${ty('float')} = ${nm('0.8')}) -> ${ty('Data')}:`,
];

const codeDrawer = `<div style="height:${DRAWER_H}px;flex:none;display:flex;flex-direction:column;background:#0a0c10;border-top:1px solid ${C.line};">
  <div style="display:flex;align-items:center;gap:18px;height:33px;flex:none;padding:0 14px;border-bottom:1px solid ${C.soft};">
    <span style="display:flex;align-items:center;gap:7px;font-size:11.5px;color:${C.hi};border-bottom:1.5px solid ${C.accent};height:33px;">${icon('code', 12, C.accent, 2)}door_check.py<span style="width:5px;height:5px;border-radius:50%;background:${C.warn};"></span></span>
    ${['Console', 'Trace', 'Variables'].map(t => `<span style="font-size:11.5px;color:${C.low};height:33px;display:flex;align-items:center;">${t}</span>`).join('')}
    <span style="flex:1;"></span>
    <span style="font-family:${MONO};font-size:10px;color:${C.low};">184 lines &middot; python 3.12 &middot; saved 4 s ago</span>
    ${chip('2 blocks in file', CAT.custom)}
    <div style="display:flex;align-items:center;gap:6px;height:24px;padding:0 10px;border-radius:5px;border:1px solid ${C.line};font-size:10.5px;color:${C.hi};">${icon('output', 11, C.mid, 1.8)}Open in editor</div>
    <span style="transform:rotate(90deg);display:flex;">${icon('chev', 12, C.low, 2)}</span>
  </div>
  <div style="flex:1;display:flex;min-height:0;">
    <div style="flex:1;padding:8px 0;font-family:${MONO};font-size:10.5px;line-height:19px;overflow:hidden;">
      ${CODE_LONG.map((l, i) => `<div style="display:flex;align-items:center;gap:12px;padding:0 14px;${i === 15 ? `background:${rgba(C.accent, 0.06)};` : ''}"><span style="width:20px;flex:none;text-align:right;color:${C.faint};font-size:9.5px;">${i + 1}</span><span style="flex:1;white-space:nowrap;overflow:hidden;color:#c3cad4;">${l || '&nbsp;'}</span>${i === 15 ? `<span style="font-size:9px;color:${C.accent};white-space:nowrap;">&larr; door_check &middot; 3 ports &middot; 1 setting</span>` : ''}</div>`).join('')}
    </div>
    <div style="width:10px;flex:none;position:relative;background:${C.field};border-left:1px solid ${C.soft};"><div style="position:absolute;left:2px;right:2px;top:4px;height:22px;border-radius:3px;background:${C.faint};"></div></div>
  </div>
</div>`;

const customDrawerNodes = [
  blockNode({ ...DB.webcam, icon: 'eye', color: CAT.senses, title: 'Webcam', state: 'running',
    body: camPreview + `<div style="margin-top:7px;font-family:${MONO};font-size:9.5px;color:${C.faint};">1280&times;720 &middot; 15 fps</div>`,
    ports: [{ kind: 'image', label: 'frames', side: 'out' }] }),
  blockNode({ ...DB.custom, icon: 'shield', color: CAT.senses, title: 'door_check', state: 'ok', selected: true,
    badge: chip('py', CAT.runtimes), view: 'summary', third: 'code',
    body: label('threshold') + field('0.80', { mono: true }) + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Is the front door open, and has it been for three frames?</div><div style="margin-top:7px;font-family:${MONO};font-size:9px;color:${C.faint};">184 lines &middot; editing below</div>`,
    ports: [{ kind: 'image', label: 'frame', side: 'in' }, { kind: 'memory', label: 'memory', side: 'in' }, { kind: 'data', label: 'result', side: 'out' }] }),
  blockNode({ ...DB.hub, icon: 'merge', color: CAT.memory, title: 'Memory hub', state: 'ok',
    body: `<div style="font-family:${MONO};font-size:9.5px;color:${C.faint};">working &middot; long-term</div>`,
    ports: [{ kind: 'memory', label: 'memory', side: 'out' }] }),
  blockNode({ ...DB.notify, icon: 'note', color: CAT.human, title: 'Notify', state: 'idle',
    body: field('slack #home', { mono: true, select: true }),
    ports: [{ kind: 'data', label: 'data', side: 'in' }] }),
].join('\n');

const customDrawerSvg = [
  dbw('webcam', 0, 'custom', 0, 'image', { live: true, dash: '6 6' }),
  dbw('hub', 0, 'custom', 1, 'memory'),
  dbw('custom', 0, 'notify', 0, 'data'),
].join('');

const CUSTOM_BODY_FILE = [
  sect('Source', segmented(['Inline', 'File'], 'File', C.accent) + `<div style="height:11px;"></div>` + rowField('File', '~/blocks/door_check.py', { mono: true, icon: 'folder', suffix: chip('watching', C.ok, { dot: true }) }) + rowField('Runtime', 'Python 3.12 &middot; .venv', { select: true, gap: 0 })
    + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Edited in the drawer or in your own editor &mdash; the block reloads on every save and keeps its wires.</div>`),
  sect('Interface', ifaceRow('in', 'frame', 'image', 'frame: Image') + ifaceRow('in', 'memory', 'memory', 'memory: Memory') + ifaceRow('out', 'result', 'data', '-&gt; Data') + ifaceRow('set', 'threshold', 'float', '= 0.8')
    + `<div style="margin-top:4px;font-family:${MONO};font-size:9.5px;color:${C.faint};">parsed from the signature &middot; 4 s ago &middot; +1 port since last save</div>`,
    { tint: C.accent, right: chip('live', C.ok, { dot: true }) }),
  sect('View', segmented(['Compact', 'Summary', 'Code'], 'Summary', C.accent) + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Remembered for this block on this graph.</div>`),
  sect('Settings', slider('threshold', '0.80', 80)),
].join('');

const CUSTOMDRAWER = doc(shell({
  top: topbar({ name: 'door-watch.graph', saved: 'edited &middot; 4 s ago' }),
  library: libraryPanel({ open: ['senses', 'custom'], placed: ['Webcam', 'door_check'] }),
  canvas: `<div style="width:${CW}px;flex:none;display:flex;flex-direction:column;min-height:0;">
    ${stage({ svg: customDrawerSvg, nodes: customDrawerNodes, overlay: zoomPill, h: CH - DRAWER_H })}
    ${codeDrawer}
  </div>`,
  insp: inspector(CUSTOM_BODY_FILE, { title: 'door_check', sub: 'custom &middot; python.block &middot; file', icn: 'shield', col: CAT.senses, tabs: ['Settings', 'Ports', 'Source', 'Tests'], tab: 'Settings' }),
  status: statusbar('4 blocks &middot; 3 wires &middot; 1 custom', 'door_check.py &middot; 184 lines &middot; reloaded 4 s ago &middot; +1 port'),
}));

/* ================================================================ 16. Avatar */

const RIGS = [['line', 'Line'], ['robot', 'Robot'], ['orb', 'Orb'], ['pixel', 'Pixel']];

const rigThumb = (rig, name, on) => `<div style="flex:1;display:flex;flex-direction:column;align-items:center;gap:5px;padding:7px 4px 6px;border-radius:7px;background:${on ? rgba(CAT.actuators, 0.12) : C.field};border:1px solid ${on ? rgba(CAT.actuators, 0.5) : C.line};">${rigFace(rig, 'neutral', 40)}<span style="font-size:10.5px;color:${on ? C.hi : C.mid};">${name}</span></div>`;

const VOCAB = ['neutral', 'smile', 'frown', 'surprised', 'thinking', 'speaking', 'love', 'sleepy', 'look', 'nod', 'shake'];

function avatarBody(stage = false) { return [
  sect('Rig', `<div style="display:flex;gap:6px;">${RIGS.map(([r, n]) => rigThumb(r, n, r === 'line')).join('')}</div><div style="margin-top:9px;display:flex;align-items:center;gap:8px;"><span style="font-size:10px;color:${C.low};">Rigs are content, not code.</span><span style="flex:1;"></span>${chip('add rig&#8230;', C.mid)}</div>`, { tint: CAT.actuators, right: chip('line', CAT.actuators) }),
  sect('Vocabulary', `<div style="display:flex;flex-wrap:wrap;gap:4px;margin-bottom:9px;">${VOCAB.map(v => chip(v, T.tools)).join('')}</div>` + rowField('Offered as', 'face.express, face.look, face.gesture', { mono: true, gap: 0 }) + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Generated from the rig. A rig without <span style="font-family:${MONO};">frown</span> simply doesn't offer it.</div>`, { right: chip('11', T.tools) }),
  sect('Inputs', connRow('note', 'speech', 'Text to speech &middot; lip sync from amplitude', 'audio', 'ok') + connRow('face', 'express', 'Affect &middot; from the orchestrator\'s own words', 'data', 'running') + connRow('approve', 'look', 'Face recognition &middot; gaze follows a person', 'data', 'ok') + switchRow('Auto-affect from speech', false, { hint: 'off &mdash; an Affect block feeds express instead' })),
  sect('Idle', rowField('Blink', 'every 3&ndash;6 s', { mono: true }) + rowField('Breathe', 'on &middot; 12 / min', { mono: true }) + rowField('Settle to neutral after', '4 s', { mono: true }) + rowField('Sleep after', '10 min without events', { select: true, gap: 0 })),
  sect('View', segmented(['Compact', 'Summary', 'Stage'], stage ? 'Stage' : 'Summary', C.accent) + `<div style="height:11px;"></div>` + rowField('Size on canvas', stage ? '240 &times; 240 &middot; drag the corner' : '200 wide &middot; drag the corner', { mono: true }) + switchRow('Keep aspect', true, { hint: 'the rig stays square while you resize' })),
  sect('Output', rowField('Target', 'Window &middot; always on top', { select: true, icon: 'form' }) + rowField('Size', '480 &times; 480', { mono: true, gap: 0 }) + `<div style="margin-top:8px;font-size:10px;line-height:1.5;color:${C.low};">Or a physical face over USB &mdash; the avatar calls <span style="font-family:${MONO};">face.render</span> on a device block.</div>`),
  sect('Live', `<div style="display:flex;align-items:center;gap:10px;">${rigFace('line', 'smile', 44)}<div style="font-family:${MONO};font-size:10px;line-height:1.7;color:${C.mid};">smile &middot; 0.8<br>speaking &middot; looking at Mykl</div></div>`, { tint: C.ok, right: chip('running', C.ok, { dot: true }) }),
].join(''); }
const AVATAR_BODY = avatarBody(false);

const vocabRow = (name, note, rigs) => `<div style="display:grid;grid-template-columns:110px minmax(0,1fr) 150px;gap:14px;align-items:center;height:30px;padding:0 10px;border-bottom:1px solid ${C.soft};">
  <span style="font-family:${MONO};font-size:10.5px;color:${C.hi};">${name}</span>
  <span style="font-size:10.5px;color:${C.low};">${note}</span>
  <span style="display:flex;gap:4px;">${RIGS.map(([r]) => `<span style="width:8px;height:8px;border-radius:2px;background:${rigs.includes(r) ? CAT.actuators : C.line};"></span>`).join('')}</span>
</div>`;

const AVATAR_SHEET = doc(sheet({
  w: 1200, h: 1370, kicker: 'Presence',
  title: 'Four rigs, one vocabulary &mdash; the rig decides what the model may ask for',
  body: `<div style="display:flex;gap:26px;align-items:flex-start;">
  <div style="width:328px;flex:none;display:flex;flex-direction:column;gap:10px;">
    <div style="display:flex;align-items:baseline;gap:8px;"><span style="width:6px;height:6px;border-radius:50%;background:${CAT.actuators};flex:none;transform:translateY(-1px);"></span><span style="font-size:12px;font-weight:600;color:${C.hi};">Avatar</span><span style="font-size:10.5px;color:${C.low};">the assistant's presence</span></div>
    <div style="height:1200px;background:${C.panel};border:1px solid ${C.line};border-radius:10px;overflow:hidden;">${panelInner(AVATAR_BODY, { title: 'Avatar', sub: 'actuators &middot; avatar.rig', icn: 'face', col: CAT.actuators, tabs: ['Settings', 'Ports', 'Runs', 'Rigs'], tab: 'Settings' })}</div>
  </div>
  <div style="flex:1;display:flex;flex-direction:column;gap:18px;">
    <div style="background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:14px 16px 12px;">
      <div style="display:grid;grid-template-columns:70px repeat(7, minmax(0, 1fr));gap:6px;align-items:center;">
        <span></span>
        ${RIG_EXPR.map(e => `<span style="font-family:${MONO};font-size:9.5px;letter-spacing:.06em;color:${C.mid};text-align:center;">${e}</span>`).join('')}
        ${RIGS.map(([r, n]) => `<span style="font-size:11.5px;font-weight:600;color:${C.hi};">${n}</span>` + RIG_EXPR.map(e => `<div style="display:flex;align-items:center;justify-content:center;height:96px;border-radius:8px;background:${C.field};border:1px solid ${C.soft};">${rigFace(r, e, 72)}</div>`).join('')).join('')}
      </div>
      <div style="margin-top:10px;font-size:10.5px;line-height:1.5;color:${C.low};">Same seven commands, four answers. Add a rig as a folder of states (Rive files are the natural format); it appears in the picker and its vocabulary is read from what it contains.</div>
    </div>
    <div style="background:${C.panel};border:1px solid ${C.line};border-radius:10px;overflow:hidden;">
      <div style="display:grid;grid-template-columns:110px minmax(0,1fr) 150px;gap:14px;padding:9px 10px;border-bottom:1px solid ${C.soft};font-family:${MONO};font-size:9.5px;letter-spacing:.12em;text-transform:uppercase;color:${C.low};"><span>command</span><span>what it does</span><span>line &middot; robot &middot; orb &middot; pixel</span></div>
      ${vocabRow('neutral', 'the resting face; idle returns here', ['line', 'robot', 'orb', 'pixel'])}
      ${vocabRow('smile / frown', 'valence, with an intensity 0&ndash;1', ['line', 'robot', 'orb', 'pixel'])}
      ${vocabRow('surprised', 'a beat, then settles', ['line', 'robot', 'orb', 'pixel'])}
      ${vocabRow('thinking', 'held while the orchestrator streams thoughts', ['line', 'robot', 'orb', 'pixel'])}
      ${vocabRow('speaking', 'driven by the speech port, not by a command', ['line', 'robot', 'orb', 'pixel'])}
      ${vocabRow('love', 'affection; heart eyes, a rose glow, or the whole matrix', ['line', 'robot', 'orb', 'pixel'])}
      ${vocabRow('sleepy', 'after the sleep timeout; any event wakes it', ['line', 'robot', 'orb'])}
      ${vocabRow('look(at)', 'gaze to a point or a person; from the look port or a call', ['line', 'robot', 'orb'])}
      ${vocabRow('nod / shake', 'gestures; a one-shot animation', ['line', 'robot'])}
    </div>
    <div style="display:flex;gap:16px;">
      ${ruleText('Intent from the model, timing from the wires', 'A tool call sets what the face means; the speech audio sets when the mouth moves; the look port sets where it looks. None of the three waits for the others.')}
      ${ruleText('Alive between turns', 'Blink, breathe, drift and settle are the block\'s own. With no events for ten minutes it sleeps; the next event wakes it.')}
      ${ruleText('One of a family', 'A Status light breathes a colour and a Sound cue plays a chime on the same commands. Same vocabulary pattern, different medium.')}
    </div>
  </div>
</div>`,
}));

/* =================================================================== write */

const files = {
  'EmptyShell.dc.html': EMPTY,
  'Main.dc.html': MAIN,
  'Running.dc.html': RUNNING,
  'Inspector.dc.html': INSPECTOR_SHEET,
  'Library.dc.html': LIBRARY_SHEET,
  'BlockAnatomy.dc.html': ANATOMY,
  'Continuous.dc.html': CONTINUOUS,
  'RunModes.dc.html': RUNMODES_SHEET,
  'Assistant.dc.html': assistant(false),
  'AssistantStage.dc.html': assistant(true),
  'SensePanels.dc.html': SENSE_SHEET,
  'CustomBlock.dc.html': CUSTOMBLOCK,
  'CustomRules.dc.html': CUSTOMRULES,
  'BlockViews.dc.html': BLOCKVIEWS,
  'Avatar.dc.html': AVATAR_SHEET,
  'CustomDrawer.dc.html': CUSTOMDRAWER,
  'Interactive.dc.html': INTERACTIVE,
};

const canvas = {
  artboards: [
    { file: 'EmptyShell.dc.html', x: 0, y: 0, w: 1560, h: 900, page: 'page-1', title: 'Empty shell' },
    { file: 'Main.dc.html', x: 1680, y: 0, w: 1560, h: 900, page: 'page-1', title: 'Wiring a block in' },
    { file: 'Running.dc.html', x: 3360, y: 0, w: 1560, h: 900, page: 'page-1', title: 'Graph running' },
    { file: 'Inspector.dc.html', x: 0, y: 1080, w: 1800, h: 980, page: 'page-1', title: 'Inspector states' },
    { file: 'Library.dc.html', x: 1920, y: 1080, w: 1470, h: 1250, page: 'page-1', title: 'Block library' },
    { file: 'BlockAnatomy.dc.html', x: 3510, y: 1080, w: 1500, h: 760, page: 'page-1', title: 'Block anatomy' },
    { file: 'Continuous.dc.html', x: 0, y: 0, w: 1560, h: 900, page: 'page-2', title: 'Live graph' },
    { file: 'RunModes.dc.html', x: 1680, y: 0, w: 1120, h: 980, page: 'page-2', title: 'Run modes' },
    { file: 'Assistant.dc.html', x: 0, y: 1160, w: 1920, h: 1080, page: 'page-2', title: 'Home assistant' },
    { file: 'SensePanels.dc.html', x: 2040, y: 1160, w: 1460, h: 1020, page: 'page-2', title: 'Embodied panels' },
    { file: 'Avatar.dc.html', x: 3620, y: 1160, w: 1200, h: 1370, page: 'page-2', title: 'Avatar and rigs' },
    { file: 'AssistantStage.dc.html', x: 0, y: 2400, w: 1920, h: 1080, page: 'page-2', title: 'Avatar staged' },
    { file: 'CustomBlock.dc.html', x: 0, y: 0, w: 1560, h: 900, page: 'page-3', title: 'Custom block' },
    { file: 'CustomRules.dc.html', x: 1680, y: 0, w: 1400, h: 820, page: 'page-3', title: 'Code becomes a block' },
    { file: 'CustomDrawer.dc.html', x: 0, y: 1080, w: 1560, h: 900, page: 'page-3', title: 'Big program, small block' },
    { file: 'BlockViews.dc.html', x: 1680, y: 1080, w: 1400, h: 1040, page: 'page-3', title: 'Block views' },
    { file: 'Interactive.dc.html', x: 0, y: 0, w: 1560, h: 900, page: 'page-4', title: 'Clickable shell', is_interactive: true },
  ],
  annotations: [
    { id: 'brief', x: 0, y: -186, w: 700, page: 'page-1',
      text: 'Block Canvas - a shell for wiring blocks into runnable graphs.\n\nLeft: the library, categorised by what a block does. Centre: an infinite node canvas. Right: an inspector that reshapes itself around whatever is selected.\n\nTop row is the same shell at three moments: nothing built, mid-wire, mid-run.' },
    { id: 'panel-note', x: 0, y: 940, w: 560, page: 'page-1',
      text: 'The right panel is the whole idea: five different selections, five different panels, one 328px column.' },
    { id: 'types-note', x: 1920, y: 940, w: 520, page: 'page-1',
      text: 'Every block declares typed ports. A wire is coloured by its data type, and the type is what makes a connection legal or refuses it mid-drag. Ten types, nine categories plus Custom.' },
    { id: 'anatomy-note', x: 3510, y: 940, w: 560, page: 'page-1',
      text: 'One block, labelled, plus every run state - and the wiring rules. Handle wires (tools, memory) carry a two-way mark at the holder\'s end: the call goes out, the reply comes back.' },
    { id: 'live-note', x: 0, y: -200, w: 720, page: 'page-2',
      text: 'Continuous running.\n\nA graph with a source block (Senses: watch folder, webhook, schedule, webcam...) never finishes - it stays armed and every event runs downstream. The transport chip switches from Run to live, and the inspector with nothing selected becomes the run-mode panel.\n\nA Loop is a dashed frame on the canvas: blocks inside repeat once per item.' },
    { id: 'asst-note', x: 0, y: 1010, w: 760, page: 'page-2',
      text: 'The home assistant, as one graph. Left to right: senses feed specialist models, which feed an orchestrator LLM; memory stores bundle through a Memory hub the same way tools bundle through a Toolbox; the model\'s text goes to a display and a speaker, its thoughts to a terminal, and its actions to motors - via a tool call that warns first and then runs. An Avatar gives it a face: lip sync from the speech audio, expression from an Affect model on its own words, gaze from face recognition.\n\nFeedback: the Motors block replies on its tool handle, streams its state into the orchestrator\'s context, and raises a fault on an exec port that pauses the Toolbox until you resume it.\n\nWarn, never block: the user owns their tools. A truly dangerous action gets a prompt; the prompt always has a Continue.' },
    { id: 'custom-note', x: 0, y: -170, w: 700, page: 'page-3',
      text: 'Custom blocks.\n\nWrite code inline or point the block at a file it watches. The signature is the interface: parameters without defaults are input ports, parameters with defaults are settings, the return annotation is the output. Save it and it sits in the library like a built-in.\n\nThe code is one view of the block, not the block: compact, summary or code, per block. A big program lives in the drawer under the canvas, or in your own editor, while the block on the canvas stays small.' },
    { id: 'avatar-note', x: 3620, y: 1010, w: 620, page: 'page-2',
      text: 'Avatar: the block that gives the assistant a presence. A rig is one aesthetic plus the expressions it supports; the vocabulary the model can call is generated from the rig. Lip sync comes from the speech audio, never from the model.' },
    { id: 'stage-note', x: 0, y: 2250, w: 700, page: 'page-2',
      text: 'The same graph with the Avatar in Stage view: the picture fills the block, the header is a strip, ports are dots on the edges at the same y as before. Drag the corner to resize; the rig keeps its aspect. Every block with a live picture has a Stage view.' },
    { id: 'proto-note', x: 0, y: -150, w: 620, page: 'page-4',
      text: 'Click any block to watch the inspector swap. Press Run to put the graph in flight - wires animate, status dots turn green, the console reports.' },
  ],
  pages: [
    { id: 'page-1', name: 'Screens' },
    { id: 'page-2', name: 'Live and embodied' },
    { id: 'page-3', name: 'Custom blocks' },
    { id: 'page-4', name: 'Clickable' },
  ],
  launch: { view: 'canvas', page: 'page-2' },
};

for (const [name, html] of Object.entries(files)) writeFileSync(name, html);
writeFileSync('canvas.json', JSON.stringify(canvas, null, 2));
console.log('wrote', Object.keys(files).length, 'artboards + canvas.json');
