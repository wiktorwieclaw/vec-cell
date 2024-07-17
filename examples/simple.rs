use vec_cell::{Error, VecCell};

fn main() {
    let v = VecCell::from_iter([1, 2]);
    let mut a = v.get_mut(0).unwrap();
    let mut b = v.get_mut(1).unwrap();
    *a += 1;
    *b += 1;
    assert_eq!(*a, 2);
    assert_eq!(*b, 3);

    // already borrowed
    assert_eq!(v.get_mut(0).unwrap_err(), Error::Aliasing);
}
