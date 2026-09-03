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
  exec:   '#e8ebf0',
  any:    '#8a93a3',
};

const CAT = {
  models:'#56c7d6', capabilities:'#e0a458', runtimes:'#6fc98a',
  data:'#a78bd0', control:'#8a93a3', human:'#d97f8f',
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
  return `<div style="position:absolute;left:${o.x}px;top:${o.y}px;width:${o.w}px;background:${C.block};border:1px solid ${borderCol};border-radius:9px;box-shadow:${shadow};${ghost}">
  <div style="display:flex;align-items:center;gap:8px;height:31px;padding:0 10px;border-bottom:1px solid ${C.soft};border-radius:8px 8px 0 0;background:linear-gradient(180deg,${rgba(c, 0.13)},${rgba(c, 0.02)});">
    ${icon(o.icon, 13, c, 1.7)}
    <span style="font-size:12px;font-weight:600;letter-spacing:-.005em;color:${C.hi};">${o.title}</span>
    <span style="flex:1;"></span>
    ${o.badge || ''}
    ${statusDot(o.state || 'idle')}
  </div>
  <div style="position:relative;padding:${o.pad || '10px 11px 12px'};">${o.body || ''}</div>
  ${(o.ports || []).map(port).join('\n  ')}
</div>`;
}

