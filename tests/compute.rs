use suffete::compute;

#[test]
fn compute_adds_two_integers() {
    assert_eq!(compute(2, 3), 5);
}

#[test]
fn compute_handles_negatives() {
    assert_eq!(compute(-7, 4), -3);
}

#[test]
fn compute_wraps_on_overflow() {
    assert_eq!(compute(i64::MAX, 1), i64::MIN);
}
