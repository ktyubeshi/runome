//! Golden diff tests placeholder.
//!
//! This integration test will eventually compare Runome output against
//! Janome-generated JSONL files under `golden/`.  For now it documents the
//! expected wiring without asserting on incomplete data.

use std::path::Path;

#[test]
#[ignore = "golden diff harness not implemented yet"]
fn diff_against_janome_goldens() {
    let golden_dir = Path::new("golden");
    assert!(
        golden_dir.exists(),
        "golden directory missing; run tools/gen_golden.py first"
    );

    todo!("Implement diff harness described in tmp/plan.md");
}
