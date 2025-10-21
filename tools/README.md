# Testing Utilities

Utility scripts and helper binaries that orchestrate the Janome compatibility workflow live here.

- `gen_golden.py` (planned): Generate golden outputs from Janome for inputs under `fixtures/`.
- `run_diffcheck.rs` (planned): Optional Rust helper to compare Runome output against goldens without hitting the Python layer.

Scripts should avoid network access and pin Janome `==0.5.0` to stay deterministic.
