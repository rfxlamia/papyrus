# Phase 1 Scaffold and Oracle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a compilable Rust workspace (`papyrus-core` + `papyrus-cli`) with core AST types and a PyMuPDF-based oracle test harness with committed PDF/JSON fixtures.

**Architecture:** Start from a minimal workspace shell, then grow behavior through TDD in small increments. Keep `papyrus-core` library-first (AST + module stubs), keep CLI behavior intentionally minimal, and isolate oracle tooling under `tests/oracle` with deterministic fixture outputs. Every task ends with verification + commit to keep rollback and review cheap.

**Tech Stack:** Rust (Cargo workspace, `lopdf`, `clap`), Python 3 (`PyMuPDF`, `pytest`), Markdown docs, Git.

---

**Execution Notes**
- Use `@test-driven-development` for every implementation task.
- Use `@verification-before-completion` before claiming each task done.
- Use `python3` for all Python commands.

### Task 1: Workspace Bootstrap and CLI Stub

**Files:**
- Create: `Cargo.toml`
- Create: `papyrus-core/Cargo.toml`
- Create: `papyrus-core/src/lib.rs`
- Create: `papyrus-cli/Cargo.toml`
- Create: `papyrus-cli/src/main.rs`

**Step 1: Write the failing test**

```rust
// papyrus-cli/src/main.rs
#[cfg(test)]
mod tests {
    use super::stub_message;

    #[test]
    fn stub_message_is_stable() {
        assert_eq!(stub_message(), "papyrus-cli: not yet implemented");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-cli stub_message_is_stable -v`
Expected: FAIL with unresolved item error for `stub_message`.

**Step 3: Write minimal implementation**

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["papyrus-core", "papyrus-cli"]
resolver = "2"
```

```toml
# papyrus-core/Cargo.toml
[package]
name = "papyrus-core"
version = "0.1.0"
edition = "2021"

[dependencies]
lopdf = "0.35"
```

```rust
// papyrus-core/src/lib.rs
pub mod ast;
pub mod detector;
pub mod parser;
pub mod renderer;
```

```toml
# papyrus-cli/Cargo.toml
[package]
name = "papyrus-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
```

```rust
// papyrus-cli/src/main.rs
pub fn stub_message() -> &'static str {
    "papyrus-cli: not yet implemented"
}

