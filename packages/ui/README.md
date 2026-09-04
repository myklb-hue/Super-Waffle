# @cyberloom/ui

The primitive set: the components used by two or more screens or panels.
Nothing here knows about graphs, blocks or the engine.

```
npm run ui        # from the repository root, opens the story site
```

## What is in here

- `src/styles/tokens.css` — the single source of truth for colour, type,
  spacing, radii, elevation and geometry. No component writes a colour
  literal; everything resolves through a token. The geometry tokens are
  load-bearing: `--block-header-h`, `--port-first` and `--port-row` are what
  the wire router will read.
- `src/styles/fonts.css` and `fonts/` — Space Grotesk and JetBrains Mono as
  committed WOFF2, latin and latin-ext only. The shell never fetches a font,
  so it renders correctly with no network (SPEC §15.13). Both are OFL 1.1;
  see `fonts/OFL.txt`. Regenerate with `node scripts/fetch-fonts.mjs`.
- `src/styles/globals.css` — the reset. It resets; it does not style.
- `src/components/` — one file per component, plus `icons.ts`, which is
  **generated** from `design/cyberloom/icons.mjs` by
  `node scripts/gen-icons.mjs`. Edit the design table and regenerate, so the
  mockups and the application can never show different glyphs.
- `src/*.stories.tsx` — the story site, grouped by where a component is used:
  display, inputs, panels, blocks. Every state the plan lists has a case.

## Two conventions worth knowing

**One stylesheet, not twenty-five.** `docs/PLAN.md` §5 originally said each
component gets a colocated `.module.css`. These are atoms that share the same
dozen tokens, so they share `ui.module.css` instead; CSS Modules already scopes
the names, and splitting it would have added files without adding isolation.
Components with real internal layout — the canvas nodes, the inspector panels —
do get their own module beside them.

**Colour is passed as a token name, never a value.** A component takes
`color="cat-senses"` or `color="type-audio"`, and resolves it to
`var(--cat-senses)`. That is why the palette can change in one file, and why
`grep` for a hex outside `tokens.css` should return nothing.

## Verifying

```
npm run typecheck          # from the root
npm run build -w @cyberloom/ui
```
