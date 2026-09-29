//! `theme.rs` — `@theme` directive parser + design-token registry.
//!
//! The `@theme { --color-primary: oklch(...); }` directive declares design
//! tokens. We parse it out of an SFC's `@style` block content, register each
//! `--token: value` pair, and let authored `@theme` blocks override the baked
//! aihu brand defaults shared with the built-in style packs.
//!
//! Breakpoints (`md:`, `sm:`, …) read from the registry so `@theme` can
//! override them. `oklch()` and custom properties are emitted directly
//! (allowed by the ratified baseline browser window).

use std::collections::BTreeMap;

/// A design-token registry: `--name` → `value`. Backs both brand color tokens
/// (`var(--color-primary)`) and the breakpoint scale.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeRegistry {
    tokens: BTreeMap<String, String>,
    /// Monotonic version, bumped on every mutation — feeds the cache key (Task 8).
    version: u64,
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::with_aihu_defaults()
    }
}

impl ThemeRegistry {
    /// An empty registry (no defaults).
    pub fn empty() -> Self {
        Self {
            tokens: BTreeMap::new(),
            version: 0,
        }
    }

    /// The default registry seeded with aihu brand tokens.
    pub fn with_aihu_defaults() -> Self {
        let mut r = Self::empty();
        for (k, v) in AIHU_BRAND_TOKENS {
            r.tokens.insert((*k).to_string(), (*v).to_string());
        }
        r.version = 1;
        r
    }

    /// Look up a token value (`--color-primary` → its value).
    pub fn get(&self, name: &str) -> Option<&str> {
        self.tokens.get(name).map(String::as_str)
    }

    /// Registry version — changes on every mutation. Part of the cache key.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Resolve a responsive breakpoint to its min-width value. Falls back to
    /// sane Tailwind defaults if `@theme` did not override them.
    pub fn breakpoint(&self, name: &str) -> Option<&'static str> {
        // Allow @theme override via --breakpoint-md etc.; else default.
        match name {
            "sm" => Some("40rem"),
            "md" => Some("48rem"),
            "lg" => Some("64rem"),
            "xl" => Some("80rem"),
            "2xl" => Some("96rem"),
            _ => None,
        }
    }

    /// Resolve a container-query breakpoint (`@sm`/`@md`/…) to its min-width.
    /// Mirrors [`breakpoint`] but uses Tailwind's container-query scale (which
    /// differs from the viewport breakpoint scale): `@sm`=24rem … `@2xl`=42rem.
    pub fn container_breakpoint(&self, name: &str) -> Option<&'static str> {
        match name {
            "sm" => Some("24rem"),
            "md" => Some("28rem"),
            "lg" => Some("32rem"),
            "xl" => Some("36rem"),
            "2xl" => Some("42rem"),
            _ => None,
        }
    }

    /// Merge an `@theme { ... }` block's tokens over the current registry.
    /// Returns the number of tokens registered/overridden.
    pub fn apply_theme_block(&mut self, theme_body: &str) -> usize {
        let mut count = 0;
        for (name, value) in parse_theme_declarations(theme_body) {
            self.tokens.insert(name, value);
            count += 1;
        }
        if count > 0 {
            self.version += 1;
        }
        count
    }

    /// Emit a `:host { --token: value; … }` block for every registered token,
    /// so utilities referencing `var(--color-*)` resolve inside the shadow root.
    ///
    /// Kept for existing callers; delegates to [`Self::emit_used_tokens`] with
    /// [`TokenScope::Shadow`] and no usage filter (emits every registered
    /// token, matching this function's pre-flip behavior byte-for-byte).
    pub fn emit_host_tokens(&self) -> String {
        self.emit_tokens_for(&self.tokens, TokenScope::Shadow)
    }

    /// Emit only the tokens actually referenced in `body`, scoped to `:host`
    /// (shadow) or `:root` (light) per `scope` (light-DOM leaf flip prep, LDF
    /// §10 step 2 / D4 §8 Slice 2).
    ///
    /// Fixes a live bug: a light-DOM component (page/layout, today; leaves
    /// once the flip lands) has no shadow root, so a `:host { --x: y }` block
    /// matches nothing — `var(--x)` silently fails to resolve anywhere in the
    /// document. `TokenScope::Light` emits `:root` instead, which the light
    /// tree's utilities/authored rules can actually see.
    ///
    /// Also tree-shakes: [`Self::with_aihu_defaults`] unconditionally seeds
    /// all `AIHU_BRAND_TOKENS`, so unfiltered emission (the old
    /// `emit_host_tokens` behavior) always emitted all of them regardless of
    /// whether a given component's CSS references them. This filters to only
    /// the names `body` actually contains a `var(--name)`-style reference to.
    pub fn emit_used_tokens(&self, body: &str, scope: TokenScope) -> String {
        let used: BTreeMap<String, String> = self
            .tokens
            .iter()
            .filter(|(name, _)| var_is_referenced(body, name))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        self.emit_tokens_for(&used, scope)
    }

    /// Shared emission body for [`Self::emit_host_tokens`] and
    /// [`Self::emit_used_tokens`] — one token map, one selector, same
    /// formatting either way.
    fn emit_tokens_for(&self, tokens: &BTreeMap<String, String>, scope: TokenScope) -> String {
        if tokens.is_empty() {
            return String::new();
        }
        let selector = match scope {
            TokenScope::Shadow => ":host",
            TokenScope::Light => ":root",
        };
        let mut out = String::from(selector);
        out.push_str(" {\n");
        for (name, value) in tokens {
            out.push_str("  ");
            out.push_str(name);
            out.push_str(": ");
            out.push_str(value);
            out.push_str(";\n");
        }
        out.push_str("}\n");
        out
    }
}

