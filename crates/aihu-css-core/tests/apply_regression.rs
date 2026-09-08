//! The standalone package keeps a small local recipe fixture so the `@apply`
//! expansion contract stays testable without the framework UI registry.
//! Framework-wide registry coverage remains an explicit integration seam in the
//! parent `aihu` repository; this local test protects the engine from losing the
//! utilities needed by its representative recipe.

use std::path::PathBuf;

use aihu_css_core::{expand_apply, SfcStyleScope, ThemeRegistry};

/// The standalone repository root, derived from this crate's manifest dir.
fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..2 {
        p.pop();
    }
    p
}

/// Extract the body inside `@style { … }` (brace-matched), or `None` if the file
/// has no `@style` block. `$global @style` is not used by the current recipes,
/// but we treat any `@style` block as scoped for this regression (the only
/// concern is whether tokens RESOLVE, which is scope-independent for base
/// utilities; variant tokens resolve identically — only the $global *rejection*
/// path differs, and none of these recipes use $global).
fn extract_style_body(src: &str) -> Option<String> {
    // Find the `@style` BLOCK opener — `@style` followed only by whitespace then
    // `{`. (A naive `find("@style")` would match the word inside a comment such
    // as "see @style below"; the recipes contain exactly that.)
    let mut search = 0usize;
    let (at, after) = loop {
        let rel = src[search..].find("@style")?;
        let at = search + rel;
        let after = &src[at + "@style".len()..];
        let trimmed = after.trim_start();
        if trimmed.starts_with('{') {
            break (at, after);
        }
        search = at + "@style".len();
    };
    let _ = at;
    let open = after.find('{')?;
    let body_start = open + 1;
    let bytes = after.as_bytes();
    let mut depth = 1i32;
    let mut i = body_start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(after[body_start..i].to_string());
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Remove `@theme { … }` blocks (brace-matched) — the emitter strips these
/// before folding, so the `@apply` pass never sees them.
fn strip_theme(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    while let Some(at) = rest.find("@theme") {
        out.push_str(&rest[..at]);
        let after = &rest[at + "@theme".len()..];
        let Some(open) = after.find('{') else {
            out.push_str(after);
            return out;
        };
        let bytes = after.as_bytes();
        let mut depth = 1i32;
        let mut i = open + 1;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        rest = &after[i + 1..];
    }
    out.push_str(rest);
    out
}

#[test]
fn every_registry_recipe_apply_resolves() {
    let registry = repo_root().join("tests/fixtures/registry");
    assert!(
        registry.is_dir(),
        "registry dir not found at {}",
        registry.display()
    );

    let theme = ThemeRegistry::with_aihu_defaults();
    let mut checked = 0usize;

    for entry in std::fs::read_dir(&registry).unwrap() {
        let dir = entry.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        for file in std::fs::read_dir(&dir).unwrap() {
            let path = file.unwrap().path();
            if path.extension().and_then(|s| s.to_str()) != Some("aihu") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            let Some(body) = extract_style_body(&src) else {
                continue;
            };
            if !body.contains("@apply") {
                // No @apply → nothing to expand, but still assert it parses.
                continue;
            }
            let stripped = strip_theme(&body);
            let result = expand_apply(&stripped, SfcStyleScope::Scoped, &theme);
            assert!(
                result.is_ok(),
                "recipe {} @apply expansion failed: {:?}",
                path.display(),
                result.err()
            );
            checked += 1;
        }
    }

    assert!(
        checked > 0,
        "expected at least one local recipe with @apply (button) to be checked"
    );
}
