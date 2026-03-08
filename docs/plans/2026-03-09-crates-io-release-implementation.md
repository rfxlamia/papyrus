# Papyrus crates.io Release Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Release papyrus-core and papyrus-cli to crates.io with proper licensing and metadata.

**Architecture:** 2-phase release where papyrus-core (library) is published first, then papyrus-cli (binary) is updated to use the published version and published second.

**Tech Stack:** Rust, Cargo, crates.io, Git

---

### Task 1: Create LICENSE-MIT File

**Files:**
- Create: `LICENSE-MIT`

**Step 1: Write MIT License**

Content for LICENSE-MIT:
```
MIT License

Copyright (c) 2026 rfxlamia

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

**Step 2: Verify file created**

Run: `cat LICENSE-MIT`
Expected: License text displayed

**Step 3: Commit**

```bash
git add LICENSE-MIT
git commit -m "chore: add MIT license"
```

---

### Task 2: Create LICENSE-APACHE File

**Files:**
- Create: `LICENSE-APACHE`

**Step 1: Download Apache 2.0 License**

Run: `curl -L https://www.apache.org/licenses/LICENSE-2.0.txt -o LICENSE-APACHE`

**Step 2: Verify file created**

Run: `head -10 LICENSE-APACHE`
Expected: "Apache License Version 2.0, January 2004"

**Step 3: Commit**

```bash
git add LICENSE-APACHE
git commit -m "chore: add Apache 2.0 license"
```

---

### Task 3: Update papyrus-core Cargo.toml with Metadata

**Files:**
- Modify: `papyrus-core/Cargo.toml`

**Step 1: Read current Cargo.toml**

Run: `cat papyrus-core/Cargo.toml`

Current content:
```toml
[package]
name = "papyrus-core"
version = "0.1.0"
edition = "2021"

[dependencies]
lopdf = "0.35"
```

**Step 2: Update with metadata**

Replace content with:
```toml
[package]
name = "papyrus-core"
version = "0.1.0"
edition = "2021"
authors = ["rfxlamia"]
description = "PDF-to-Markdown conversion engine with smart heading detection, bold/italic text extraction, and CommonMark output. Pure Rust, best-effort parsing for corrupted PDFs."
repository = "https://github.com/rfxlamia/papyrus"
license = "MIT OR Apache-2.0"
keywords = ["pdf", "markdown", "extract", "convert", "text-extraction"]
categories = ["text-processing", "parser-implementations", "encoding"]

[dependencies]
lopdf = "0.35"

[dev-dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
pulldown-cmark = "0.10"
```

**Step 3: Verify TOML is valid**

Run: `cargo check -p papyrus-core`
Expected: No errors, possibly some warnings

**Step 4: Commit**

```bash
git add papyrus-core/Cargo.toml
git commit -m "chore(core): add crates.io metadata

- Add author, description, repository, license
- Add keywords and categories for discoverability"
```

---

### Task 4: Verify papyrus-core Builds and Tests Pass

**Files:** None (verification only)

**Step 1: Clean build**

Run: `cargo clean`

**Step 2: Build papyrus-core**

Run: `cargo build --release -p papyrus-core`
Expected: Successful build

**Step 3: Run tests**

Run: `cargo test -p papyrus-core`
Expected: All tests pass (110 tests)

**Step 4: Dry-run publish**

Run: `cargo publish -p papyrus-core --dry-run`
Expected: "Uploaded" message (dry run), no packaging errors

---

### Task 5: Publish papyrus-core to crates.io

**Prerequisites:**
- crates.io account exists
- Logged in via `cargo login`

**Step 1: Login to crates.io**

If not already logged in:
Run: `cargo login`
Prompt: Enter API token from crates.io

**Step 2: Publish papyrus-core**

Run: `cargo publish -p papyrus-core`
Expected: Upload progress, "Uploaded papyrus-core v0.1.0"

**Step 3: Verify on crates.io**

Open: https://crates.io/crates/papyrus-core
Verify: Version 0.1.0 is visible

**Step 4: Wait for indexing**

Wait 30-60 seconds for crates.io to index the crate.

---

### Task 6: Update papyrus-cli Cargo.toml with Metadata and Version Dependency

**Files:**
- Modify: `papyrus-cli/Cargo.toml`

**Step 1: Read current Cargo.toml**

Run: `cat papyrus-cli/Cargo.toml`

Current content:
```toml
[package]
name = "papyrus-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "papyrus"
path = "src/main.rs"

[dependencies]
papyrus-core = { path = "../papyrus-core" }
clap = { version = "4.5", features = ["derive"] }
indicatif = "0.17"
owo-colors = "4.1"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3.13"
```

**Step 2: Update with metadata and version dependency**

