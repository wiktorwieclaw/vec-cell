# `vec-cell`
`vec-cell` provides a `Vec`-like type that allows you to safely get many non-aliasing mutable references to it's elements.

## Example
```rs
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
    assert_eq!(v.get_mut(0).unwrap_err(), Error::Borrowed);
}
```

## How it works
`VecCell` stores additional metadata at runtime to track if an element has already been borrowed.
```rs
pub struct VecCell<T> {
    elems: UnsafeCell<Vec<T>>,
    borrows: Vec<Cell<BorrowState>>,
    immutable_borrow_count: Cell<usize>,
    mutable_borrow_count: Cell<usize>,
}

enum BorrowState {
    None,
    Immutable,
    Mutable,
}
```
