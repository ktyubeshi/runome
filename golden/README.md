# Golden Outputs

Janome-driven expected outputs will be checked in here as JSONL or plain-text snapshots.

Each file should encode a single test run so that Rust and Python suites can diff against the exact same artifact.

Regenerate these files exclusively via `tools/gen_golden.py` to keep provenance auditable.
