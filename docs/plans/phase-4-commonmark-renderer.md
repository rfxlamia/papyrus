# Phase 4: CommonMark Renderer

**Goal**: Implement `Document::to_markdown()` that renders the AST into a valid CommonMark string.

**Depends on**: Phase 3 (Smart Outline & Public API)
**Blocks**: Phase 5 (CLI Interface)

---

## Tasks

### 4.1 Implement Span Rendering

Render a single `Span` to inline Markdown:

```rust
fn render_span(span: &Span) -> String
```

Rules:
- Plain text -> `text`
- Bold -> `**text**`
- Italic -> `*text*`
- Bold + Italic -> `***text***`
- Trim leading/trailing whitespace within formatting marks (CommonMark rule: `** bold**` is invalid, must be `**bold**`)
- Handle empty text spans (skip, don't emit empty markers like `****`)

### 4.2 Implement Node Rendering

Render each `Node` variant to block-level Markdown:

```rust
fn render_node(node: &Node) -> String
```

Rules:
- `Node::Heading { level, spans }` -> `# ` / `## ` / etc. prefix + rendered spans + `\n\n`
- `Node::Paragraph { spans }` -> rendered spans joined + `\n\n`
- `Node::RawText(text)` -> text as-is + `\n\n`
- Between consecutive blocks: exactly one blank line (two newlines)

### 4.3 Implement `Document::to_markdown()`

Render the full document:

```rust
impl Document {
    pub fn to_markdown(&self) -> String {
        self.nodes.iter()
            .map(render_node)
            .collect::<Vec<_>>()
            .join("")
            .trim_end()
            .to_string()
    }
}
```

Additional rules:
- No trailing whitespace on any line
- No leading blank lines
- Single trailing newline at end of file
- Consecutive spans within a node are joined without extra whitespace (the span text itself contains necessary spacing)

### 4.4 CommonMark Escape Handling

Escape characters that have special meaning in CommonMark when they appear in the source text:

Characters to escape: `\`, `` ` ``, `*`, `_`, `{`, `}`, `[`, `]`, `(`, `)`, `#`, `+`, `-`, `.`, `!`, `|`

Only escape these when they appear in body text, NOT inside code spans (not applicable in v0.1 since we don't detect code) and NOT when they're being used as formatting markers.

### 4.5 Wire Rendering into ConversionResult

Update `ConversionResult` or add a convenience method so callers can get Markdown directly:

```rust
impl ConversionResult {
    pub fn to_markdown(&self) -> String {
        self.document.to_markdown()
    }
}
```

### 4.6 Unit Tests for Rendering

Test cases for span rendering:
- Plain text passthrough
- Bold wrapping
- Italic wrapping
- Bold+italic wrapping
- Empty span -> no output
- Span with only whitespace -> no formatting markers

Test cases for node rendering:
- Heading levels 1-4 with correct `#` prefix count
- Paragraph with mixed bold/italic spans
- RawText passthrough
- Blank line separation between nodes

Test cases for full document rendering:
- Multi-node document produces valid CommonMark
- No trailing whitespace
- Single trailing newline

Test cases for escaping:
- Special characters in body text are escaped
- Characters inside formatting markers are not double-escaped

### 4.7 CommonMark Compliance Validation

Create a test that:
1. Renders a document to Markdown
2. Parses the output with a CommonMark parser (`pulldown-cmark` crate, dev-dependency only)
3. Asserts the parsed structure matches the original AST

This ensures round-trip fidelity: AST -> Markdown -> parsed AST should be structurally equivalent.

**Dev dependency added:**
- `pulldown-cmark` in `papyrus-core/Cargo.toml` (dev-dependencies only)

---

## Definition of Done

- [ ] `Span` rendering handles plain, bold, italic, bold+italic, empty
- [ ] `Node` rendering produces correct heading prefixes and paragraph blocks
- [ ] `Document::to_markdown()` produces well-formed CommonMark
- [ ] Special characters are escaped in body text
- [ ] No trailing whitespace, correct blank line separation, single trailing newline
- [ ] `ConversionResult::to_markdown()` convenience method works
- [ ] Round-trip test passes (AST -> Markdown -> parsed AST via `pulldown-cmark`)
- [ ] Unit tests cover all rendering edge cases
- [ ] `cargo test` passes with all new tests green
