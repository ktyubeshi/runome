# Testing Utilities

Utility scripts and helper binaries that orchestrate the Janome compatibility workflow live here.

- `gen_golden.py`: Generate golden outputs from Janome for inputs under `fixtures/`.
  - Requires `janome==0.5.0`.
  - Walks every `*.txt` under the provided directory; keep fixture files curated to avoid bloating goldens.
  - Examples:
    - `python tools/gen_golden.py --inputs fixtures/cases --mode full`
    - `python tools/gen_golden.py --inputs fixtures/cases --mode wakati`
- `run_diffcheck.rs` (planned): Optional Rust helper to compare Runome output against goldens without hitting the Python layer.

Scripts should avoid network access and pin Janome `==0.5.0` to stay deterministic.