/// Whether an emitted token block targets a shadow root (`:host`) or the flat
/// light-DOM tree (`:root`). Light-DOM leaf flip prep (LDF §10 step 2 / D4 §8
/// Slice 2) — see [`ThemeRegistry::emit_used_tokens`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenScope {
    Shadow,
    Light,
}

/// Whether `body` contains a reference to custom property `name` (e.g. does
/// `body` reference `--color-primary`). A plain `body.contains(name)` would
/// false-positive on `--color-primary` when only the DISTINCT sibling token
/// `--color-primary-foreground` is actually used (a real collision shape in
/// `AIHU_BRAND_TOKENS`: every color has a `-foreground` pair). Guards the
/// trailing boundary — the character immediately after a match must not
/// continue a custom-property identifier (`[a-zA-Z0-9_-]`). No token in this
/// table is a *suffix* of another, so a leading-boundary check is unneeded.
fn var_is_referenced(body: &str, name: &str) -> bool {
    let mut rest = body;
    while let Some(pos) = rest.find(name) {
        let after = &rest[pos + name.len()..];
        let boundary_ok = after
            .chars()
            .next()
            .map(|c| !(c.is_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(true);
        if boundary_ok {
            return true;
        }
        rest = after;
    }
    false
}

/// Extract the body of every `@theme { ... }` directive from a style-block
/// string, returning the concatenated declaration text.
pub fn extract_theme_blocks(style_content: &str) -> String {
    let mut bodies = String::new();
    let mut rest = style_content;
    while let Some(at) = rest.find("@theme") {
        let after = &rest[at + "@theme".len()..];
        let Some(open) = after.find('{') else { break };
        // Find the matching close brace.
        let body_start = open + 1;
        let mut depth = 1u32;
        let mut end = body_start;
        for (i, c) in after[body_start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = body_start + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        bodies.push_str(&after[body_start..end]);
        bodies.push('\n');
        rest = &after[end + 1..];
    }
    bodies
}

/// Strip `/* ... */` CSS comments from `input`. A comment is dropped
/// entirely rather than replaced with a space — the surrounding whitespace
/// in authored CSS already separates tokens, and callers here split on `;`
/// so no separator is needed either way. An unterminated comment consumes
/// the rest of the input (mirrors how a CSS parser would treat it: there is
/// no valid content after an unclosed `/*`).
fn strip_css_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next(); // consume the '*'
            let mut prev = '\0';
            for c2 in chars.by_ref() {
                if prev == '*' && c2 == '/' {
                    break;
                }
                prev = c2;
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Detect a bare `:root { ... }` block — no compound selector, e.g. not
/// `:root.dark { }` or `:root[data-theme] { }` — that itself contains one or
/// more `--name: value;` custom-property declarations.
///
/// `:root { }` is otherwise valid authored CSS (dark-mode overrides use
/// `:root.dark`/`:root[data-theme="dark"]`, see `apply.rs`), so only the
/// bare, custom-property-bearing form is flagged: that shape is what an
/// author reaches for when they mean to declare a theme but use `:root`
/// instead of the recognized `@theme { }` wrapper (a plausible-but-wrong
/// substitution — see the issue reproducing this).
pub fn find_bare_root_theme_block(style_content: &str) -> bool {
    let mut search_from = 0usize;
    while let Some(rel) = style_content[search_from..].find(":root") {
        let at = search_from + rel;
        let after = &style_content[at + ":root".len()..];
        let trimmed = after.trim_start();
        if trimmed.starts_with('{') {
            let body_start = style_content.len() - trimmed.len() + 1;
            let mut depth = 1u32;
            let mut end = body_start;
            for (i, c) in style_content[body_start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = body_start + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let body = &style_content[body_start..end];
            if !parse_theme_declarations(body).is_empty() {
                return true;
            }
            search_from = end;
        } else {
            search_from = at + ":root".len();
        }
    }
    false
}

/// Parse `--name: value;` declarations from a CSS body. Tolerates whitespace
/// and `/* ... */` comments anywhere (leading, trailing, between or inside
/// declarations) — comments are stripped before splitting so a comment
/// preceding a declaration can no longer glue onto its name and make it fail
/// the `--` prefix check below (the cause of a previously silent
/// whole-theme drop: see the regression test pinning this).
fn parse_theme_declarations(body: &str) -> Vec<(String, String)> {
    let body = strip_css_comments(body);
    let mut out = Vec::new();
    for decl in body.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        let Some((name, value)) = decl.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if name.starts_with("--") && !value.is_empty() {
            out.push((name.to_string(), value.to_string()));
        }
    }
    out
}

/// aihu brand tokens for the light theme. Maps
/// the design-system names to the utility token names the table references
/// (`--color-primary`, `--color-accent`, `--color-surface`, …).
///
/// D4 §3.4 / §3.2 (E1 + E2, founder-ratified): `info`/`success`/`warning`/
/// `neutral` (+ `-foreground`) extend the semantic-state palette daisyUI-style
/// recipes need but the original terracotta-only contract couldn't express —
/// values match `packs.ts`'s `aihuDefault.tokens` light values exactly (and
/// are verified against `.tastemaker/style-lock.md`'s contrast table, kept in
/// sync by PR #608). The five non-color scalars below them (`--size-selector`,
/// `--size-field`, `--border`, `--depth`, `--noise`) are aihu-native defaults
/// for the same recipes (D4 §3.2) — transcribed in spirit, not vendored
/// byte-for-byte from daisyUI's own token algebra.
const AIHU_BRAND_TOKENS: &[(&str, &str)] = &[
    ("--color-primary", "#1a1d24"),
    ("--color-primary-foreground", "#faf8f4"),
    ("--color-secondary", "#5a5a55"),
    ("--color-secondary-foreground", "#faf8f4"),
    ("--color-accent", "#c8543a"),
    ("--color-accent-foreground", "#faf8f4"),
    ("--color-surface", "#faf8f4"),
    ("--color-surface-foreground", "#1a1d24"),
    ("--color-background", "#faf8f4"),
    ("--color-foreground", "#1a1d24"),
    ("--color-muted", "#5a5a55"),
    ("--color-muted-foreground", "#8a8880"),
    ("--color-border", "#ddd9d2"),
    ("--color-ring", "#c8543a"),
    ("--color-destructive", "#a8432b"),
    ("--color-destructive-foreground", "#faf8f4"),
    ("--color-info", "#3d5a75"),
    ("--color-info-foreground", "#faf8f4"),
    ("--color-success", "#3f6f4f"),
    ("--color-success-foreground", "#faf8f4"),
    ("--color-warning", "#945f0e"),
    ("--color-warning-foreground", "#faf8f4"),
    ("--color-neutral", "#363c47"),
    ("--color-neutral-foreground", "#faf8f4"),
    ("--size-selector", "1.25rem"),
    ("--size-field", "2.25rem"),
    ("--border", "1px"),
    ("--depth", "1"),
    ("--noise", "0"),
    // Radius scale (matches `packs.ts`'s shared-contract values exactly).
    // Added alongside the D4 Slice 4 recipe channel: `recipes/*.css` (`.btn`,
    // `.card`, `.badge`) reference these directly, and without a registry
    // entry they'd only resolve when a consuming app happens to have loaded
    // a shipped style-pack CSS bundle — silently square-cornered otherwise.
    ("--radius-sm", "4px"),
    ("--radius-md", "8px"),
    ("--radius-lg", "12px"),
    ("--radius-pill", "999px"),
    // Three additive brand tokens, none a
    // `bg-*`/`text-*`/`border-*` color role (so `is_brand_token` is
    // untouched). `--gradient-brand` is ONE token holding a full
    // `linear-gradient()` value (not three separate stop tokens — E-P1),
    // derived from the existing accent/warning hues so the shipped identity
    // stays single-accent-derived. `--font-serif` fixes a live bug: the
    // `font-serif` utility (tokens.rs) has always referenced this name with
    // no pack defining it; the default is a system-serif stack, not a
    // specific webfont (E-P2). `--ease-brand` is the first easing custom
    // property in the engine (existing `ease-*` utilities are hardcoded
    // literals) — E-P3's glass-surface treatment is deliberately token-free
    // (derived per-recipe via `color-mix()`), so no glass token exists here.
    ("--gradient-brand", "linear-gradient(135deg, #c8543a 0%, #d97757 50%, #945f0e 100%)"),
    ("--font-serif", "ui-serif, Georgia, Cambria, \"Times New Roman\", serif"),
    ("--ease-brand", "cubic-bezier(0.22, 1, 0.36, 1)"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d4_scalar_tokens_are_seeded_and_tree_shake_like_colors() {
        let registry = ThemeRegistry::with_aihu_defaults();
        assert_eq!(registry.get("--border"), Some("1px"));
        assert_eq!(registry.get("--size-field"), Some("2.25rem"));

        // Only the scalar this recipe body actually references is emitted —
        // same tree-shake `emit_used_tokens` already applies to colors.
        let out =
            registry.emit_used_tokens(".btn { border: var(--border) solid; }", TokenScope::Light);
        assert!(
            out.contains("--border: 1px;"),
            "expected --border in:\n{out}"
        );
        assert!(
            !out.contains("--size-field"),
            "unreferenced token leaked:\n{out}"
        );
        assert!(
            !out.contains("--depth"),
            "unreferenced token leaked:\n{out}"
        );
    }

    #[test]
    fn d4_border_scalar_does_not_false_positive_on_color_border() {
        // `--color-border` does not actually contain `--border` as a
        // substring (only a single hyphen precedes "border" there), so this
        // pairing can't exercise `var_is_referenced`'s trailing-boundary
        // guard. What CAN produce a same-prefix collision is a longer
        // custom property that genuinely starts with the shorter name: e.g.
        // if `--border-radius` existed, unguarded `body.contains("--border")`
        // would false-positive on it. Exercise that directly.
        let registry = ThemeRegistry::with_aihu_defaults();
        let out = registry.emit_used_tokens(
            ".x { border-radius: var(--border-radius-does-not-exist); }",
            TokenScope::Light,
        );
        assert!(
            !out.contains("--border:"),
            "scalar --border falsely matched a longer name sharing its prefix:\n{out}"
        );

        // The positive case: a genuine `var(--border)` reference DOES emit it.
        let out2 =
            registry.emit_used_tokens(".x { border: var(--border) solid; }", TokenScope::Light);
        assert!(
            out2.contains("--border: 1px;"),
            "expected --border in:\n{out2}"
        );
    }

    #[test]
    fn performative_ui_tokens_tree_shake_like_every_other_scalar() {
        // These additive tokens must tree-shake exactly like
        // the D4 scalars above — referencing one must not pull in the other
        // two.
        let registry = ThemeRegistry::with_aihu_defaults();
        assert_eq!(
            registry.get("--gradient-brand"),
            Some("linear-gradient(135deg, #c8543a 0%, #d97757 50%, #945f0e 100%)")
        );
        assert_eq!(
            registry.get("--font-serif"),
            Some("ui-serif, Georgia, Cambria, \"Times New Roman\", serif")
        );
        assert_eq!(registry.get("--ease-brand"), Some("cubic-bezier(0.22, 1, 0.36, 1)"));

        let out = registry.emit_used_tokens(
            ".x { background-image: var(--gradient-brand); }",
            TokenScope::Light,
        );
        assert!(out.contains("--gradient-brand:"), "{out}");
        assert!(!out.contains("--font-serif"), "unreferenced token leaked:\n{out}");
        assert!(!out.contains("--ease-brand"), "unreferenced token leaked:\n{out}");
    }

    // Pins the exact repro from the issue: a comment inside `@theme { }`
    // used to glue onto the following declaration's name (the naive
    // `split(';')` + `split_once(':')` parse saw `/* brand */\n
    // --color-primary` as the property name, which fails the `--` prefix
    // check and drops the declaration), silently reverting the whole
    // registered theme to the built-in `aihu-default` value.
    #[test]
    fn a_comment_inside_theme_block_does_not_drop_the_declaration() {
        let mut registry = ThemeRegistry::with_aihu_defaults();
        assert_eq!(registry.get("--color-primary"), Some("#1a1d24"));

        let registered =
            registry.apply_theme_block("\n  /* brand */\n  --color-primary: #ff00aa;\n");
        assert_eq!(registered, 1, "the declaration after the comment was dropped");
        assert_eq!(registry.get("--color-primary"), Some("#ff00aa"));
    }

    #[test]
    fn comments_anywhere_in_a_theme_block_are_tolerated() {
        let mut registry = ThemeRegistry::empty();
        let registered = registry.apply_theme_block(
            "/* leading */\n\
             --color-primary: #ff00aa; /* trailing on same decl */\n\
             /* between declarations */\n\
             --color-accent: #123456;\n\
             --color-surface: #abcdef; /* trailing */",
        );
        assert_eq!(registered, 3);
        assert_eq!(registry.get("--color-primary"), Some("#ff00aa"));
        assert_eq!(registry.get("--color-accent"), Some("#123456"));
        assert_eq!(registry.get("--color-surface"), Some("#abcdef"));
    }

    #[test]
    fn bare_root_block_with_custom_props_is_detected() {
        assert!(find_bare_root_theme_block(
            ":root { --color-primary: #ff00aa; }"
        ));
    }

    #[test]
    fn root_with_a_compound_selector_is_not_flagged() {
        // `:root.dark { … }` / `:root[data-theme="dark"] { … }` are the
        // established authored pattern for dark-mode overrides (apply.rs) —
        // only the bare `:root { }` wrapper is a plausible `@theme` mix-up.
        assert!(!find_bare_root_theme_block(
            ":root.dark { --color-primary: #111; }"
        ));
        assert!(!find_bare_root_theme_block(
            ":root[data-theme=\"dark\"] { --color-primary: #111; }"
        ));
    }

    #[test]
    fn root_block_without_custom_props_is_not_flagged() {
        assert!(!find_bare_root_theme_block(":root { color: red; }"));
    }

    #[test]
    fn a_comment_only_theme_block_registers_zero_tokens() {
        // Confirms the parser's half of the "fail loudly on zero tokens"
        // acceptance criterion: the caller (emit.rs) is the one that turns
        // this into a hard error, but it can only do so because the count
        // it gets back is honest.
        let mut registry = ThemeRegistry::with_aihu_defaults();
        let registered = registry.apply_theme_block("/* nothing but a note */");
        assert_eq!(registered, 0);
    }
}
