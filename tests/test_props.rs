//! Property-based test skeleton for Runome tokenizer.
//!
//! The real implementation will use `proptest` to generate Unicode-heavy inputs
//! and verify invariants between Janome and Runome.  The test is marked as
//! ignored so that the suite stays green until the harness lands.

#[test]
#[ignore = "proptest scenarios not implemented yet"]
fn tokenizer_respects_core_invariants() {
    todo!("Add property-based checks (see tmp/plan.md §1.4).");
}
