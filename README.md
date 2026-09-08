# @aihu/css-engine

> **Aihu** — agentic discovery and interaction, for human purpose.

aihu CSS engine — Tailwind v4 hard fork with WC-native scoped output.

Part of the **compiler + toolchain** layer of Aihu. Build-time only — does not ship to the client. The compiler reads `.aihu` SFC source and emits standards-compliant Web Components.

<!-- BEGIN_HANDWRITTEN: prose -->
> aihu CSS engine — a hard fork of Tailwind v4 with Web-Component-native scoped output, AST-aware scanning, and progressive feature emission.

**Status:** v1 — shipped. The fork identity, AST scanner, scoped emitter, WC-native
variants, progressive features, both style packs, and the `cn()` runtime helper
are all landed. See [`docs/integration-seams.md`](docs/integration-seams.md) for
the boundary between this standalone engine and the framework repository.

### Status by capability

| Capability | Status |
|---|---|
| Package builds; `compile()` / `compileSfc()` pipeline | ✅ shipped |
| AST scanner consuming `@aihu/compiler` (`compileSfc`) | ✅ shipped |
| Scoped-output mode (`:host` embedding) | ✅ shipped |
| WC-native variants (`host:`, `slotted:`, `part-*:`) | ✅ shipped |
| Progressive features (`view-transition:`, `anchor:`, etc.) — `@aihu/css-engine/runtime/progressive` | ✅ shipped |
| Style packs (`aihu-default`, `aihu-graphite`) — `defineStylePack()`, `./packs`, `./styles/*.css` | ✅ shipped |
| `cn()` runtime helper — `@aihu/css-engine/runtime/cn` | ✅ shipped |

### Using the compiler provider contract

The engine can be installed as an explicit provider, which keeps the compiler
independent of any particular CSS implementation while preserving the direct
`compileSfc()` API:

```ts
import { aihuCompilerPlugin } from '@aihu/compiler'
import { aihuCssProvider } from '@aihu/css-engine'

export default {
  plugins: [aihuCompilerPlugin({ cssProvider: aihuCssProvider })],
}
```

`aihuCssProvider` forwards the compiler's resolved light-DOM scope to the CSS
engine. Use `createAihuCssProvider(compileSfcCompatible)` when wrapping a
compatible engine implementation or test double.

### Native binary distribution

`compile()` / `compileSfc()` shell out to the prebuilt `aihu-css-compile`
executable (the `aihu-css-core` Rust crate). For npm consumers the binary ships
as a per-platform `optionalDependencies` package
(`@aihu/css-engine-{darwin-arm64,darwin-x64,linux-x64-gnu,win32-x64-msvc}`),
resolved automatically at build time — no Rust toolchain required. In a source
clone the engine falls back to the checkout's `target/release` binary.

Release tags use `css-v<host-version>`. The host package and native packages
have separate version streams (`0.6.x` and `0.1.x` respectively), but all four
native packages move together. CI verifies the exact pins, builds each native
binary on its target runner, publishes new native versions first, and publishes
the host package only after those jobs succeed. A Rust change therefore needs a
native version bump in all four `npm/*/package.json` files plus the matching
host `optionalDependencies` pins.

### Enabling utility-class CSS in a user project

Utility-class CSS is **auto-wired by presence** — install `@aihu/css-engine`
alongside `@aihu/app`, set one config knob, and every `.aihu` file in your
project is scanned and the matched rules land in your bundle's CSS asset.
There is no separate `viteCssEnginePlugin`, no `aihu.config.ts` file, no
preset selection step. The entire user surface is the `css` option on
`viteAihuPlugin()` in `vite.config.ts`:

```ts
// vite.config.ts — the entire surface
import { viteAihuPlugin } from '@aihu/app'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    viteAihuPlugin({
      css: { shadowMode: 'light' },   // ← styles this component + external light-DOM children
    }),
  ],
})
```

```bash
bun add @aihu/css-engine
```

```jsx
// any .aihu file — classes are scanned automatically
@template {
  <section class="flex gap-6 px-4 py-6">
    <span class="text-lg font-semibold">hi</span>
  </section>
}
```

That's it. With the default `shadowMode: 'shadow'`, the scoped rules
(`.flex{display:flex}`, `.gap-6{gap:1.5rem}`, etc.) fold into each component's
shadow `<style>`. With `shadowMode: 'light'` (the example above) they instead
land in `dist/assets/index-*.css` after `bun run build`.

#### When do you need `shadowMode: 'light'`?

Scoped utility classes work fine behind a shadow root (`'shadow'`, the leaf default):
`@aihu/css-engine` compiles each SFC's classes to a per-component stylesheet and
folds it into that component's shadow `<style>`. It is scoped by design and does
**not** rely on the global cascade.

Use `'light'` only when you want the utility CSS in the light DOM — for example to
style external / slotted child elements that live outside your component's shadow
root, or to emit a single global sheet. (Truly global frameworks like Tailwind,
UnoCSS, or Pico do require `'light'`, but `@aihu/css-engine` does not.)

#### Style packs vs the utility scanner — they are separate

