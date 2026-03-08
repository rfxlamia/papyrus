# Phase 4 CommonMark Renderer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement deterministic CommonMark output from the Phase 3 AST via `Document::to_markdown()` and `ConversionResult::to_markdown()`.

**Architecture:** Keep rendering logic centralized in `papyrus-core/src/renderer/mod.rs` as pure functions for escaping, span rendering, node rendering, and whole-document normalization. Expose public convenience methods on AST types (`Document`, `ConversionResult`) that delegate to renderer internals so callers use one obvious API. Validate behavior bottom-up (escape -> span -> node -> document -> CommonMark parser round-trip) with strict TDD.

**Tech Stack:** Rust 2021, Cargo tests, existing papyrus AST/detector pipeline, `pulldown-cmark` (dev-dependency only) for compliance validation.

---

**Execution Notes**
- Work in a dedicated worktree created before execution (`@brainstorming` context assumption).
- Use `@test-driven-development` for every task.
- Use `@systematic-debugging` immediately for any unexpected red test.
- Use `@verification-before-completion` before claiming Phase 4 is done.
- Keep implementation DRY/YAGNI: no new AST variants, no code-span support in v0.1.

### Task 1: Replace Renderer Stub with Phase 4 Surface

**Files:**
- Modify: `papyrus-core/src/renderer/mod.rs:1-4`
- Modify: `papyrus-core/tests/module_surface.rs:1-122`
- Test: `papyrus-core/tests/module_surface.rs`

**Step 1: Write the failing test**

```rust
// papyrus-core/tests/module_surface.rs
use papyrus_core::ast::{Document, DocumentMetadata};
use papyrus_core::renderer;

#[test]
fn renderer_surface_exposes_document_entrypoint() {
    let doc = Document {
        metadata: DocumentMetadata {
            title: None,
            author: None,
            page_count: 0,
        },
        nodes: vec![],
    };

    let markdown = renderer::render_document(&doc);
    assert!(markdown.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core renderer_surface_exposes_document_entrypoint -v`  
Expected: FAIL with unresolved function `renderer::render_document`.

**Step 3: Write minimal implementation**

```rust
// papyrus-core/src/renderer/mod.rs
use crate::ast::Document;

pub fn render_document(_document: &Document) -> String {
    String::new()
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core renderer_surface_exposes_document_entrypoint -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/renderer/mod.rs papyrus-core/tests/module_surface.rs
git commit -m "feat(renderer): add phase-4 renderer document entrypoint"
```

### Task 2: Implement CommonMark Escaping Helper

**Files:**
- Modify: `papyrus-core/src/renderer/mod.rs`
- Test: `papyrus-core/src/renderer/mod.rs` (`#[cfg(test)]` module)

**Step 1: Write the failing test**

```rust
#[test]
fn escape_text_escapes_all_phase4_commonmark_special_chars() {
    let raw = r"\`*_{}[]()#+-.!|";
    let escaped = escape_text(raw);
    assert_eq!(escaped, r"\\\`\*\_\{\}\[\]\(\)\#\+\-\.\!\|");
}

#[test]
fn escape_text_leaves_safe_text_unchanged() {
    assert_eq!(escape_text("Papyrus Renderer 123"), "Papyrus Renderer 123");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core escape_text_ -v`  
Expected: FAIL with missing function `escape_text`.

**Step 3: Write minimal implementation**

```rust
fn escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#'
            | '+' | '-' | '.' | '!' | '|' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core escape_text_ -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/renderer/mod.rs
git commit -m "feat(renderer): add commonmark character escaping helper"
```

### Task 3: Implement Span Rendering Rules

**Files:**
- Modify: `papyrus-core/src/renderer/mod.rs`
- Test: `papyrus-core/src/renderer/mod.rs`

**Step 1: Write the failing test**

