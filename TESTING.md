# Testing Roadmap Skeleton

This document mirrors the test-first recovery plan in `tmp/plan.md` and pinpoints the artefacts that now exist in the repository.

## Pillars

- **Janome as Oracle:** Pin `janome==0.5.0` and its bundled mecab-ipadic dictionary. All differential tests must use this exact environment.
- **Golden Snapshots:** `tools/gen_golden.py` (stub) will enumerate inputs under `fixtures/cases/` and store Janome outputs in `golden/*.jsonl`. These files become the contract for Runome.
- **Layered Tests:** Rust integration tests (`tests/test_diff_golden.rs`, `tests/test_props.rs`, `tests/test_metamorphic.rs`) will cover golden diffs, property-based checks, and metamorphic invariants. Python tests complement them by ensuring binding parity.

## Current Status

- Directory skeletons exist for fixtures, goldens, and tooling.
- Scripts/tests are placeholders with clear TODOs; they are marked to fail-fast with informative messages instead of silently doing nothing.
- CI workflow is still pending; once in place it must install Janome, build Runome with the Python feature, and execute both Rust and Python suites.

## Next Steps

1. Implement `tools/gen_golden.py` so that goldens can be regenerated deterministically.
2. Fill `fixtures/` with minimal text samples and user dictionary CSVs (move or symlink from the legacy `tests/` location).
3. Flesh out the Rust integration tests using `proptest` and the golden files.
4. Expand the Python binding tests, potentially reorganising them into their own package if they grow large.