Two distinct things ship in `@aihu/css-engine`:

| Concern | What you do | Output |
|---|---|---|
| **Utility scanner** (this section) | Install the package (works in either shadow mode; `'shadow'` folds into the shadow style) | Per-component scoped rules (or the Vite CSS bundle under `'light'`) |
| **Theme packs** (color tokens, fonts) | `import "@aihu/css-engine/styles/aihu-graphite.css"` in your entry | `--color-*` / `--font-*` CSS custom properties at `:root` |

Theme packs are plain CSS imports — they emit token variables, not utility
rules. Importing a pack alone does **not** enable the scanner; setting
`css.shadowMode: 'light'` alone does **not** import a pack. Use both for a
complete look; either independently is fine.

### Utility vocabulary

The engine is **inspired by Tailwind v4, not a fork** — the supported set is
hand-curated in [`crates/aihu-css-core/src/tokens.rs`](crates/aihu-css-core/src/tokens.rs).

**Fixed long-tail utilities** (literal class names):

- **Display** — `block`, `inline-block`, `inline`, `flex`, `inline-flex`, `grid`, `inline-grid`, `hidden`, `contents`
- **Flexbox** — `flex-row`, `flex-col`, `flex-wrap`, `flex-nowrap`, `flex-1`, `flex-auto`, `flex-none`
- **Alignment** — `items-{start,center,end,stretch,baseline}`, `justify-{start,center,end,between,around,evenly}`
- **Position** — `static`, `relative`, `absolute`, `fixed`, `sticky`
- **Overflow** — `overflow-{hidden,auto,scroll,visible}`
- **Typography** — `text-{left,center,right,justify}`, `italic`, `not-italic`, `underline`, `line-through`, `no-underline`, `uppercase`, `lowercase`, `capitalize`, `truncate`
- **Font weight** — `font-{thin,normal,medium,semibold,bold,black}`
- **Borders / radius** — `border`, `rounded`, `rounded-{none,sm,md,lg,xl,2xl,full}`
- **Shadows** — `shadow`, `shadow-{sm,md,lg,none}`
- **Sizing keywords** — `w-{full,screen,auto}`, `h-{full,screen,auto}`

**Parameterized utilities** (`<prefix>-<value>`):

| Prefix family | Examples | Notes |
|---|---|---|
| Spacing — `p`, `px`, `py`, `pt`, `pr`, `pb`, `pl`, `m`, `mx`, `my`, `mt`, `mr`, `mb`, `ml`, `gap`, `gap-x`, `gap-y` | `p-4`, `mx-auto`, `gap-6` | value × `0.25rem` |
| Sizing — `w`, `h`, `min-w`, `max-w`, `min-h`, `max-h` | `w-64`, `max-w-7xl` | spacing scale or fractions |
| Font size — `text-{xs,sm,base,lg,xl,2xl,3xl,…}` | `text-lg` | includes paired line-height |
| z-index — `z-{N}` | `z-10`, `z-50` | numeric only |
| Opacity — `opacity-{N}` | `opacity-75` | 0–100 |
| Palette colors — `bg`, `text`, `border`, `fill`, `stroke`, `ring`, `outline` followed by a named keyword | `bg-red-500`, `text-slate-700` | |

**Arbitrary values** (`<prefix>-[<value>]` — emitted verbatim into the mapped
property): `bg-[#1a1d24]`, `w-[34ch]`, `text-[14px]`, `px-[2rem]`, etc.
Supported prefixes: `bg`, `text`, `w`, `h`, `min-w`, `max-w`, `min-h`, `max-h`,
`p`, `px`, `py`, `m`, `mx`, `my`, `gap`, `rounded`, `border`, `leading`,
`tracking`, `z`, `top`/`right`/`bottom`/`left`/`inset`, `fill`, `stroke`,
`shadow`. Underscores in the bracket become spaces (Tailwind convention).

**Brand color utilities** — `bg-primary`, `text-accent`, `border-muted`, etc.
resolve to `var(--color-*)` registered by a `@theme` block or a theme pack.

#### Not supported (common misses — call out to avoid surprises)

The vocabulary is intentionally narrower than full Tailwind. The following are
**not** in the table; use them and you'll get the class string in your HTML
with **no matching CSS rule** (the scanner returns `None` and emits nothing):

- `grid-cols-N` / `grid-rows-N` / `col-span-N` / `row-span-N` — **no grid template utilities**. Use `display: grid` via the `grid` class, then write a hand-authored `@style` block or an arbitrary `style="grid-template-columns:…"` attribute.
- `mx-auto` / `my-auto` — the spacing scale is `value × 0.25rem` and doesn't include the `auto` keyword. Use `style="margin-inline: auto"` or an arbitrary value (`mx-[auto]`) instead.
- `max-w-Nxl` / `max-w-screen` / etc. — `max-w` parameterized values follow the spacing/sizing scale, not Tailwind's named breakpoint scale. `max-w-[1280px]` (arbitrary value) works; `max-w-7xl` does not.
- Tailwind v4 colors *outside* the named keyword set (e.g. `text-zinc-200` — depends on whether `zinc` is in `named_keyword_color`)
- Responsive prefixes (`sm:`, `md:`, `lg:`, `xl:`, `2xl:`) — **not yet handled** by the scanner
- State variants outside the WC-native set (`host:`, `slotted:`, `part-*:` are supported per the variants section above; arbitrary `hover:`, `focus:`, `dark:` etc. depend on what the current `parse_variant` accepts)
- Container queries, `@layer` ordering, JIT-style arbitrary properties (`[mask-image:url(...)]`)