```rust
fn span(text: &str, bold: bool, italic: bool) -> Span {
    Span {
        text: text.to_string(),
        bold,
        italic,
        font_size: 12.0,
        font_name: None,
    }
}

#[test]
fn render_span_supports_plain_bold_italic_and_bold_italic() {
    assert_eq!(render_span(&span("plain", false, false)), "plain");
    assert_eq!(render_span(&span("bold", true, false)), "**bold**");
    assert_eq!(render_span(&span("italic", false, true)), "*italic*");
    assert_eq!(render_span(&span("both", true, true)), "***both***");
}

#[test]
fn render_span_drops_empty_formatted_output() {
    assert_eq!(render_span(&span("", true, false)), "");
    assert_eq!(render_span(&span("   ", true, true)), "");
}

#[test]
fn render_span_trims_whitespace_inside_markers_only() {
    assert_eq!(render_span(&span("  bold me  ", true, false)), "  **bold me**  ");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core render_span_ -v`  
Expected: FAIL because `render_span` is not implemented.

**Step 3: Write minimal implementation**

```rust
use crate::ast::Span;

pub(crate) fn render_span(span: &Span) -> String {
    if span.text.is_empty() {
        return String::new();
    }

    if !span.bold && !span.italic {
        return escape_text(&span.text);
    }

    let leading_len = span.text.len() - span.text.trim_start().len();
    let trailing_len = span.text.len() - span.text.trim_end().len();
    let leading = &span.text[..leading_len];
    let trailing = &span.text[span.text.len() - trailing_len..];
    let core = span.text.trim();

    if core.is_empty() {
        return String::new();
    }

    let marker = match (span.bold, span.italic) {
        (true, true) => "***",
        (true, false) => "**",
        (false, true) => "*",
        (false, false) => "",
    };

    let escaped_core = escape_text(core);
    format!("{leading}{marker}{escaped_core}{marker}{trailing}")
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core render_span_ -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/renderer/mod.rs
git commit -m "feat(renderer): implement span rendering semantics"
```

### Task 4: Implement Node Rendering Rules

**Files:**
- Modify: `papyrus-core/src/renderer/mod.rs`
- Test: `papyrus-core/src/renderer/mod.rs`

**Step 1: Write the failing test**

```rust
use crate::ast::Node;

#[test]
fn render_node_heading_uses_hash_prefix_and_blank_line() {
    let node = Node::Heading {
        level: 3,
        spans: vec![span("Heading", false, false)],
    };
    assert_eq!(render_node(&node), "### Heading\n\n");
}

#[test]
fn render_node_paragraph_joins_spans_without_extra_spaces() {
    let node = Node::Paragraph {
        spans: vec![
            span("Hello", false, false),
            span(" ", false, false),
            span("world", true, false),
        ],
    };
    assert_eq!(render_node(&node), "Hello **world**\n\n");
}

#[test]
fn render_node_raw_text_passthrough_appends_blank_line() {
    assert_eq!(render_node(&Node::RawText("raw".to_string())), "raw\n\n");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core render_node_ -v`  
Expected: FAIL because `render_node` is not implemented.

**Step 3: Write minimal implementation**

```rust
use crate::ast::Node;

fn render_spans(spans: &[Span]) -> String {
    spans
        .iter()
        .map(render_span)
        .filter(|s| !s.is_empty())
        .collect::<String>()
}

fn trim_trailing_ws_per_line(input: &str) -> String {
    input
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_node(node: &Node) -> String {
    match node {
        Node::Heading { level, spans } => {
            let hashes = "#".repeat((*level).clamp(1, 6) as usize);
            let text = trim_trailing_ws_per_line(&render_spans(spans));
            format!("{hashes} {text}\n\n")
        }
        Node::Paragraph { spans } => {
            let text = trim_trailing_ws_per_line(&render_spans(spans));
            format!("{text}\n\n")
        }
        Node::RawText(text) => {
            let cleaned = trim_trailing_ws_per_line(text);
            format!("{cleaned}\n\n")
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core render_node_ -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/renderer/mod.rs
git commit -m "feat(renderer): implement block-level node rendering"
```

### Task 5: Implement Full Document Rendering Normalization

**Files:**
- Modify: `papyrus-core/src/renderer/mod.rs`
- Test: `papyrus-core/src/renderer/mod.rs`

