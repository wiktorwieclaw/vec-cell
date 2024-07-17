use vec_cell::VecCell;

fn main() {
    let mut v = VecCell::from_iter([1, 2]);

    // `&VecCell` doesn't implement IntoIterator, so `for e in &v {}` does not compile.
    // You have to use `VecCell::try_iter`.
    for _ in v.try_iter().unwrap() {}

    let a = v.get_mut(0).unwrap();

    // Cannot immutably iterate while we have borrowed elements.
    assert!(v.try_iter().is_err());

    // Note: For other kind of iteration borrow checker will ensure we do not have any borrows.
    drop(a);
    for _ in &mut v {}
    for _ in v {}
}
