# Test Fixtures

This directory contains PDF test fixtures used by papyrus integration tests.

## Required Files

The following PDF files are required for tests to pass:

- `simple.pdf` - Single-page PDF with heading and body text
- `multi-page.pdf` - Multi-page PDF for testing page iteration
- `bold-italic.pdf` - PDF with bold and italic text for style detection tests
- `corrupted.pdf` - Malformed PDF for error handling tests
- `multi-heading.pdf` - PDF with multiple heading levels

## Generating Fixtures

Fixtures can be regenerated using the Python script:

```bash
cd tests/fixtures
python generate_fixtures.py
```

This requires the `reportlab` library:

```bash
pip install reportlab
```

## Oracle Files

Each PDF fixture has a corresponding `.oracle.json` file containing the expected extraction output. These are used for regression testing to ensure extraction behavior remains consistent.