**Step 1: Write the failing test**

```rust
use crate::ast::{Document, DocumentMetadata};

#[test]
fn render_document_has_single_trailing_newline_for_non_empty_docs() {
    let doc = Document {
        metadata: DocumentMetadata {
            title: None,
            author: None,
            page_count: 1,
        },
        nodes: vec![
            Node::Heading {
                level: 1,
                spans: vec![span("Title", false, false)],
            },
            Node::Paragraph {
                spans: vec![span("Body", false, false)],
            },
        ],
    };

    let markdown = render_document(&doc);
    assert_eq!(markdown, "# Title\n\nBody\n");
    assert!(markdown.ends_with('\n'));
    assert!(!markdown.ends_with("\n\n"));
}

#[test]
fn render_document_empty_doc_is_empty_string() {
    let doc = Document {
        metadata: DocumentMetadata {
            title: None,
            author: None,
            page_count: 0,
        },
        nodes: vec![],
    };

    assert_eq!(render_document(&doc), "");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core render_document_ -v`  
Expected: FAIL because `render_document` still returns empty output for all inputs.

**Step 3: Write minimal implementation**

```rust
pub fn render_document(document: &Document) -> String {
    let body = document
        .nodes
        .iter()
        .map(render_node)
        .collect::<String>()
        .trim_start_matches('\n')
        .trim_end_matches('\n')
        .to_string();

    if body.is_empty() {
        String::new()
    } else {
        format!("{body}\n")
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core render_document_ -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/renderer/mod.rs
git commit -m "feat(renderer): normalize full document markdown output"
```

### Task 6: Wire `Document::to_markdown()` and `ConversionResult::to_markdown()`

**Files:**
- Modify: `papyrus-core/src/ast/mod.rs`
- Modify: `papyrus-core/tests/module_surface.rs`
- Test: `papyrus-core/tests/module_surface.rs`

**Step 1: Write the failing test**

```rust
// papyrus-core/tests/module_surface.rs
use papyrus_core::ast::{ConversionResult, Document, DocumentMetadata, Node, Span};

#[test]
fn markdown_api_methods_delegate_to_renderer_output() {
    let document = Document {
        metadata: DocumentMetadata {
            title: None,
            author: None,
            page_count: 1,
        },
        nodes: vec![Node::Paragraph {
            spans: vec![Span {
                text: "phase4".to_string(),
                bold: false,
                italic: false,
                font_size: 12.0,
                font_name: None,
            }],
        }],
    };

    let result = ConversionResult {
        document: document.clone(),
        warnings: vec![],
    };

    assert_eq!(document.to_markdown(), "phase4\n");
    assert_eq!(result.to_markdown(), "phase4\n");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core markdown_api_methods_delegate_to_renderer_output -v`  
Expected: FAIL with missing methods on `Document` and `ConversionResult`.

**Step 3: Write minimal implementation**

```rust
// papyrus-core/src/ast/mod.rs
impl Document {
    pub fn to_markdown(&self) -> String {
        crate::renderer::render_document(self)
    }
}

impl ConversionResult {
    pub fn to_markdown(&self) -> String {
        self.document.to_markdown()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core markdown_api_methods_delegate_to_renderer_output -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/ast/mod.rs papyrus-core/tests/module_surface.rs
git commit -m "feat(ast): expose document and conversionresult markdown APIs"
```

### Task 7: Add CommonMark Compliance Round-Trip Test

**Files:**
- Modify: `papyrus-core/Cargo.toml`
- Create: `papyrus-core/tests/integration_markdown_roundtrip.rs`
- Test: `papyrus-core/tests/integration_markdown_roundtrip.rs`

**Step 1: Write the failing test**

