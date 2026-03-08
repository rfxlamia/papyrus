# Papyrus: Current Status and Known Limitations

**Last Updated:** 2026-03-09  
**Version:** 0.1.0

## Executive Summary

Papyrus is a Rust-based PDF-to-Markdown converter that successfully extracts text content from PDFs and converts it to readable Markdown format. The project has completed all 5 planned phases and includes a fully functional CLI. However, as a young project, it has known limitations compared to mature PDF manipulation libraries like PyMuPDF, pdfplumber, or Adobe's PDF libraries.

## What We've Accomplished

### Phase 1: Scaffold and Oracle (Completed)
- ✅ Project structure with `papyrus-core` and `papyrus-cli` crates
- ✅ Oracle-based testing infrastructure using PyMuPDF as ground truth
- ✅ Test fixtures for simple, multi-page, bold/italic, and corrupted PDFs
- ✅ Comprehensive test coverage (110 tests passing)

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

## Known Limitations

### 1. Text Extraction Quality

#### Word Spacing Heuristics
**Status:** Partially solved, but not perfect

Our current approach uses a threshold of `-100` for positioning adjustments in TJ arrays to detect word boundaries. This works well for many PDFs but has limitations:

- **Issue:** The threshold is a heuristic that may not work for all PDF generators
- **Impact:** Some PDFs may have missing spaces or extra spaces
- **Comparison:** PyMuPDF uses more sophisticated layout analysis and glyph positioning
- **Example:** Complex layouts with multiple columns or unusual kerning may not extract perfectly

**Why PyMuPDF is better:**
- Uses actual glyph bounding boxes and spatial analysis
- Considers font metrics and character widths
- Has years of refinement across thousands of PDF variations

#### Line and Paragraph Detection
**Status:** Not implemented

- **Issue:** We don't detect line breaks or paragraph boundaries
- **Impact:** All text within a heading or body section is concatenated
- **Comparison:** PyMuPDF can detect:
  - Line breaks based on vertical positioning
  - Paragraph boundaries using spacing analysis
  - Column layouts and reading order
- **Example:** Multi-column academic papers may have text from different columns mixed together

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
| Word spacing | ⚠️ Heuristic | ✅ Accurate | ✅ Accurate |
| Heading detection | ✅ Yes | ❌ No | ❌ No |
| Bold/italic detection | ✅ Yes | ✅ Yes | ⚠️ Limited |
| Markdown output | ✅ Yes | ❌ No | ❌ No |
| Table extraction | ❌ No | ✅ Yes | ✅ Excellent |
| Image extraction | ❌ No | ✅ Yes | ✅ Yes |
| Layout analysis | ❌ No | ✅ Yes | ✅ Yes |
| Math formulas | ❌ No | ❌ No | ❌ No |
| Metadata | ⚠️ Basic | ✅ Complete | ✅ Complete |
| Performance | ⚠️ Untested | ✅ Excellent | ⚠️ Good |
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
1. **Improve word spacing detection**
   - Analyze glyph bounding boxes
   - Use font metrics for character widths
   - Implement adaptive thresholds per font

2. **Line break detection**
   - Track vertical positioning
   - Detect paragraph boundaries
   - Handle multi-column layouts

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
