# Golden Outputs

Janome-driven expected outputs will be checked in here as JSONL or plain-text snapshots.

Each file should encode a single test run so that Rust and Python suites can diff against the exact same artifact.
`tools/gen_golden.py` writes files such as `janome_full.jsonl` or
`janome_full_sample_simpledic.jsonl`; rerun it with `--overwrite` whenever the fixtures change.

Regenerate these files exclusively via `tools/gen_golden.py` to keep provenance auditable.