```rust
// papyrus-core/tests/integration_markdown_roundtrip.rs
use papyrus_core::ast::{Document, DocumentMetadata, Node, Span};
use pulldown_cmark::{Event, Parser, Tag};

fn span(text: &str, bold: bool, italic: bool) -> Span {
    Span {
        text: text.to_string(),
        bold,
        italic,
        font_size: 12.0,
        font_name: None,
    }
}

#[test]
fn markdown_round_trip_matches_ast_shape() {
    let doc = Document {
        metadata: DocumentMetadata {
            title: None,
            author: None,
            page_count: 1,
        },
        nodes: vec![
            Node::Heading {
                level: 2,
                spans: vec![span("Phase 4", false, false)],
            },
            Node::Paragraph {
                spans: vec![
                    span("bold", true, false),
                    span(" and ", false, false),
                    span("italic", false, true),
                ],
            },
        ],
    };

    let markdown = doc.to_markdown();
    let events = Parser::new(&markdown).collect::<Vec<_>>();

    let heading_count = events
        .iter()
        .filter(|event| matches!(event, Event::Start(Tag::Heading { .. })))
        .count();
    let paragraph_count = events
        .iter()
        .filter(|event| matches!(event, Event::Start(Tag::Paragraph)))
        .count();
    let strong_count = events
        .iter()
        .filter(|event| matches!(event, Event::Start(Tag::Strong)))
        .count();
    let emphasis_count = events
        .iter()
        .filter(|event| matches!(event, Event::Start(Tag::Emphasis)))
        .count();

    assert_eq!(heading_count, 1);
    assert_eq!(paragraph_count, 1);
    assert_eq!(strong_count, 1);
    assert_eq!(emphasis_count, 1);
    assert!(markdown.ends_with('\n'));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core --test integration_markdown_roundtrip -v`  
Expected: FAIL with unresolved crate `pulldown_cmark`.

**Step 3: Write minimal implementation**

```toml
# papyrus-core/Cargo.toml
[dev-dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
pulldown-cmark = "0.10"
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core --test integration_markdown_roundtrip -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/Cargo.toml papyrus-core/tests/integration_markdown_roundtrip.rs
git commit -m "test(renderer): add commonmark round-trip compliance coverage"
```

### Task 8: Final Verification and Phase 4 Log

**Files:**
- Create: `docs/plans/phase-4-verification-log.md`
- Test: `papyrus-core/src/renderer/mod.rs` and integration suites

**Step 1: Add a final regression test for escaping inside formatting**

```rust
#[test]
fn render_span_escapes_inner_text_without_escaping_markers() {
    let out = render_span(&span("A*B", true, false));
    assert_eq!(out, "**A\\*B**");
}
```

**Step 2: Run targeted and full verification**

Run:

```bash
cargo test -p papyrus-core render_span_escapes_inner_text_without_escaping_markers -v
cargo test -p papyrus-core --test module_surface -v
cargo test -p papyrus-core --test integration_markdown_roundtrip -v
cargo test -p papyrus-core -v
cargo test --workspace
```

Expected: all PASS, 0 failures.

**Step 3: Record verification log**

```markdown
# Phase 4 Verification Log

- **Date:** 2026-03-08
- **Branch:** `feature/phase-4-commonmark-renderer`
- **Base commit:** `<fill after execution>`
- **HEAD commit:** `<fill after execution>`

## Commands run
... (paste exact commands from Step 2)

## Results
All green — 0 failures.
```

**Step 4: Commit verification artifacts**

```bash
git add papyrus-core/src/renderer/mod.rs docs/plans/phase-4-verification-log.md
git commit -m "test(renderer): finalize escaping edge case and phase-4 verification log"
```

**Step 5: Final sanity check**

Run: `git status --short`  
Expected: clean working tree.

---

## Definition of Done Checklist

- [ ] `render_span` supports plain, bold, italic, bold+italic, and empty/whitespace-only formatted spans.
- [ ] CommonMark special chars are escaped in body text.
- [ ] `render_node` emits valid block markdown with single blank-line separation.
- [ ] `render_document` has no leading blank lines, no trailing whitespace, and exactly one trailing newline for non-empty docs.
- [ ] `Document::to_markdown()` and `ConversionResult::to_markdown()` are publicly available.
- [ ] Round-trip CommonMark parser test passes with `pulldown-cmark`.
- [ ] `cargo test --workspace` is green.