function wire(x1, y1, x2, y2, kind, opt = {}) {
  const c = T[kind] || kind;
  const dx = Math.max(48, Math.abs(x2 - x1) * 0.55, Math.abs(y2 - y1) * 0.22);
  const d = `M${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
  const halo = `<path d="${d}" fill="none" stroke="${c}" stroke-width="${opt.live ? 7 : 5}" stroke-linecap="round" opacity="${opt.live ? 0.16 : 0.09}"/>`;
  const core = `<path d="${d}" fill="none" stroke="${c}" stroke-width="${opt.width || 1.9}" stroke-linecap="round" opacity="${opt.opacity ?? 0.95}"${opt.dash ? ` stroke-dasharray="${opt.dash}"` : ''}${opt.live ? ' style="animation:flow .85s linear infinite;"' : ''}/>`;
  return halo + core;
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

const chip = (t, col, opt = {}) => `<span style="display:inline-flex;align-items:center;gap:5px;height:20px;padding:0 8px;border-radius:5px;background:${rgba(col, opt.solid ? 0.9 : 0.12)};border:1px solid ${rgba(col, 0.3)};font-family:${MONO};font-size:9.5px;font-weight:600;letter-spacing:.04em;color:${opt.solid ? '#0b0d11' : col};">${opt.dot ? `<span style="width:5px;height:5px;border-radius:50%;background:${col};"></span>` : ''}${t}</span>`;

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

function topbar({ name = 'untitled.graph', saved = 'saved', running = false, elapsed = '' } = {}) {
  const transport = running
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
      <span style="font-family:${MONO};font-size:10px;color:${C.mid};">local &middot; ollama</span>
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
    { n:'LLM', i:'llm', sig:'text, tools &rarr; text', t:['text','tools'] },
    { n:'Vision', i:'eye', sig:'image &rarr; text', t:['file','text'] },
    { n:'Embedding', i:'embed', sig:'text &rarr; vector', t:['text','data'] },
    { n:'Classifier', i:'shield', sig:'text &rarr; label', t:['text','data'] },
  ]},
  { id:'capabilities', name:'Capabilities', blocks:[
    { n:'Toolbox', i:'toolbox', sig:'tool[] &rarr; tools', t:['tools'] },
    { n:'Web Search', i:'search', sig:'query &rarr; results', t:['text','data'] },
    { n:'File System', i:'folder', sig:'path &rarr; file', t:['text','file'] },
    { n:'MCP Server', i:'plug', sig:'&rarr; tool[]', t:['tools'] },
  ]},
  { id:'runtimes', name:'Runtimes', blocks:[
    { n:'Terminal', i:'terminal', sig:'cmd &rarr; stdout', t:['text','stream'] },
    { n:'Python', i:'python', sig:'src &rarr; value', t:['text','data'] },
    { n:'Node', i:'bolt', sig:'src &rarr; value', t:['text','data'] },
    { n:'SQL', i:'db', sig:'query &rarr; rows', t:['text','data'] },
    { n:'HTTP Request', i:'http', sig:'url &rarr; json', t:['text','data'] },
  ]},
  { id:'data', name:'Data', blocks:[
    { n:'Input', i:'input', sig:'&rarr; any', t:['any'] },
    { n:'Output', i:'output', sig:'any &rarr;', t:['any'] },
    { n:'Variable', i:'braces', sig:'any &rarr; any', t:['any'] },
    { n:'Chunker', i:'chunk', sig:'text &rarr; text[]', t:['text'] },
    { n:'Vector Store', i:'db', sig:'vector &rarr; match[]', t:['data'] },
    { n:'Secret', i:'key', sig:'&rarr; string', t:['text'] },
  ]},
  { id:'control', name:'Control', blocks:[
    { n:'Branch', i:'branch', sig:'any &rarr; a | b', t:['exec'] },
    { n:'Loop', i:'loop', sig:'any[] &rarr; any', t:['exec'] },
    { n:'Merge', i:'merge', sig:'any[] &rarr; any', t:['exec'] },
    { n:'Gate', i:'shield', sig:'any &rarr; any', t:['exec'] },
    { n:'Delay', i:'clock', sig:'any &rarr; any', t:['exec'] },
  ]},
  { id:'human', name:'Human', blocks:[
    { n:'Approval', i:'approve', sig:'any &rarr; any | halt', t:['exec'] },
    { n:'Form', i:'form', sig:'&rarr; record', t:['data'] },
    { n:'Notify', i:'note', sig:'text &rarr;', t:['text'] },
  ]},
];

const typeDots = (ts) => `<span style="display:flex;align-items:center;gap:3px;flex:none;">${ts.map(k => `<span style="width:5px;height:5px;border-radius:50%;background:${T[k]};"></span>`).join('')}</span>`;

function libRow(b, catCol, opt = {}) {
  const bg = opt.state === 'placed' ? rgba(catCol, 0.1)
    : opt.state === 'drag' ? rgba(C.accent, 0.08) : 'transparent';
  const bd = opt.state === 'placed' ? rgba(catCol, 0.4)
    : opt.state === 'drag' ? rgba(C.accent, 0.4) : 'transparent';
  return `<div style="display:flex;align-items:center;gap:9px;height:29px;padding:0 8px;border-radius:6px;background:${bg};border:1px solid ${bd};">
  ${icon(b.i, 13, catCol, 1.7)}
  <span style="flex:1;font-size:11.5px;color:${C.hi};letter-spacing:-.005em;">${b.n}</span>
  ${opt.state === 'placed' ? chip('on canvas', catCol) : typeDots(b.t)}
</div>`;
}

function catHeader(cat, open, count) {
  const col = CAT[cat.id];
  return `<div style="display:flex;align-items:center;gap:7px;height:26px;padding:0 6px;">
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
  sect('Tools', connRow('terminal', 'Terminal', 'shell.exec &middot; sandboxed', 'stream', 'ok')
    + connRow('python', 'Python', 'python.exec &middot; 3.12', 'data', 'ok')
    + connRow('toolbox', 'Toolbox', 'tools &rarr; llm.tools', 'tools', 'pending')
    + `<div style="height:5px;"></div>` + rowField('Tool choice', 'auto', { select: true, gap: 0 }),
    { tint: T.tools, right: chip('2 + 1', T.tools) }),
].join('');

