use std::{
    cell::{Cell, UnsafeCell},
    fmt::{self, Display},
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub struct VecCell<T> {
    elems: UnsafeCell<Vec<T>>,
    borrows: Vec<Cell<BorrowState>>,
    immutable_borrow_count: Cell<usize>,
    mutable_borrow_count: Cell<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorrowState {
    None,
    Immutable,
    Mutable,
}

#[derive(Debug)]
struct UnsafeRef<'a, T> {
    index: usize,
    vec: &'a VecCell<T>,
}

#[derive(Debug)]
pub struct Ref<'a, T>(UnsafeRef<'a, T>);

#[derive(Debug)]
pub struct RefMut<'a, T>(UnsafeRef<'a, T>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    OutOfBounds,
    Aliasing,
}

pub type Result<T> = std::result::Result<T, Error>;

impl<T> VecCell<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elems: UnsafeCell::new(Vec::with_capacity(capacity)),
            borrows: Vec::with_capacity(capacity),
            immutable_borrow_count: Cell::new(0),
            mutable_borrow_count: Cell::new(0),
        }
    }

    pub fn len(&self) -> usize {
        // # Safety
        // We do not mutate elems here.
        unsafe { &*self.elems.get() }.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, v: T) {
        let elems = self.elems.get_mut();
        elems.push(v);
        self.borrows.push(Cell::new(BorrowState::None));
    }

    pub fn pop(&mut self) -> Option<T> {
        let elems = self.elems.get_mut();
        self.borrows.pop();
        elems.pop()
    }

    pub fn get(&self, index: usize) -> Result<Ref<T>> {
        let borrow = self.borrows.get(index).ok_or(Error::OutOfBounds)?;
        if borrow.get() == BorrowState::Mutable {
            Err(Error::Aliasing)
        } else {
            borrow.set(BorrowState::Immutable);
            cell_update(&self.immutable_borrow_count, |c| c + 1);
            Ok(Ref(UnsafeRef { index, vec: self }))
        }
    }

    pub fn get_mut(&self, index: usize) -> Result<RefMut<T>> {
        let borrow = self.borrows.get(index).ok_or(Error::OutOfBounds)?;
        if borrow.get() != BorrowState::None {
            Err(Error::Aliasing)
        } else {
            borrow.set(BorrowState::Mutable);
            cell_update(&self.mutable_borrow_count, |c| c + 1);
            Ok(RefMut(UnsafeRef { index, vec: self }))
        }
    }

    pub fn try_iter(&self) -> Result<impl Iterator<Item = &T>> {
        if self.mutable_borrow_count.get() != 0 {
            return Err(Error::Aliasing);
        }
        // # Safety
        // Asserted above that no element is mutably borrowed.
        // Aliasing rules allow multiple immutable borrows.
        let elems = unsafe { &*self.elems.get() };
        Ok(elems.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.into_iter()
    }
}

impl<T> Default for VecCell<T> {
    fn default() -> Self {
        Self {
            elems: UnsafeCell::new(Vec::new()),
            borrows: Vec::new(),
            immutable_borrow_count: Cell::new(0),
            mutable_borrow_count: Cell::new(0),
        }
    }
}

impl<T> FromIterator<T> for VecCell<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let elems: Vec<_> = iter.into_iter().collect();
        let len = elems.len();
        Self {
            elems: UnsafeCell::new(elems),
            borrows: vec![Cell::new(BorrowState::None); len],
            immutable_borrow_count: Cell::new(0),
            mutable_borrow_count: Cell::new(0),
        }
    }
}

impl<T> IntoIterator for VecCell<T> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.elems.into_inner().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a mut VecCell<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        let elems = unsafe { &mut *self.elems.get() };
        elems.iter_mut()
    }
}

impl<'a, T> UnsafeRef<'a, T> {
    /// # Safety
    /// Need to ensure we do not create aliasing mutable borrow and that `self.index` is in
    /// `self.vec`'s bounds.
    unsafe fn deref(&self) -> &T {
        unsafe {
            let elems = &*self.vec.elems.get();
            elems.get_unchecked(self.index)
        }
    }

    /// # Safety
    /// Need to ensure we do not create aliasing borrow of any kind  and that self.index is in
    /// `self.vec`'s bounds.
    unsafe fn deref_mut(&mut self) -> &mut T {
        unsafe {
            let elems = &mut *self.vec.elems.get();
            elems.get_unchecked_mut(self.index)
        }
    }
}

impl<T> Deref for Ref<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // # Safety
        // Preconditions ensured on `Ref`'s construction in [`VecCell::get`].
        // `VecCell` makes sure that we never break invariants.
        unsafe { self.0.deref() }
    }
}

impl<T> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        // # Safety
        // We assert on Ref construction that index is in bounds.
        let borrow = unsafe { self.0.vec.borrows.get_unchecked(self.0.index) };
        let new_count = cell_update(&self.0.vec.immutable_borrow_count, |count| count - 1);
        if new_count == 0 {
            borrow.set(BorrowState::None);
        }
    }
}

impl<T: Display> Display for Ref<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", **self)
    }
}

impl<T> Deref for RefMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Preconditions ensured on `RefMut`'s construction in [`VecCell::get_mut`]
        // `VecCell` makes sure that we never break invariants.
        unsafe { self.0.deref() }
    }
}

impl<T> DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Preconditions ensured on `RefMut`'s construction in [`VecCell::get_mut`]
        // `VecCell` makes sure that we never break invariants.
        unsafe { self.0.deref_mut() }
    }
}

impl<T> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        // # Safety
        // We assert on Ref construction that index is in bounds.
        let borrow = unsafe { self.0.vec.borrows.get_unchecked(self.0.index) };
        cell_update(&self.0.vec.mutable_borrow_count, |count| count - 1);
        borrow.set(BorrowState::None);
    }
}

impl<T: Display> Display for RefMut<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", **self)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OutOfBounds => write!(f, "Out of bounds index"),
            Error::Aliasing => write!(f, "Borrow would lead to illegal aliasing"),
        }
    }
}

impl std::error::Error for Error {}

fn cell_update<T: Copy>(cell: &Cell<T>, f: impl FnOnce(T) -> T) -> T {
    let v = cell.get();
    let new = f(v);
    cell.set(new);
    new
}
