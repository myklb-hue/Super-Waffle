// The four reference rigs, generated from the drawings in the design mockups.
//
// `design/cyberloom/build.mjs` draws every rig and expression for Figure 15,
// and that figure is the visual specification. This is the same drawing code,
// carried over so the shipped rigs cannot drift from the mockups again: run
// `node scripts/gen-rigs.mjs` and `rigs/*/states/*.svg` are rewritten.
//
// Two things are added on the way. Every state is split into `#eyes` and
// `#mouth` groups, which is the one convention the shell's animation relies on
// (blink and gaze move the eyes, the envelope moves the mouth); and `sleepy`,
// which the specification lists for Line, Robot and Orb and the mockup grid
// does not draw, is added in each rig's own idiom.
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));

// The mockups' palette (build.mjs: C, T, CAT).
const C = { block: '#191d24', field: '#0b0d11', soft: '#1a1e25', hi: '#e8ebf0', mid: '#98a2ae', accent: '#56c7d6' };
const T = { tools: '#e0a458', data: '#a78bd0' };
const ROSE = '#d97f8f';
const INK = '#0b0d11';

const heart = (cx, cy, r, fill) =>
  `<path d="M${cx} ${cy + r * 0.95} C${cx - r * 1.5} ${cy - r * 0.05}, ${cx - r * 1.05} ${cy - r * 1.25}, ${cx} ${cy - r * 0.45} C${cx + r * 1.05} ${cy - r * 1.25}, ${cx + r * 1.5} ${cy - r * 0.05}, ${cx} ${cy + r * 0.95}Z" fill="${fill}"/>`;

const EXPRESSIONS = ['neutral', 'smile', 'frown', 'surprised', 'thinking', 'speaking', 'love', 'sleepy'];

