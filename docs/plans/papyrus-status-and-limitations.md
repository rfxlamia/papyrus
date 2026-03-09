# Papyrus: Current Status and Known Limitations

**Last Updated:** 2026-03-09  
**Version:** 0.1.1 (dev — unreleased)

## Executive Summary

Papyrus is a Rust-based PDF-to-Markdown converter that successfully extracts text content from PDFs and converts it to readable Markdown format. The project has completed all 5 planned phases and includes a fully functional CLI. However, as a young project, it has known limitations compared to mature PDF manipulation libraries like PyMuPDF, pdfplumber, or Adobe's PDF libraries.

## What We've Accomplished

### Phase 1: Scaffold and Oracle (Completed)
- ✅ Project structure with `papyrus-core` and `papyrus-cli` crates
- ✅ Oracle-based testing infrastructure using PyMuPDF as ground truth
- ✅ Test fixtures for simple, multi-page, bold/italic, and corrupted PDFs
- ✅ Comprehensive test coverage (131 tests passing)

### Phase 2: Low-Level Extraction (Completed)
- ✅ PDF loading and validation with error handling
- ✅ Font resolution and descriptor metrics extraction (FontWeight, ItalicAngle)
- ✅ Text encoding support:
  - UTF-16BE with and without BOM
  - WinAnsiEncoding (PDF spec §D.1)
  - Automatic encoding detection
- ✅ Content stream parsing (Tf, Tj, TJ, BT, ET operators)
- ✅ Text state machine with proper font tracking
- ✅ Word spacing detection in TJ arrays using positioning heuristics

### Phase 3: Smart Outline and API (Completed)
- ✅ Heading detection using font-size ratios (4 levels)
- ✅ Bold/italic detection from font names and descriptors
- ✅ Body text classification
- ✅ AST-based document representation
- ✅ Configurable detection thresholds
- ✅ Public API with builder pattern

