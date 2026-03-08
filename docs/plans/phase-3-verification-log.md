# Phase 3 Verification Log

- **Date:** 2026-03-08
- **Branch:** `feature/phase-3-smart-outline`
- **Base commit:** `3ca59c0` (chore: complete phase-2 low-level extraction implementation)
- **HEAD commit:** `adac78b` (test(core): add phase-3 extraction pipeline integration coverage)

## Commands run

```
cargo test -p papyrus-core module_surfaces_are_linked -v
cargo test -p papyrus-core parse_pdf -- --nocapture
cargo test -p papyrus-core detect_ -- --nocapture
cargo test -p papyrus-core --test integration_extraction -v
cargo test -p papyrus-core --test integration_phase3_pipeline -v
cargo test --workspace
```

## Results

All green — 0 failures, 0 warnings.

| Suite | Tests | Result |
|-------|-------|--------|
| `papyrus_core` unit | 49 | PASS |
| `integration_extraction` | 7 | PASS |
| `integration_phase3_pipeline` | 3 | PASS |
| `module_surface` | 6 | PASS |
| **Total** | **65** | **PASS** |

## Definition of Done

- [x] `compute_body_size()` returns mode font size with deterministic tie-breaker.
- [x] `detect_headings()` maps ratio boundaries exactly (`2.0`, `1.7`, `1.4`, `heading_size_ratio`).
- [x] `detect_formatting()` supports name-based detection and descriptor fallback.
- [x] `build_document()` groups consecutive classification blocks into `Node::Heading`/`Node::Paragraph`.
- [x] Missing font info creates `Node::RawText` and emits `Warning::MissingFontMetrics`.
- [x] `PapyrusBuilder` supports `heading_size_ratio`, `detect_bold`, `detect_italic`.
- [x] `Papyrus::extract()` orchestrates parser + detector and aggregates warnings.
- [x] `convert()` uses default configuration.
- [x] New Phase 3 integration tests pass against fixtures.
- [x] `cargo test --workspace` is fully green.

## Notes

- `strip_subset_prefix` promoted to `pub(crate)` and shared between `parser` and `detector` to eliminate duplication.
- `detect_headings` takes ownership of segments (`Vec<RawTextSegment>`) to avoid heap-allocation clones per segment.
- `flush_group` helper extracted in `build_document` to consolidate three previously identical flush blocks.
- Warnings from parser and detector are additive; all preserved in `ConversionResult`.