/** { base, eyes, mouth } for one rig and expression, in a 64-unit box. */
function draw(rig, expr) {
  const W = C.hi, cy = C.accent, am = T.tools, vi = T.data;
  if (rig === 'line') {
    const eyes = {
      neutral: `<circle cx="22" cy="26" r="2.6" fill="${W}"/><circle cx="42" cy="26" r="2.6" fill="${W}"/>`,
      smile: `<path d="M17 27q5-6 10 0"/><path d="M37 27q5-6 10 0"/>`,
      frown: `<circle cx="22" cy="27" r="2.6" fill="${W}"/><circle cx="42" cy="27" r="2.6" fill="${W}"/><path d="M16 19l9 3"/><path d="M48 19l-9 3"/>`,
      surprised: `<circle cx="22" cy="25" r="4.2"/><circle cx="42" cy="25" r="4.2"/>`,
      thinking: `<circle cx="22" cy="26" r="2.6" fill="${W}"/><circle cx="43" cy="22" r="2.6" fill="${W}"/><path d="M37 16q5-3 10 0"/>`,
      speaking: `<circle cx="22" cy="26" r="2.6" fill="${W}"/><circle cx="42" cy="26" r="2.6" fill="${W}"/>`,
      love: heart(22, 26, 5, ROSE) + heart(42, 26, 5, ROSE),
      // Lids down: the same arcs as the smile, the other way up.
      sleepy: `<path d="M17 25q5 5 10 0"/><path d="M37 25q5 5 10 0"/>`,
    }[expr];
    const mouth = {
      neutral: `<path d="M25 43h14"/>`,
      smile: `<path d="M21 40q11 11 22 0"/>`,
      frown: `<path d="M21 46q11-9 22 0"/>`,
      surprised: `<ellipse cx="32" cy="44" rx="4.5" ry="6.5"/>`,
      thinking: `<path d="M25 44q5-4 10 0"/><circle cx="43" cy="45" r="1.4" fill="${W}"/><circle cx="48" cy="45" r="1.4" fill="${W}"/>`,
      speaking: `<ellipse cx="32" cy="43" rx="7" ry="4"/>`,
      love: `<path d="M21 40q11 11 22 0"/>`,
      sleepy: `<path d="M27 44h10"/>`,
    }[expr];
    const stroke = `fill="none" stroke="${W}" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"`;
    return { base: '', eyes, mouth, eyesAttrs: stroke, mouthAttrs: stroke };
  }
  if (rig === 'robot') {
    const eye = (x, h) => `<rect x="${x}" y="${28 - h / 2}" width="11" height="${h}" rx="2" fill="${cy}"/>`;
    const eyes = {
      neutral: eye(17, 6) + eye(36, 6),
      smile: `<path d="M17 30q5.5-7 11 0" fill="none" stroke="${cy}" stroke-width="3.5" stroke-linecap="round"/><path d="M36 30q5.5-7 11 0" fill="none" stroke="${cy}" stroke-width="3.5" stroke-linecap="round"/>`,
      frown: `<rect x="17" y="24" width="11" height="6" rx="2" fill="${cy}" transform="rotate(12 22.5 27)"/><rect x="36" y="24" width="11" height="6" rx="2" fill="${cy}" transform="rotate(-12 41.5 27)"/>`,
      surprised: eye(17, 11) + eye(36, 11),
      thinking: eye(17, 6) + `<rect x="36" y="22" width="11" height="4" rx="2" fill="${cy}"/>`,
      speaking: eye(17, 6) + eye(36, 6),
      love: heart(22.5, 28, 5.5, ROSE) + heart(41.5, 28, 5.5, ROSE),
      sleepy: eye(17, 2) + eye(36, 2),
    }[expr];
    const bars = {
      neutral: [[0, 3], [0, 3], [0, 3], [0, 3], [0, 3]],
      smile: [[2, 3], [0.5, 3], [-1.5, 3], [0.5, 3], [2, 3]],
      frown: [[-2.5, 3], [-0.5, 3], [1.5, 3], [-0.5, 3], [-2.5, 3]],
      surprised: null,
      thinking: [[0, 3], [0, 3], [0, 3], null, null],
      speaking: [[0, 3], [0, 7], [0, 5], [0, 8], [0, 4]],
      love: [[2, 3], [0.5, 3], [-1.5, 3], [0.5, 3], [2, 3]],
      sleepy: [[0, 2], [0, 2], [0, 2], [0, 2], [0, 2]],
    }[expr];
    const mouth = bars === null
      ? `<rect x="28" y="38" width="8" height="8" rx="2" fill="${cy}"/>`
      : bars.map((b, i) => (b ? `<rect x="${20 + i * 5.2}" y="${42 - b[0] - b[1] / 2}" width="3.6" height="${b[1]}" rx="1" fill="${cy}"/>` : '')).join('');
    const base = `<rect x="10" y="12" width="44" height="42" rx="9" fill="${C.block}" stroke="${C.mid}" stroke-width="2"/><path d="M32 12V6" stroke="${C.mid}" stroke-width="2" stroke-linecap="round"/><circle cx="32" cy="5" r="2" fill="${C.mid}"/>`;
    return { base, eyes, mouth, eyesAttrs: '', mouthAttrs: '' };
  }
  if (rig === 'orb') {
    const col = { neutral: cy, smile: am, frown: '#4e6392', surprised: W, thinking: vi, speaking: cy, love: ROSE, sleepy: '#4e5566' }[expr];
    const r = expr === 'surprised' ? 23 : 20;
    const eyes = {
      neutral: `<circle cx="26" cy="30" r="2.2" fill="${INK}"/><circle cx="38" cy="30" r="2.2" fill="${INK}"/>`,
      smile: `<path d="M22 31q4-5 8 0M34 31q4-5 8 0" fill="none" stroke="${INK}" stroke-width="2.4" stroke-linecap="round"/>`,
      frown: `<path d="M22 28l7 3M42 28l-7 3" fill="none" stroke="${W}" stroke-width="2.2" stroke-linecap="round"/>`,
      surprised: `<circle cx="25" cy="29" r="3.4" fill="${INK}"/><circle cx="39" cy="29" r="3.4" fill="${INK}"/>`,
      thinking: `<circle cx="26" cy="31" r="2.2" fill="${INK}"/><circle cx="39" cy="27" r="2.2" fill="${INK}"/><circle cx="50" cy="16" r="2.4" fill="${vi}"/>`,
      speaking: `<circle cx="26" cy="30" r="2.2" fill="${INK}"/><circle cx="38" cy="30" r="2.2" fill="${INK}"/>`,
      love: heart(26, 30, 3.2, INK) + heart(38, 30, 3.2, INK) + heart(51, 13, 3.4, ROSE),
      sleepy: `<path d="M22 29q4 5 8 0M34 29q4 5 8 0" fill="none" stroke="${INK}" stroke-width="2.4" stroke-linecap="round"/>`,
    }[expr];
    // An orb has no mouth. Speaking is the rings, and they live in the mouth
    // group so that the envelope can bring them in while the orb talks.
    const mouth = expr === 'speaking'
      ? `<circle cx="32" cy="32" r="26" fill="none" stroke="${cy}" stroke-width="1.2" opacity=".55"/><circle cx="32" cy="32" r="30" fill="none" stroke="${cy}" stroke-width="1" opacity=".25"/>`
      : '';
    const base = `<circle cx="32" cy="32" r="${r + 6}" fill="${col}" opacity=".16"/><circle cx="32" cy="32" r="${r}" fill="${col}"/><circle cx="26" cy="24" r="6" fill="#ffffff" opacity=".16"/>`;
    return { base, eyes, mouth, eyesAttrs: '', mouthAttrs: '' };
  }
  if (rig === 'pixel') {
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
    if (!cells) return null;
    const lit = expr === 'love' ? ROSE : T.tools;
    const cell = (x, y, fill) => `<rect x="${8 + x * 6}" y="${8 + y * 6}" width="5" height="5" rx="1" fill="${fill}"/>`;
    let grid = '';
    for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) grid += cell(x, y, C.soft);
    // The top half of the matrix is the eyes, the bottom the mouth; the heart
    // is one picture and goes with the eyes.
    const eyes = cells.filter(([, y]) => expr === 'love' || y <= 3).map(([x, y]) => cell(x, y, lit)).join('');
    const mouth = cells.filter(([, y]) => expr !== 'love' && y > 3).map(([x, y]) => cell(x, y, lit)).join('');
    const base = `<rect x="4" y="4" width="56" height="56" rx="4" fill="${INK}"/>${grid}`;
    return { base, eyes, mouth, eyesAttrs: '', mouthAttrs: '' };
  }
  return null;
}

for (const rig of ['line', 'robot', 'orb', 'pixel']) {
  const dir = `${root}rigs/${rig}/states`;
  mkdirSync(dir, { recursive: true });
  for (const expr of EXPRESSIONS) {
    const parts = draw(rig, expr);
    if (!parts || parts.eyes === undefined) continue;
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="240" height="240">
  <rect width="64" height="64" fill="none"/>
  ${parts.base}
  <g id="eyes"${parts.eyesAttrs ? ' ' + parts.eyesAttrs : ''}>${parts.eyes}</g>
  <g id="mouth"${parts.mouthAttrs ? ' ' + parts.mouthAttrs : ''}>${parts.mouth}</g>
</svg>
`;
    writeFileSync(`${dir}/${expr}.svg`, svg);
    process.stdout.write(`${rig}/${expr} `);
  }
}
console.log('\nrigs written');