fn main() {
    println!("{}", stub_message());
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-cli stub_message_is_stable -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml papyrus-core/Cargo.toml papyrus-core/src/lib.rs papyrus-cli/Cargo.toml papyrus-cli/src/main.rs
git commit -m "chore: scaffold workspace and cli stub"
```

### Task 2: Core AST Types and Contracts

**Files:**
- Create: `papyrus-core/src/ast/mod.rs`
- Modify: `papyrus-core/src/lib.rs`

**Step 1: Write the failing test**

```rust
// append into papyrus-core/src/ast/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_result_keeps_warnings_and_raw_text() {
        let result = ConversionResult {
            document: Document {
                metadata: DocumentMetadata {
                    title: None,
                    author: None,
                    page_count: 0,
                },
                nodes: vec![Node::RawText("fallback".to_string())],
            },
            warnings: vec![Warning::MalformedPdfObject {
                detail: "broken object".to_string(),
            }],
        };

        assert_eq!(result.document.nodes.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core conversion_result_keeps_warnings_and_raw_text -v`
Expected: FAIL with missing type errors (`ConversionResult`, `Document`, `Node`, `Warning`).

**Step 3: Write minimal implementation**

```rust
// papyrus-core/src/ast/mod.rs
#[derive(Debug, Clone, PartialEq)]
pub struct ConversionResult {
    pub document: Document,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub metadata: DocumentMetadata,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Heading { level: u8, spans: Vec<Span> },
    Paragraph { spans: Vec<Span> },
    RawText(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub font_size: f32,
    pub font_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    MissingFontMetrics { font_name: String, page: usize },
    UnreadableTextStream { page: usize, detail: String },
    UnsupportedEncoding { encoding: String, page: usize },
    MalformedPdfObject { detail: String },
}
```

```rust
// papyrus-core/src/lib.rs
pub mod ast;
pub mod detector;
pub mod parser;
pub mod renderer;
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core conversion_result_keeps_warnings_and_raw_text -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/ast/mod.rs papyrus-core/src/lib.rs
git commit -m "feat: add phase-1 ast core types"
```

### Task 3: Parser/Detector/Renderer Module Stubs

**Files:**
- Create: `papyrus-core/src/parser/mod.rs`
- Create: `papyrus-core/src/detector/mod.rs`
- Create: `papyrus-core/src/renderer/mod.rs`
- Modify: `papyrus-core/src/lib.rs`

**Step 1: Write the failing test**

```rust
// papyrus-core/tests/module_surface.rs
use papyrus_core::{detector, parser, renderer};

#[test]
fn module_surfaces_are_linked() {
    let parsed = parser::parse_pdf_bytes(b"%PDF-1.7");
    let detected = detector::detect_structure(parsed);
    let markdown = renderer::render_markdown(&detected);
    assert!(markdown.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core module_surfaces_are_linked -v`
Expected: FAIL with missing functions in `parser`, `detector`, `renderer`.

**Step 3: Write minimal implementation**

```rust
// papyrus-core/src/parser/mod.rs
pub fn parse_pdf_bytes(_bytes: &[u8]) -> Vec<String> {
    Vec::new()
}
```

```rust
// papyrus-core/src/detector/mod.rs
pub fn detect_structure(chunks: Vec<String>) -> Vec<String> {
    chunks
}
```

```rust
// papyrus-core/src/renderer/mod.rs
pub fn render_markdown(_nodes: &[String]) -> String {
    String::new()
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p papyrus-core module_surfaces_are_linked -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add papyrus-core/src/parser/mod.rs papyrus-core/src/detector/mod.rs papyrus-core/src/renderer/mod.rs papyrus-core/tests/module_surface.rs
git commit -m "chore: add parser detector renderer stubs"
```

### Task 4: Deterministic PDF Fixture Generator

**Files:**
- Create: `tests/fixtures/generate_fixtures.py`
- Create: `tests/fixtures/simple.pdf`
- Create: `tests/fixtures/multi-heading.pdf`
- Create: `tests/fixtures/bold-italic.pdf`
- Create: `tests/fixtures/corrupted.pdf`

**Step 1: Write the failing test**

```python
# tests/fixtures/test_generate_fixtures.py
from pathlib import Path

def test_expected_fixture_files_exist():
    base = Path("tests/fixtures")
    expected = {
        "simple.pdf",
        "multi-heading.pdf",
        "bold-italic.pdf",
        "corrupted.pdf",
    }
    assert expected.issubset({p.name for p in base.glob("*.pdf")})
```

**Step 2: Run test to verify it fails**

Run: `python3 -m pytest tests/fixtures/test_generate_fixtures.py -q`
Expected: FAIL because fixture PDFs do not exist yet.

**Step 3: Write minimal implementation**

```python
# tests/fixtures/generate_fixtures.py
from pathlib import Path
import fitz

OUT = Path(__file__).resolve().parent

def _write_pdf(path: Path, lines):
    doc = fitz.open()
    page = doc.new_page()
    y = 72
    for text, size, font in lines:
        page.insert_text((72, y), text, fontsize=size, fontname=font)
        y += size + 10
    doc.save(path)
    doc.close()

def main():
    OUT.mkdir(parents=True, exist_ok=True)
    _write_pdf(OUT / "simple.pdf", [("Chapter 1", 24, "helv"), ("Body text.", 12, "helv")])
    _write_pdf(OUT / "multi-heading.pdf", [("H1", 28, "helv"), ("H2", 22, "helv"), ("Body", 12, "helv")])
    _write_pdf(OUT / "bold-italic.pdf", [("Bold", 14, "helvb"), ("Italic", 14, "helvi")])

    good = (OUT / "simple.pdf").read_bytes()
    (OUT / "corrupted.pdf").write_bytes(good[: max(100, len(good) // 4)])

if __name__ == "__main__":
    main()
```

Run generator:

```bash
python3 tests/fixtures/generate_fixtures.py
```

**Step 4: Run test to verify it passes**

Run: `python3 -m pytest tests/fixtures/test_generate_fixtures.py -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add tests/fixtures/generate_fixtures.py tests/fixtures/test_generate_fixtures.py tests/fixtures/simple.pdf tests/fixtures/multi-heading.pdf tests/fixtures/bold-italic.pdf tests/fixtures/corrupted.pdf
git commit -m "test: add deterministic phase-1 pdf fixtures"
```

### Task 5: PyMuPDF Oracle Extractor + Dependency Pinning

**Files:**
- Create: `tests/oracle/extract_oracle.py`
- Create: `tests/oracle/requirements.txt`
- Create: `tests/oracle/test_extract_oracle.py`

**Step 1: Write the failing test**

```python
# tests/oracle/test_extract_oracle.py
import json
import subprocess
from pathlib import Path

def test_oracle_emits_pages_and_blocks(tmp_path):
    pdf = Path("tests/fixtures/simple.pdf")
    out = tmp_path / "simple.oracle.json"
    subprocess.run(
        ["python3", "tests/oracle/extract_oracle.py", str(pdf), "--out", str(out)],
        check=True,
    )
    data = json.loads(out.read_text())
    assert "pages" in data
    assert isinstance(data["pages"], list)
```

**Step 2: Run test to verify it fails**

Run: `python3 -m pytest tests/oracle/test_extract_oracle.py::test_oracle_emits_pages_and_blocks -q`
Expected: FAIL because extractor script does not exist yet.

**Step 3: Write minimal implementation**

```txt
# tests/oracle/requirements.txt
PyMuPDF==1.24.10
pytest==8.3.2
```

```python
# tests/oracle/extract_oracle.py
import argparse
import json
from pathlib import Path
import fitz

def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("pdf", type=Path)
    p.add_argument("--out", type=Path, required=True)
    return p.parse_args()

def block_to_record(span):
    text = span.get("text", "").strip()
    if not text:
        return None
    font_name = span.get("font")
    flags = span.get("flags", 0)
    return {
        "text": text,
        "font_name": font_name,
        "font_size": float(span.get("size", 0.0)),
        "is_bold": bool(flags & (1 << 4)),
        "is_italic": bool(flags & (1 << 1)),
    }

def extract(pdf_path: Path):
    doc = fitz.open(pdf_path)
    pages = []
    for i, page in enumerate(doc):
        page_blocks = []
        for block in page.get_text("dict").get("blocks", []):
            for line in block.get("lines", []):
                for span in line.get("spans", []):
                    record = block_to_record(span)
                    if record:
                        page_blocks.append(record)
        pages.append({"page_number": i, "blocks": page_blocks})
    doc.close()
    return {"pages": pages}

def main():
    args = parse_args()
    payload = extract(args.pdf)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")

if __name__ == "__main__":
    main()
```

**Step 4: Run test to verify it passes**

Run: `python3 -m pytest tests/oracle/test_extract_oracle.py::test_oracle_emits_pages_and_blocks -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add tests/oracle/extract_oracle.py tests/oracle/requirements.txt tests/oracle/test_extract_oracle.py
git commit -m "test: add pymupdf oracle extractor and tests"
```

### Task 6: Oracle Baselines for Core Fixtures

**Files:**
- Create: `tests/fixtures/simple.oracle.json`
- Create: `tests/fixtures/multi-heading.oracle.json`
- Create: `tests/fixtures/bold-italic.oracle.json`
- Create: `tests/oracle/test_baselines.py`

**Step 1: Write the failing test**

```python
# tests/oracle/test_baselines.py
import json
from pathlib import Path

def test_baseline_files_exist_and_have_pages():
    fixtures = [
        "simple.oracle.json",
        "multi-heading.oracle.json",
        "bold-italic.oracle.json",
    ]
    for name in fixtures:
        path = Path("tests/fixtures") / name
        assert path.exists(), f"missing baseline: {name}"
        data = json.loads(path.read_text())
        assert "pages" in data
```

**Step 2: Run test to verify it fails**

Run: `python3 -m pytest tests/oracle/test_baselines.py -q`
Expected: FAIL with missing baseline file assertions.

**Step 3: Write minimal implementation**

Generate baselines:

```bash
python3 tests/oracle/extract_oracle.py tests/fixtures/simple.pdf --out tests/fixtures/simple.oracle.json
python3 tests/oracle/extract_oracle.py tests/fixtures/multi-heading.pdf --out tests/fixtures/multi-heading.oracle.json
python3 tests/oracle/extract_oracle.py tests/fixtures/bold-italic.pdf --out tests/fixtures/bold-italic.oracle.json
```

**Step 4: Run test to verify it passes**

Run: `python3 -m pytest tests/oracle/test_baselines.py -q`
Expected: PASS.

**Step 5: Commit**

```bash
git add tests/fixtures/simple.oracle.json tests/fixtures/multi-heading.oracle.json tests/fixtures/bold-italic.oracle.json tests/oracle/test_baselines.py
git commit -m "test: add oracle baseline json fixtures"
```

### Task 7: Phase-1 Verification Gate

**Files:**
- Modify: `papyrus-core/src/ast/mod.rs` (add one more assertion-level test if needed)
- Create: `docs/plans/phase-1-verification-log.md`

**Step 1: Write the failing test**

```rust
// add in papyrus-core/src/ast/mod.rs tests
#[test]
fn raw_text_variant_round_trips() {
    let node = Node::RawText("unclassified".to_string());
    match node {
        Node::RawText(s) => assert_eq!(s, "unclassified"),
        _ => panic!("expected raw text"),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p papyrus-core raw_text_variant_round_trips -v`
Expected: FAIL before test exists, then PASS after implementation (this is the final sanity gate test).

**Step 3: Write minimal implementation**

No new implementation needed if AST is correct; only add missing AST logic if this test reveals a regression.

Create verification log:

```md
# Phase 1 Verification Log

- cargo build --workspace
- cargo test --workspace
- python3 -m pytest tests/fixtures tests/oracle -q
- python3 tests/oracle/extract_oracle.py tests/fixtures/simple.pdf --out /tmp/simple.oracle.json
```

**Step 4: Run test to verify it passes**

Run:
- `cargo build --workspace`
- `cargo test --workspace`
- `python3 -m pytest tests/fixtures tests/oracle -q`

Expected:
- All commands PASS.
- CLI prints `papyrus-cli: not yet implemented` when run.

**Step 5: Commit**

```bash
git add papyrus-core/src/ast/mod.rs docs/plans/phase-1-verification-log.md
git commit -m "chore: finalize phase-1 verification gate"
```

---

## Done Checklist

- [ ] Workspace crates compile with `cargo build --workspace`
- [ ] AST types and warnings match design contract and are tested
- [ ] Parser/detector/renderer stubs exist and are link-tested
- [ ] PDF fixtures exist (`simple`, `multi-heading`, `bold-italic`, `corrupted`)
- [ ] PyMuPDF oracle script works with pinned dependencies
- [ ] Oracle baseline JSON files are committed
- [ ] Rust + Python test suites pass in one clean run
- [ ] Verification log is committed