const TERMINAL_BODY = [
  sect('Command', textBox('cargo build --target \\\n  aarch64-unknown-linux-gnu', 46) + `<div style="height:11px;"></div>` + rowField('Shell', '/bin/bash', { select: true, mono: true, gap: 0 })),
  sect('Working directory', field('~/projects/tandem', { mono: true, icon: 'folder' })),
  sect('Safety', switchRow('Sandbox filesystem', true, { hint: 'read-only outside the working directory', col: C.err })
    + switchRow('Require approval', true, { hint: 'pauses the graph before every run', col: C.err })
    + `<div style="height:9px;"></div>${label('Allowed commands')}<div style="display:flex;flex-wrap:wrap;gap:5px;">${chip('cargo', C.err)}${chip('rg', C.err)}${chip('git', C.err)}${plusChip}</div>`,
    { tint: C.err, right: chip('elevated', C.err, { dot: true }) }),
  sect('Limits', rowField('Timeout', '90 s') + rowField('Max output', '64 KB') + switchRow('Capture stderr', true)),
  `<div style="padding:12px 16px;display:flex;gap:9px;align-items:flex-start;">
    ${icon('shield', 14, C.err, 1.7)}
    <div style="font-size:10.5px;line-height:1.55;color:${C.low};">This block runs commands on the host machine. Every setting above is a real boundary &mdash; the panel shows them first for a reason.</div>
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

const mainNodes = [
  blockNode({ x: 28, y: 110, w: 178, icon: 'input', color: CAT.data, title: 'Input', state: 'idle',
    body: field('"triage ticket #4192"', { mono: true }),
    ports: [{ kind: 'text', label: 'text', side: 'out', top: 56, dim: true }] }),
  blockNode({ x: 26, y: 418, w: 186, icon: 'terminal', color: CAT.runtimes, title: 'Terminal', state: 'idle',
    body: label('command') + field('cargo build', { mono: true }),
    ports: [{ kind: 'stream', label: 'tool', side: 'out', top: 76 }] }),
  blockNode({ x: 26, y: 558, w: 186, icon: 'python', color: CAT.runtimes, title: 'Python', state: 'idle',
    body: label('source') + field('analyse.py', { mono: true }),
    ports: [{ kind: 'data', label: 'tool', side: 'out', top: 76 }] }),
  blockNode({ x: 250, y: 452, w: 196, icon: 'toolbox', color: CAT.capabilities, title: 'Toolbox', state: 'idle',
    badge: chip('2', T.tools),
    body: `<div style="display:flex;flex-direction:column;gap:5px;">
      <div style="display:flex;align-items:center;gap:7px;height:21px;padding:0 7px;border-radius:5px;background:${C.field};border:1px solid ${C.soft};">${icon('terminal', 11, CAT.runtimes, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${C.mid};">terminal.run</span></div>
      <div style="display:flex;align-items:center;gap:7px;height:21px;padding:0 7px;border-radius:5px;background:${C.field};border:1px solid ${C.soft};">${icon('python', 11, CAT.runtimes, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${C.mid};">python.exec</span></div>
      <div style="font-family:${MONO};font-size:9px;color:${C.faint};padding-left:2px;">exposes 4 functions</div>
    </div>`,
    ports: [
      { kind: 'stream', label: '', side: 'in', top: 54 },
      { kind: 'data', label: '', side: 'in', top: 85 },
      { kind: 'tools', label: 'tools', side: 'out', top: 68, glow: true },
    ] }),
  blockNode({ x: 560, y: 104, w: 236, icon: 'llm', color: CAT.models, title: 'LLM', state: 'idle', selected: true,
    badge: chip('selected', C.accent),
    body: label('model') + field('llama3.2:3b', { mono: true, select: true })
      + `<div style="margin-top:9px;font-family:${MONO};font-size:9.5px;line-height:1.6;color:${C.faint};">You triage build failures. Read<br>the error, run the smallest&#8230;</div>`
      + `<div style="position:absolute;left:-1px;right:-1px;top:97px;height:26px;border-top:1px solid ${rgba(T.tools, 0.35)};border-bottom:1px solid ${rgba(T.tools, 0.35)};background:${rgba(T.tools, 0.07)};"></div>`,
    ports: [
      { kind: 'text', label: 'prompt', side: 'in', top: 56, dim: true },
      { kind: 'text', label: 'context', side: 'in', top: 84, dim: true },
      { kind: 'tools', label: 'tools', side: 'in', top: 112, glow: true },
      { kind: 'text', label: 'text', side: 'out', top: 56, dim: true },
      { kind: 'data', label: 'calls', side: 'out', top: 84, dim: true },
    ] }),
].join('\n');

const mainSvg = [
  wire(206, 162, 560, 160, 'text', { opacity: 0.5 }),
  wire(212, 494, 250, 506, 'stream'),
  wire(212, 634, 250, 537, 'data'),
  wire(446, 520, 560, 216, 'tools', { live: true, dash: '7 6', width: 2.3 }),
  `<circle cx="560" cy="216" r="10" fill="none" stroke="${T.tools}" stroke-width="1.6" opacity=".95"/>`,
  `<circle cx="560" cy="216" r="16" fill="none" stroke="${T.tools}" stroke-width="1" opacity=".35"/>`,
].join('');

const dragCursor = `<div style="position:absolute;left:564px;top:220px;">
  <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" style="filter:drop-shadow(0 2px 4px rgba(0,0,0,.8));"><path d="M5.5 3l12 8.2-5.4 1.1 2.6 5.6-2.4 1.1-2.6-5.6-4.2 3.6z" fill="${C.hi}" stroke="#0b0d11" stroke-width="1.2"/></svg>
  <div style="position:absolute;left:16px;top:16px;display:flex;align-items:center;gap:6px;height:24px;padding:0 9px;border-radius:6px;background:${rgba('#12161c', 0.96)};border:1px solid ${rgba(T.tools, 0.5)};box-shadow:0 8px 20px rgba(0,0,0,.6);white-space:nowrap;">
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

const runNodes = [
  blockNode({ x: 24, y: 110, w: 168, icon: 'input', color: CAT.data, title: 'Input', state: 'ok',
    body: field('ticket #4192', { mono: true }),
    ports: [{ kind: 'text', label: 'text', side: 'out', top: 56 }] }),
  blockNode({ x: 24, y: 344, w: 168, icon: 'terminal', color: CAT.runtimes, title: 'Terminal', state: 'ok',
    badge: chip('42 lines', C.ok),
    body: label('last run') + field('exit 101', { mono: true, suffix: `<span style="font-family:${MONO};font-size:9px;color:${C.err};">err</span>` }),
    ports: [{ kind: 'stream', label: 'tool', side: 'out', top: 76 }] }),
  blockNode({ x: 24, y: 462, w: 168, icon: 'python', color: CAT.runtimes, title: 'Python', state: 'queued',
    body: label('source') + field('analyse.py', { mono: true, muted: true }),
    ports: [{ kind: 'data', label: 'tool', side: 'out', top: 76 }] }),
  blockNode({ x: 236, y: 392, w: 180, icon: 'toolbox', color: CAT.capabilities, title: 'Toolbox', state: 'ok',
    body: `<div style="display:flex;flex-direction:column;gap:5px;">
      <div style="display:flex;align-items:center;gap:7px;height:21px;padding:0 7px;border-radius:5px;background:${C.field};border:1px solid ${C.soft};">${icon('terminal', 11, CAT.runtimes, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${C.mid};">terminal.run</span><span style="flex:1;"></span>${statusDot('ok')}</div>
      <div style="display:flex;align-items:center;gap:7px;height:21px;padding:0 7px;border-radius:5px;background:${C.field};border:1px solid ${C.soft};">${icon('python', 11, CAT.runtimes, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${C.mid};">python.exec</span><span style="flex:1;"></span>${statusDot('queued')}</div>
      <div style="font-family:${MONO};font-size:9px;color:${C.faint};padding-left:2px;">1 call &middot; 1 queued</div>
    </div>`,
    ports: [
      { kind: 'stream', label: '', side: 'in', top: 54 },
      { kind: 'data', label: '', side: 'in', top: 85 },
      { kind: 'tools', label: 'tools', side: 'out', top: 68 },
    ] }),
  blockNode({ x: 470, y: 56, w: 240, icon: 'llm', color: CAT.models, title: 'LLM', state: 'running',
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
      { kind: 'text', label: 'prompt', side: 'in', top: 54 },
      { kind: 'tools', label: 'tools', side: 'in', top: 82 },
      { kind: 'text', label: 'text', side: 'out', top: 54 },
      { kind: 'data', label: 'calls', side: 'out', top: 82 },
    ] }),
  blockNode({ x: 770, y: 96, w: 180, icon: 'output', color: CAT.data, title: 'Report', state: 'queued',
    body: `<div style="font-family:${MONO};font-size:9.5px;line-height:1.65;color:${C.faint};">waiting for llm.text<br>&#8230;</div>`,
    ports: [{ kind: 'text', label: 'text', side: 'in', top: 54 }] }),
].join('\n');

const runSvg = [
  wire(192, 166, 470, 110, 'text', { live: true, dash: '7 7' }),
  wire(192, 420, 236, 446, 'stream', { live: true, dash: '6 6' }),
  wire(192, 538, 236, 477, 'data', { opacity: 0.35 }),
  wire(416, 460, 470, 138, 'tools', { live: true, dash: '7 7', width: 2.2 }),
  wire(710, 110, 770, 150, 'text', { opacity: 0.28, dash: '4 5' }),
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
  tool: { title: 'Toolbox', sub: 'capabilities &middot; tools.bundle', icn: 'toolbox', col: CAT.capabilities, tabs: ['Settings', 'Ports'], tab: 'Settings' },
  input: { title: 'Input', sub: 'data &middot; graph.input', icn: 'input', col: CAT.data, tabs: ['Settings', 'Ports'], tab: 'Settings' },
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
      <span style="width:98px;flex:none;font-size:11.5px;color:${C.hi};">${b.n}</span>
      <span style="flex:1;font-family:${MONO};font-size:10px;color:${C.low};">${b.sig}</span>
      ${typeDots(b.t)}
    </div>`).join('')}
  </div>
</div>`;

const TYPE_DOC = [
  ['text', 'prompts, stdout, any string', 'text &middot; any'],
  ['tools', 'a bundle of callable functions', 'llm.tools'],
  ['data', 'structured json or a record', 'data &middot; any'],
  ['stream', 'output arriving incrementally', 'text &middot; data'],
  ['file', 'a path or blob on disk', 'file &middot; any'],
  ['exec', 'control flow, never a value', 'exec'],
  ['any', 'accepts every type', 'everything'],
];

const LIBRARY_SHEET = doc(sheet({
  w: 1180, h: 1040, kicker: 'Left panel',
  title: 'Six categories, twenty-seven blocks, one type system',
  body: `<div style="display:flex;gap:26px;align-items:flex-start;">
  <div style="width:${LIB_W}px;flex:none;height:748px;border:1px solid ${C.line};border-radius:10px;overflow:hidden;display:flex;">${libraryPanel({ open: ['models', 'capabilities', 'runtimes'] })}</div>
  <div style="flex:1;display:grid;grid-template-columns:repeat(2, minmax(0, 1fr));gap:18px;align-content:start;">
    <div style="display:flex;flex-direction:column;gap:18px;">${catCard(LIB[0])}${catCard(LIB[2])}${catCard(LIB[4])}</div>
    <div style="display:flex;flex-direction:column;gap:18px;">${catCard(LIB[1])}${catCard(LIB[3])}${catCard(LIB[5])}</div>
  </div>
</div>
<div style="background:${C.panel};border:1px solid ${C.line};border-radius:10px;padding:15px 18px;">
  <div style="display:flex;align-items:center;gap:9px;margin-bottom:13px;">
    <span style="font-family:${MONO};font-size:10px;font-weight:700;letter-spacing:.13em;text-transform:uppercase;color:${C.mid};">Port types</span>
    <span style="flex:1;height:1px;background:${C.soft};"></span>
    <span style="font-size:10.5px;color:${C.low};">a wire is legal when its source type is accepted by the target port</span>
  </div>
  <div style="display:grid;grid-template-columns:repeat(7, minmax(0, 1fr));gap:14px;">
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

const anatomyBlock = blockNode({
  x: 170, y: 60, w: 280, icon: 'llm', color: CAT.models, title: 'LLM', state: 'running',
  badge: chip('streaming', C.ok, { dot: true }),
  body: label('model') + field('llama3.2:3b', { mono: true, select: true })
    + `<div style="margin-top:9px;font-family:${MONO};font-size:9.5px;line-height:1.6;color:${C.faint};">The arm64 build fails at link<br>time: ld cannot find -lssl&#8230;</div>`,
  ports: [
    { kind: 'text', label: 'prompt', side: 'in', top: 56 },
    { kind: 'text', label: 'context', side: 'in', top: 84 },
    { kind: 'tools', label: 'tools', side: 'in', top: 112 },
    { kind: 'text', label: 'text', side: 'out', top: 56 },
    { kind: 'data', label: 'calls', side: 'out', top: 84 },
  ],
});

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
  w: 1120, h: 760, kicker: 'Vocabulary',
  title: 'A block, its ports, and the states it moves through',
  body: `<div style="display:flex;gap:28px;align-items:flex-start;">
  <div style="position:relative;width:620px;height:290px;flex:none;">
    <svg xmlns="http://www.w3.org/2000/svg" width="620" height="290" viewBox="0 0 620 290" style="position:absolute;left:0;top:0;">
      ${[[154, 75, 174], [154, 116, 165], [154, 172, 165]].map(([x1, y, x2]) => `<path d="M${x1} ${y}H${x2}" stroke="${C.faint}" stroke-width="1"/>`).join('')}
      ${[[466, 75, 448], [466, 116, 455], [466, 178, 452]].map(([x1, y, x2]) => `<path d="M${x1} ${y}H${x2}" stroke="${C.faint}" stroke-width="1"/>`).join('')}
    </svg>
    ${anatomyBlock}
    ${callout(0, 60, 148, 'Category colour and icon &mdash; the block always looks like its shelf in the library', 'right')}
    ${callout(0, 101, 148, 'Typed input ports', 'right')}
    ${callout(0, 157, 148, 'Port label is the name the graph API uses', 'right')}
    ${callout(472, 62, 148, 'Run status, mirrored in the run panel', 'left')}
    ${callout(472, 103, 148, 'Typed output ports', 'left')}
    ${callout(472, 158, 148, 'Inline preview of the current value &mdash; no need to open the inspector', 'left')}
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
  return `<div onClick="{{pick${o.h}}}" style="position:absolute;left:${o.x}px;top:${o.y}px;width:${o.w}px;background:${C.block};border:1px solid {{bd${o.h}}};border-radius:9px;box-shadow:{{sh${o.h}}};cursor:pointer;">
  <div style="display:flex;align-items:center;gap:8px;height:31px;padding:0 10px;border-bottom:1px solid ${C.soft};border-radius:8px 8px 0 0;background:linear-gradient(180deg,${rgba(c, 0.13)},${rgba(c, 0.02)});">
    ${icon(o.icon, 13, c, 1.7)}
    <span style="font-size:12px;font-weight:600;color:${C.hi};">${o.title}</span>
    <span style="flex:1;"></span>
    <span style="width:7px;height:7px;border-radius:50%;background:{{dot${o.h}}};box-shadow:0 0 0 3px {{ring${o.h}}};flex:none;"></span>
  </div>
  <div style="position:relative;padding:10px 11px 12px;">${o.body}</div>
  ${(o.ports || []).map(port).join('')}
</div>`;
}

const iNodes = [
  iblock({ h: 'In', x: 40, y: 120, w: 178, icon: 'input', color: CAT.data, title: 'Input',
    body: field('"triage ticket #4192"', { mono: true }),
    ports: [{ kind: 'text', label: 'text', side: 'out', top: 56 }] }),
  iblock({ h: 'Term', x: 40, y: 430, w: 186, icon: 'terminal', color: CAT.runtimes, title: 'Terminal',
    body: label('command') + field('cargo build', { mono: true }),
    ports: [{ kind: 'stream', label: 'tool', side: 'out', top: 76 }] }),
  iblock({ h: 'Tool', x: 300, y: 470, w: 196, icon: 'toolbox', color: CAT.capabilities, title: 'Toolbox',
    body: `<div style="display:flex;flex-direction:column;gap:5px;">
      <div style="display:flex;align-items:center;gap:7px;height:21px;padding:0 7px;border-radius:5px;background:${C.field};border:1px solid ${C.soft};">${icon('terminal', 11, CAT.runtimes, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${C.mid};">terminal.run</span></div>
      <div style="display:flex;align-items:center;gap:7px;height:21px;padding:0 7px;border-radius:5px;background:${C.field};border:1px solid ${C.soft};">${icon('python', 11, CAT.runtimes, 1.7)}<span style="font-family:${MONO};font-size:9.5px;color:${C.mid};">python.exec</span></div>
    </div>`,
    ports: [
      { kind: 'stream', label: '', side: 'in', top: 54 },
      { kind: 'tools', label: 'tools', side: 'out', top: 68 },
    ] }),
  iblock({ h: 'Llm', x: 560, y: 110, w: 236, icon: 'llm', color: CAT.models, title: 'LLM',
    body: label('model') + field('llama3.2:3b', { mono: true, select: true })
      + `<div style="margin-top:9px;font-family:${MONO};font-size:9.5px;line-height:1.6;color:${C.faint};">You triage build failures. Read<br>the error, run the smallest&#8230;</div>`,
    ports: [
      { kind: 'text', label: 'prompt', side: 'in', top: 56 },
      { kind: 'text', label: 'context', side: 'in', top: 84 },
      { kind: 'tools', label: 'tools', side: 'in', top: 112 },
      { kind: 'text', label: 'text', side: 'out', top: 56 },
      { kind: 'data', label: 'calls', side: 'out', top: 84 },
    ] }),
].join('\n');

const iWires = (live) => [
  wire(218, 176, 560, 166, 'text', live ? { live: true, dash: '7 7' } : {}),
  wire(226, 506, 300, 524, 'stream', live ? { live: true, dash: '6 6' } : {}),
  wire(496, 538, 560, 222, 'tools', live ? { live: true, dash: '7 7', width: 2.2 } : { width: 2.1 }),
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

/* =================================================================== write */

const files = {
  'EmptyShell.dc.html': EMPTY,
  'Main.dc.html': MAIN,
  'Running.dc.html': RUNNING,
  'Inspector.dc.html': INSPECTOR_SHEET,
  'Library.dc.html': LIBRARY_SHEET,
  'BlockAnatomy.dc.html': ANATOMY,
  'Interactive.dc.html': INTERACTIVE,
};

const canvas = {
  artboards: [
    { file: 'EmptyShell.dc.html', x: 0, y: 0, w: 1560, h: 900, page: 'page-1', title: 'Empty shell' },
    { file: 'Main.dc.html', x: 1680, y: 0, w: 1560, h: 900, page: 'page-1', title: 'Wiring a block in' },
    { file: 'Running.dc.html', x: 3360, y: 0, w: 1560, h: 900, page: 'page-1', title: 'Graph running' },
    { file: 'Inspector.dc.html', x: 0, y: 1080, w: 1800, h: 980, page: 'page-1', title: 'Inspector states' },
    { file: 'Library.dc.html', x: 1920, y: 1080, w: 1180, h: 1040, page: 'page-1', title: 'Block library' },
    { file: 'BlockAnatomy.dc.html', x: 3200, y: 1080, w: 1120, h: 760, page: 'page-1', title: 'Block anatomy' },
    { file: 'Interactive.dc.html', x: 0, y: 0, w: 1560, h: 900, page: 'page-2', title: 'Clickable shell', is_interactive: true },
  ],
  annotations: [
    { id: 'brief', x: 0, y: -186, w: 700, page: 'page-1',
      text: 'Block Canvas - a shell for wiring blocks into runnable graphs.\n\nLeft: the library, categorised by what a block does. Centre: an infinite node canvas. Right: an inspector that reshapes itself around whatever is selected.\n\nTop row is the same shell at three moments: nothing built, mid-wire, mid-run.' },
    { id: 'panel-note', x: 0, y: 940, w: 560, page: 'page-1',
      text: 'The right panel is the whole idea: five different selections, five different panels, one 328px column.' },
    { id: 'types-note', x: 1920, y: 940, w: 520, page: 'page-1',
      text: 'Every block declares typed ports. A wire is coloured by its data type, and the type is what makes a connection legal or refuses it mid-drag.' },
    { id: 'anatomy-note', x: 3200, y: 940, w: 480, page: 'page-1',
      text: 'One block, labelled, plus every run state - so the vocabulary reads the same on all three shells.' },
    { id: 'proto-note', x: 0, y: -150, w: 620, page: 'page-2',
      text: 'Click any block to watch the inspector swap. Press Run to put the graph in flight - wires animate, status dots turn green, the console reports.' },
  ],
  pages: [
    { id: 'page-1', name: 'Screens' },
    { id: 'page-2', name: 'Clickable' },
  ],
  launch: { view: 'canvas', page: 'page-1' },
};

for (const [name, html] of Object.entries(files)) writeFileSync(name, html);
writeFileSync('canvas.json', JSON.stringify(canvas, null, 2));
console.log('wrote', Object.keys(files).length, 'artboards + canvas.json');
