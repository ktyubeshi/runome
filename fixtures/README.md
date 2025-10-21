# Fixtures Layout

This directory will store handcrafted inputs for the rebuilt test suite.

- `cases/`: Minimal and real-world text samples that will be fed to Janome and Runome.
  - `basic_sumomo.txt`: Canonical 「すもももももももものうち」 smoke test.
  - Add further `.txt` files (UTF-8) as coverage grows.
- `userdic/`: User dictionary CSVs (both ipadic and simple formats) used for compatibility testing.
  - `sample_simpledic.csv`: Tiny simpledic example from tmp/plan.md to verify plumbing.

Populate these folders with UTF-8 encoded files. Large corpora should live outside the repo and be referenced through the golden generator script instead.
