# Testing Roadmap Skeleton

This document mirrors the test-first recovery plan in `tmp/plan.md` and pinpoints the artefacts that now exist in the repository.

## Pillars

- **Janome as Oracle:** Pin `janome==0.5.0` and its bundled mecab-ipadic dictionary. All differential tests must use this exact environment.
- **Golden Snapshots:** `tools/gen_golden.py` enumerates inputs under `fixtures/cases/` and stores Janome outputs in `golden/*.jsonl`. These files become the contract for Runome.
- **Layered Tests:** Rust integration tests (`tests/test_diff_golden.rs`, `tests/test_props.rs`, `tests/test_metamorphic.rs`) will cover golden diffs, property-based checks, and metamorphic invariants. Python tests complement them by ensuring binding parity.

## Current Status

- Fixtures now include `basic_sumomo.txt` plus a sample simple dictionary in `fixtures/userdic/`.
- `tools/gen_golden.py` generates JSONL outputs (full or wakati) and enforces Janome `0.5.0` by default.
- The Rust/Python test files remain placeholders, each marked `#[ignore]` or `pytest.skip` until populated.
- CI workflow is still pending; once in place it must install Janome, build Runome with the Python feature, and execute both Rust and Python suites.

## Next Steps

1. Move/expand fixtures from the legacy `tests/` directory so that goldens cover the documented edge cases.
2. Flesh out the Rust integration tests using `proptest` and the golden files.
3. Expand the Python binding tests, potentially reorganising them into their own package if they grow large.