Replace content with:
```toml
[package]
name = "papyrus-cli"
version = "0.1.0"
edition = "2021"
authors = ["rfxlamia"]
description = "Command-line tool for PDF-to-Markdown conversion with smart heading detection, bold/italic extraction, and CommonMark output. Pure Rust, pipe-friendly."
repository = "https://github.com/rfxlamia/papyrus"
license = "MIT OR Apache-2.0"
keywords = ["pdf", "markdown", "cli", "convert", "extract"]
categories = ["command-line-utilities", "text-processing"]

[[bin]]
name = "papyrus"
path = "src/main.rs"

[dependencies]
papyrus-core = "0.1.0"
clap = { version = "4.5", features = ["derive"] }
indicatif = "0.17"
owo-colors = "4.1"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3.13"
```

**Step 3: Verify TOML is valid and can fetch dependency**

Run: `cargo check -p papyrus-cli`
Expected: No errors, downloads papyrus-core from crates.io

**Step 4: Commit**

```bash
git add papyrus-cli/Cargo.toml
git commit -m "chore(cli): add crates.io metadata and use published core

- Add author, description, repository, license
- Add keywords and categories for discoverability
- Switch from path dependency to version dependency"
```

---

### Task 7: Verify papyrus-cli Builds and Tests Pass

**Files:** None (verification only)

**Step 1: Clean build**

Run: `cargo clean`

**Step 2: Build papyrus-cli**

Run: `cargo build --release -p papyrus-cli`
Expected: Successful build, uses papyrus-core from crates.io

**Step 3: Run tests**

Run: `cargo test -p papyrus-cli`
Expected: All tests pass (22 tests)

**Step 4: Test CLI functionality**

Run: `./target/release/papyrus --help`
Expected: Help text displayed

Run: `./target/release/papyrus convert tests/fixtures/simple.pdf -o /tmp/test.md`
Expected: Conversion succeeds, markdown output

**Step 5: Dry-run publish**

Run: `cargo publish -p papyrus-cli --dry-run`
Expected: "Uploaded" message (dry run), no packaging errors

---

### Task 8: Publish papyrus-cli to crates.io

**Files:** None (publish action)

**Step 1: Publish papyrus-cli**

Run: `cargo publish -p papyrus-cli`
Expected: Upload progress, "Uploaded papyrus-cli v0.1.0"

**Step 2: Verify on crates.io**

Open: https://crates.io/crates/papyrus-cli
Verify: Version 0.1.0 is visible

**Step 3: Test install**

Run: `cargo install papyrus-cli`
Expected: Installs successfully

Run: `~/.cargo/bin/papyrus --version` (or wherever cargo installs)
Expected: Version displayed

---

### Task 9: Create Git Tag v0.1.0

**Files:** None (git action)

**Step 1: Verify clean working directory**

Run: `git status`
Expected: "nothing to commit, working tree clean"

**Step 2: Create annotated tag**

Run: `git tag -a v0.1.0 -m "Release v0.1.0"`

**Step 3: Push tag to origin**

Run: `git push origin v0.1.0`
Expected: Tag pushed to remote

**Step 4: Verify tag exists**

Run: `git tag -l`
Expected: "v0.1.0" listed

---

### Task 10: Create GitHub Release

**Files:** None (GitHub UI action)

**Step 1: Navigate to GitHub releases**

Open: https://github.com/rfxlamia/papyrus/releases/new
Tag: Select "v0.1.0"

**Step 2: Fill release title**

Title: `Papyrus v0.1.0`

**Step 3: Fill release notes**

Content:
```markdown
## Papyrus v0.1.0

First release of Papyrus PDF-to-Markdown converter.

### Crates
- `papyrus-core` v0.1.0 - https://crates.io/crates/papyrus-core
- `papyrus-cli` v0.1.0 - https://crates.io/crates/papyrus-cli

### Install
```bash
cargo install papyrus-cli
```

### Features
- Smart heading detection (H1-H4) from font sizes
- Bold and italic text detection from font names/descriptors
- CommonMark Markdown output
- Pipe-friendly CLI (stdin/stdout support)
- Batch conversion with progress bars
- Best-effort parsing with warning reports
- Pure Rust, no C dependencies

### Documentation
- [README](https://github.com/rfxlamia/papyrus#readme)
- [Contributing Guide](https://github.com/rfxlamia/papyrus/blob/main/CONTRIBUTING.md)
```

**Step 4: Publish release**

Click "Publish release"

**Step 5: Verify release is public**

Open: https://github.com/rfxlamia/papyrus/releases
Expected: v0.1.0 release visible

---

## Summary

After completing all tasks:

1. ✅ `papyrus-core` v0.1.0 on crates.io
2. ✅ `papyrus-cli` v0.1.0 on crates.io
3. ✅ Dual license files (MIT + Apache-2.0) in repository
4. ✅ GitHub release tag v0.1.0
5. ✅ Users can `cargo install papyrus-cli`

### Post-Release Verification Commands

```bash
# Verify crates are installable
cargo install papyrus-cli

# Verify both crates appear on crates.io
curl -s https://crates.io/api/v1/crates/papyrus-core | head -c 500
curl -s https://crates.io/api/v1/crates/papyrus-cli | head -c 500

# Verify tag exists
git ls-remote --tags origin | grep v0.1.0
```