### Phase 4: CommonMark Renderer (Completed)
- ✅ Markdown output with proper escaping
- ✅ Heading levels (# through ######)
- ✅ Bold (**text**) and italic (*text*) formatting
- ✅ Combined formatting (***text***)
- ✅ Special character escaping for CommonMark compliance
- ✅ HTML entity prevention (<, >, &)
- ✅ Single trailing newline normalization

### Phase 5: CLI Interface (Completed)
- ✅ Single-file conversion (file or stdout)
- ✅ Batch directory conversion with progress indicators
- ✅ Stdin/stdout pipeline support (explicit `-` flag)
- ✅ Colored warning output with `--quiet` flag
- ✅ Configurable flags: `--heading-ratio`, `--no-bold`, `--no-italic`
- ✅ Correct exit codes (0=success, 1=I/O error, 2=invalid args)
- ✅ 22 CLI tests (14 unit + 8 integration)

### Recent Bug Fixes
- ✅ **Critical:** Fixed word spacing in TJ operator extraction (2026-03-09)
  - Previously all spaces were removed, making output unreadable
  - Now uses positioning heuristics to detect word boundaries
  - Tested successfully on "Attention Is All You Need" paper (2.2MB, 13 pages)

### Position-Aware Extraction (v0.1.1 work, 2026-03-09)
- ✅ **TextState machine** — tracks PDF text matrix (`Tm`, `Td`, `TD`, `T*`, `TL` operators)
- ✅ **Absolute coordinates** — every `RawTextSegment` now carries `x`, `y`, `is_rotated`
- ✅ **Spatial layout module** (`papyrus-core/src/layout/`) with:
  - Y-grouping: segments within `font_size * 0.5` tolerance merged into same line
  - X-gap word spacing: space inserted when gap exceeds `font_size * 0.3 * 0.8`
  - Paragraph detection: blank segment inserted when Y-gap > `median_line_height * 1.5`
  - Rotated text quarantine: flagged as `Warning::RotatedTextDetected`
- ✅ **Image-only page detection**: empty pages emit `Warning::ImageOnlyPage`
- ✅ **Pipeline integration**: layout runs between raw extraction and AST build
- ✅ **Hypothesis verification**: H1 (Tm coordinates plausible) and H6 (X-cursor advancement) confirmed with integration tests

## Known Limitations

### 1. Text Extraction Quality

#### Word Spacing Heuristics
**Status:** Partially solved — improved in v0.1.1 but still imperfect

Word spacing now uses two mechanisms:
1. **TJ array**: displacement threshold of `font_size * 0.3` to detect intentional gaps
2. **X-gap analysis**: post-layout check — inserts space between segments when X-gap exceeds `space_width * 0.8`

Remaining issues:
- **Junction gaps**: when a PDF line wraps, the last word of line N and first word of line N+1 lose the space between them (e.g., `orconvolutional` instead of `or convolutional`). Root cause: the PDF stores each visual line as a separate text object; word boundary at the junction is not encoded in the TJ/Tj data.
- **Impact:** Multi-line paragraphs in academic papers show missing spaces at line boundaries — still readable but not clean
- **Fix planned:** v0.1.2 — use actual font metrics (character advance widths from the font dictionary) to estimate end-X of each word more precisely, closing junction gaps

#### Line and Paragraph Detection
**Status:** Implemented in v0.1.1, partially working

- ✅ **Section breaks** between heading and body: working (headings and body text on separate lines)
- ✅ **Paragraph breaks** within body text: working for large Y-gaps
- ⚠️ **Within-paragraph line wraps**: body text lines that are wrapped (close Y-gap) are concatenated into one paragraph block, but word boundary at the wrap junction is missing a space
- ❌ **Multi-column layouts**: reading order not detected — text from adjacent columns can interleave

### 2. Layout Analysis

#### Tables
**Status:** Not supported

- **Issue:** Tables are extracted as plain text without structure
- **Impact:** Tabular data loses its meaning
- **Comparison:** PyMuPDF and pdfplumber can:
  - Detect table boundaries
  - Extract cell contents
  - Preserve row/column structure
  - Export to CSV or structured formats

#### Lists
**Status:** Not supported

- **Issue:** Bulleted and numbered lists are not detected
- **Impact:** Lists appear as plain paragraphs
- **Comparison:** Mature libraries detect list markers and indentation

#### Multi-Column Layouts
**Status:** Not supported

- **Issue:** No reading order detection for multi-column documents
- **Impact:** Text from different columns may be interleaved
- **Comparison:** PyMuPDF uses spatial analysis to determine reading order

### 3. Mathematical Notation

**Status:** Not supported

- **Issue:** Mathematical formulas are extracted as escaped text
- **Impact:** Equations are unreadable (e.g., `*x*1*;:::;x**n*` instead of proper LaTeX)
- **Comparison:** Specialized tools like pdf2latex can:
  - Detect mathematical notation
  - Convert to LaTeX format
  - Preserve equation structure

### 4. Images and Figures

**Status:** Not supported

- **Issue:** Images are completely ignored
- **Impact:** Figure captions are extracted but figures themselves are lost
- **Comparison:** PyMuPDF can:
  - Extract embedded images
  - Save images to files
  - Provide image metadata (dimensions, format, DPI)

### 5. Metadata Extraction

**Status:** Basic support only

Currently supported:
- ✅ Title (from /Info dictionary)
- ✅ Author (from /Info dictionary)
- ✅ Page count

Not supported:
- ❌ Subject, Keywords, Creator, Producer
- ❌ Creation/modification dates
- ❌ PDF version
- ❌ Encryption status
- ❌ Custom metadata fields

**Comparison:** PyMuPDF provides comprehensive metadata access

### 6. Font Handling

#### Font Embedding
**Status:** Limited support

- **Issue:** We rely on font names and descriptors, not actual font data
- **Impact:** Custom or embedded fonts may not be handled correctly
- **Comparison:** PyMuPDF can:
  - Extract embedded font data
  - Use font metrics for accurate text positioning
  - Handle CIDFonts and composite fonts

#### Character Encoding
**Status:** Basic support

Currently supported:
- ✅ UTF-16BE (with/without BOM)
- ✅ WinAnsiEncoding
- ✅ Basic ASCII

Not supported:
- ❌ MacRomanEncoding
- ❌ PDFDocEncoding
- ❌ Custom encodings
- ❌ CMap-based encodings for CJK fonts

### 7. PDF Features

#### Not Supported
- ❌ Annotations and comments
- ❌ Form fields
- ❌ Bookmarks/outlines
- ❌ Hyperlinks (internal and external)
- ❌ Attachments
- ❌ Digital signatures
- ❌ Encrypted/password-protected PDFs
- ❌ PDF/A compliance checking
- ❌ Layers (Optional Content Groups)

### 8. Performance

**Status:** Good for small-to-medium PDFs, untested for large documents

- **Tested:** Successfully converted 2.2MB, 13-page academic paper
- **Unknown:** Performance on 100+ page documents
- **Comparison:** PyMuPDF is highly optimized with C++ backend
- **Consideration:** Our Rust implementation should be reasonably fast, but lacks years of optimization

### 9. Error Recovery

**Status:** Basic error handling

- **Current:** Graceful degradation with warnings
- **Issue:** Some malformed PDFs may fail completely
- **Comparison:** Mature libraries have extensive error recovery from years of encountering edge cases

## Comparison Matrix

| Feature | Papyrus | PyMuPDF | pdfplumber |
|---------|---------|---------|------------|
| Basic text extraction | ✅ Good | ✅ Excellent | ✅ Excellent |
| Word spacing | ⚠️ Heuristic (junction gaps) | ✅ Accurate | ✅ Accurate |
| Line break detection | ✅ Yes (v0.1.1) | ✅ Yes | ✅ Yes |
| Paragraph detection | ⚠️ Partial (v0.1.1) | ✅ Yes | ✅ Yes |
| Heading detection | ✅ Yes | ❌ No | ❌ No |
| Bold/italic detection | ✅ Yes | ✅ Yes | ⚠️ Limited |
| Markdown output | ✅ Yes | ❌ No | ❌ No |
| Table extraction | ❌ No | ✅ Yes | ✅ Excellent |
| Image extraction | ❌ No | ✅ Yes | ✅ Yes |
| Multi-column layout | ❌ No | ✅ Yes | ✅ Yes |
| Math formulas | ❌ No | ❌ No | ❌ No |
| Metadata | ⚠️ Basic | ✅ Complete | ✅ Complete |
| Position tracking | ✅ Yes (v0.1.1) | ✅ Yes | ✅ Yes |
| Error recovery | ⚠️ Basic | ✅ Excellent | ✅ Good |

## Use Cases

### Where Papyrus Excels
1. **Simple text extraction** from well-formatted PDFs
2. **Markdown conversion** for documentation or note-taking
3. **Heading detection** for document structure analysis
4. **Rust ecosystem integration** for projects that need PDF parsing
5. **Learning tool** for understanding PDF structure

### Where to Use Alternatives
1. **Table extraction** → Use pdfplumber or tabula-py
2. **Image extraction** → Use PyMuPDF or pdfimages
3. **Complex layouts** → Use PyMuPDF with layout analysis
4. **Mathematical papers** → Use pdf2latex or mathpix
5. **Production systems** → Use mature, battle-tested libraries
6. **Form processing** → Use PyMuPDF or pdftk
7. **PDF manipulation** (merge, split, etc.) → Use PyPDF2 or PyMuPDF

## Future Improvement Opportunities

### High Priority
1. **Fix junction word spacing** (v0.1.2)
   - Use actual character advance widths from font dictionary
   - Eliminate missing spaces at PDF line-wrap boundaries
   - Implement per-font space width metrics

2. **Multi-column layout** (v0.2.0)
   - Detect column boundaries from X-position clustering
   - Reconstruct correct reading order
   - Handle academic 2-column papers

3. **Table detection**
   - Identify grid structures
   - Extract cell contents
   - Preserve table structure in Markdown

### Medium Priority
4. **Image extraction**
   - Extract embedded images
   - Generate image references in Markdown
   - Support common image formats (JPEG, PNG)

5. **Hyperlink preservation**
   - Extract URL annotations
   - Convert to Markdown links
   - Preserve internal references

6. **Better font handling**
   - Support more encodings
   - Handle CIDFonts properly
   - Use actual font metrics

### Low Priority
7. **Metadata expansion**
   - Extract all standard metadata fields
   - Support custom metadata
   - Export metadata to YAML frontmatter

8. **Performance optimization**
   - Benchmark large documents
   - Optimize memory usage
   - Add streaming support for huge PDFs

## Testing Strategy

### Current Coverage
- ✅ 110 tests passing
- ✅ Unit tests for all core modules
- ✅ Integration tests with real PDFs
- ✅ Oracle-based validation against PyMuPDF
- ✅ CLI integration tests

### Gaps
- ❌ No performance benchmarks
- ❌ Limited real-world PDF corpus
- ❌ No fuzzing or property-based testing
- ❌ No comparison tests against other libraries

## Conclusion

Papyrus is a functional PDF-to-Markdown converter that works well for its intended use case: extracting text from simple, well-formatted PDFs and converting to Markdown with heading detection. However, it is not a replacement for mature PDF libraries like PyMuPDF when you need:

- Accurate layout analysis
- Table extraction
- Image handling
- Complex font support
- Production-grade reliability

The project serves as:
1. A **learning tool** for understanding PDF structure
2. A **Rust-native option** for basic PDF text extraction
3. A **foundation** for future enhancements
4. A **specialized tool** for Markdown conversion with heading detection

For production use cases requiring robust PDF handling, we recommend using established libraries like PyMuPDF, pdfplumber, or commercial solutions.

## References

- [PDF 1.7 Specification](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf)
- [PyMuPDF Documentation](https://pymupdf.readthedocs.io/)
- [pdfplumber Documentation](https://github.com/jsvine/pdfplumber)
- [CommonMark Specification](https://spec.commonmark.org/)
