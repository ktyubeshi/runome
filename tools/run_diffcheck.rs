//! Placeholder binary that will compare Runome output with Janome-generated goldens.
//!
//! The eventual implementation will:
//! - Load fixtures from `fixtures/` and golden JSONL files under `golden/`.
//! - Execute Runome with the same parameters used to create the goldens.
//! - Emit human-readable diffs and a non-zero exit status on mismatches.
//!
//! Keeping this stub in `tools/` clarifies the expected tooling layout without
//! forcing it into the main crate until the design is ready.

fn main() {
    eprintln!("run_diffcheck is not implemented yet. See tmp/plan.md for details.");
    std::process::exit(2);
}
