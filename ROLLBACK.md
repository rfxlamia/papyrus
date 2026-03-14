# Rollback Plan — Papyrus v0.1.1

## Trigger Conditions

Rollback if any of the following occur after publish:
- Users report compilation failures from the new version
- Spatial layout produces worse output than v0.1.0 for common PDFs
- A critical CVE is discovered in new or existing dependencies

## Rollback Steps

### 1. Yank the published crates (order matters: CLI first, then core)

```bash
cargo yank --version 0.1.1 papyrus-cli
cargo yank --version 0.1.1 papyrus-core
```

### 2. Verify yank took effect

```bash
cargo search papyrus-core   # should show 0.1.0 as latest
cargo search papyrus-cli    # should show 0.1.0 as latest
```

### 3. Revert the release commit on main

```bash
git revert <release-commit-sha>
git push origin main
```

### 4. Communicate

- Update CHANGELOG.md to note the yank and reason
- If users opened issues, respond with the rollback notice

## Notes

- `cargo yank` does not delete the crate — it prevents new projects from depending on the yanked version. Existing Cargo.lock files that already resolved to 0.1.1 will continue to work.
- v0.1.0 remains available as the fallback version.
