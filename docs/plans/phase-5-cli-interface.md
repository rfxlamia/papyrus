# Phase 5: CLI Interface (The Hands)

**Goal**: Build the `papyrus-cli` binary with single file conversion, batch mode, pipe support, and colored warning output.

**Depends on**: Phase 4 (CommonMark Renderer)

---

## Tasks

### 5.1 Implement CLI Argument Parsing

Using `clap` with derive macros, define the CLI structure:

```rust
#[derive(Parser)]
#[command(name = "papyrus", about = "PDF to Markdown conversion engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert PDF to Markdown
    Convert {
        /// Input PDF file or directory
        input: PathBuf,

        /// Output file or directory (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Heading size ratio threshold (default: 1.2)
        #[arg(long, default_value_t = 1.2)]
        heading_ratio: f32,

        /// Disable bold detection
        #[arg(long)]
        no_bold: bool,

        /// Disable italic detection
        #[arg(long)]
        no_italic: bool,
    },
}
```

### 5.2 Implement Single File Conversion

When input is a file path:

1. Read PDF bytes from disk
2. Build `Papyrus` with CLI-provided config
3. Call `extract()` then `to_markdown()`
4. Write Markdown to output path (or stdout if no `-o` flag)
5. Print warnings to stderr using colored output

Error handling:
- File not found -> print error to stderr, exit code 1
- Permission denied -> print error to stderr, exit code 1
- PDF conversion warnings -> print to stderr, still exit code 0

### 5.3 Implement Batch Directory Conversion

When input is a directory path:

1. Scan directory for all `*.pdf` files (non-recursive by default)
2. If `-o` is a directory: map each `input/foo.pdf` -> `output/foo.md`
3. If `-o` is not specified: create `.md` files alongside `.pdf` files
4. Show progress bar using `indicatif` (`[3/10] Converting report.pdf...`)
5. Collect warnings per file, print summary at end

Error handling:
- If output directory doesn't exist, create it
- If a single file fails, log warning and continue with remaining files
- Exit code 0 if at least one file converted, exit code 1 if all failed

### 5.4 Implement Pipe Mode (stdin/stdout)

When input is `-` or stdin is not a terminal:

1. Read all bytes from stdin
2. Convert and write Markdown to stdout
3. Warnings to stderr

This enables Unix pipeline usage: `cat doc.pdf | papyrus convert - > doc.md`

### 5.5 Implement Warning Renderer

Create a warning formatter that produces colored terminal output:

```
⚠ Warning: Missing font metrics for "ComicSans" on page 3
⚠ Warning: Unreadable text stream on page 7 (invalid operator sequence)
```

Using `owo-colors`:
- Warning prefix in yellow
- File/page reference in cyan
- Detail text in default color

Provide a `--quiet` flag that suppresses warnings.

### 5.6 Exit Code Implementation

| Code | Condition |
|------|-----------|
| `0` | Success (with or without warnings) |
| `1` | Fatal I/O error (file not found, permission, all files failed in batch) |
| `2` | Invalid arguments (handled by `clap` automatically) |

### 5.7 Integration Tests for CLI

Using `assert_cmd` and `predicates` crates (dev-dependencies):

- Single file: `papyrus convert tests/fixtures/simple.pdf -o /tmp/out.md` -> exit 0, file created
- Stdout mode: `papyrus convert tests/fixtures/simple.pdf` -> exit 0, Markdown on stdout
- Batch mode: `papyrus convert tests/fixtures/ -o /tmp/out/` -> exit 0, multiple `.md` files
- Invalid input: `papyrus convert nonexistent.pdf` -> exit 1, error on stderr
- Custom flags: `papyrus convert simple.pdf --heading-ratio 2.0 --no-bold` -> exit 0, modified output
- Pipe mode: `cat simple.pdf | papyrus convert -` -> exit 0, Markdown on stdout
- Warning output: `papyrus convert corrupted.pdf` -> exit 0, warnings on stderr

**Dev dependencies added to `papyrus-cli`:**
- `assert_cmd`
- `predicates`

---

## Definition of Done

- [ ] `papyrus convert <file> -o <output>` works for single file conversion
- [ ] `papyrus convert <dir> -o <dir>` works for batch conversion with progress bar
- [ ] Pipe mode works (`stdin` -> `stdout`)
- [ ] Warnings render in color to stderr
- [ ] `--quiet` flag suppresses warnings
- [ ] `--heading-ratio`, `--no-bold`, `--no-italic` flags work and pass through to `PapyrusBuilder`
- [ ] Exit codes: 0 for success, 1 for fatal I/O, 2 for bad arguments
- [ ] Integration tests cover all CLI modes
- [ ] `cargo test` passes with all new tests green
- [ ] `cargo build --release` produces a single working binary
