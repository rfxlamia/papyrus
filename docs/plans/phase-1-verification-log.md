# Phase 1 Verification Log

- cargo build --workspace
- cargo test --workspace
- python3 -m pytest tests/fixtures tests/oracle -q
- python3 tests/oracle/extract_oracle.py tests/fixtures/simple.pdf --out /tmp/simple.oracle.json