**The authoritative source is [`crates/aihu-css-core/src/tokens.rs`](crates/aihu-css-core/src/tokens.rs)** — the lists above are a summary of that file's match arms and may drift; when in doubt, grep that file or write a one-liner test against `utility_to_css(class_name)`.

#### Production caveat — native binary must resolve

The scanner runs through the prebuilt `aihu-css-compile` Rust binary. For npm
consumers the binary ships as a per-platform `optionalDependencies` package
(e.g. `@aihu/css-engine-darwin-arm64`) and is auto-resolved by your package
manager. If resolution fails (offline install, unsupported platform), the
compiler emits a one-shot console warning, the build succeeds, and utility
rules are silently omitted from the bundle CSS. In source clones the fallback
is `target/release/aihu-css-compile` (run `cargo build --release`).

### Local development

```bash
# Build Rust core
cargo build --release

# Build TS layer
bun run build

# Run tests
bun run test         # vitest e2e
bun run test:rust    # cargo + insta snapshots
```
<!-- END_HANDWRITTEN: prose -->

## Install

<!-- BEGIN_AUTOGEN: install -->
<!-- regenerate: bun scripts/sync-readme.ts (also runs in pre-commit + CI) -->

```bash
npm install @aihu/css-engine
# or
bun add @aihu/css-engine
```

<sub><i>Auto-generated against `@aihu/css-engine@0.6.1`.</i></sub>

<!-- END_AUTOGEN: install -->

## Package facts

<!-- BEGIN_AUTOGEN: stats -->
<!-- regenerate: bun scripts/sync-readme.ts (also runs in pre-commit + CI) -->

| | |
|---|---|
| **Version** | `0.6.1` |
| **Tier** | D — Compiler — CSS engine (Tailwind v4 hard fork, WC-native scoped output) |
| **Published files** | 5 entries |
| **License** | MIT |

<sub><i>Auto-generated against `@aihu/css-engine@0.6.1`.</i></sub>

<!-- END_AUTOGEN: stats -->

## Exports

<!-- BEGIN_AUTOGEN: exports -->
<!-- regenerate: bun scripts/sync-readme.ts (also runs in pre-commit + CI) -->

| Subpath | ESM | CJS |
|---|---|---|
| `.` | `./dist/index.js` | `—` |
| `./packs` | `./dist/packs.js` | `—` |
| `./styles/aihu-default.css` | `./styles/aihu-default.css` | — |
| `./styles/aihu-graphite.css` | `./styles/aihu-graphite.css` | — |
| `./styles/*` | `./styles/*` | — |
| `./runtime/cn` | `./dist/runtime/cn.js` | `—` |
| `./runtime/progressive` | `./dist/runtime/progressive.js` | `—` |

<sub><i>Auto-generated against `@aihu/css-engine@0.6.1`.</i></sub>

<!-- END_AUTOGEN: exports -->

## Dependencies

<!-- BEGIN_AUTOGEN: deps -->
<!-- regenerate: bun scripts/sync-readme.ts (also runs in pre-commit + CI) -->

**Dependencies:**

- `@aihu/arbor` — `^4.1.2`
- `@aihu/compiler` — `^1.3.2`

**Optional dependencies (platform-specific):**

- `@aihu/css-engine-darwin-arm64` — `0.1.17`
- `@aihu/css-engine-darwin-x64` — `0.1.17`
- `@aihu/css-engine-linux-x64-gnu` — `0.1.17`
- `@aihu/css-engine-win32-x64-msvc` — `0.1.17`

<sub><i>Auto-generated against `@aihu/css-engine@0.6.1`.</i></sub>

<!-- END_AUTOGEN: deps -->

## See also

<!-- BEGIN_AUTOGEN: see-also -->
<!-- regenerate: bun scripts/sync-readme.ts (also runs in pre-commit + CI) -->

- [Integration seams](docs/integration-seams.md)
- [@aihu/compiler](https://www.npmjs.com/package/@aihu/compiler)
- [Aihu CSS Engine repository](https://github.com/aihu-project/aihu-css)

<sub><i>Auto-generated against `@aihu/css-engine@0.6.1`.</i></sub>

<!-- END_AUTOGEN: see-also -->

## License

<!-- BEGIN_AUTOGEN: license -->
<!-- regenerate: bun scripts/sync-readme.ts (also runs in pre-commit + CI) -->

MIT — see [LICENSE](LICENSE).

<sub><i>Auto-generated against `@aihu/css-engine@0.6.1`.</i></sub>

<!-- END_AUTOGEN: license -->
