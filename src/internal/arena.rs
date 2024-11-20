use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Index, Range};

type FnvIndexSet<V> = indexmap::IndexSet<V, rustc_hash::FxBuildHasher>;

/// The index of a value allocated in an arena that holds `T`s.
///
/// The Clone, Copy and other traits are defined manually because
/// deriving them adds some additional constraints on the `T` generic type
/// that we actually don't need since it is phantom.
///
/// <https://github.com/rust-lang/rust/issues/26925>
pub(crate) struct Id<T> {
    raw: u32,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Id<T>) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state)
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut type_name = std::any::type_name::<T>();
        if let Some(id) = type_name.rfind(':') {
            type_name = &type_name[id + 1..]
        }
        write!(f, "Id::<{}>({})", type_name, self.raw)
    }
}

impl<T> Id<T> {
    pub(crate) fn into_raw(self) -> usize {
        self.raw as usize
    }
    fn from(n: u32) -> Self {
        Self {
            raw: n,
            _ty: PhantomData,
        }
    }
    pub(crate) fn range_to_iter(range: Range<Self>) -> impl Iterator<Item = Self> {
        let start = range.start.raw;
        let end = range.end.raw;
        (start..end).map(Self::from)
    }
}

/// Yet another index-based arena.
///
/// An arena is a kind of simple grow-only allocator, backed by a `Vec`
/// where all items have the same lifetime, making it easier
/// to have references between those items.
/// They are all dropped at once when the arena is dropped.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Arena<T> {
    data: Vec<T>,
}

impl<T: fmt::Debug> fmt::Debug for Arena<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Arena")
            .field("len", &self.data.len())
            .field("data", &self.data)
            .finish()
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub(crate) fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub(crate) fn alloc(&mut self, value: T) -> Id<T> {
        let raw = self.data.len();
        self.data.push(value);
        Id::from(raw as u32)
    }

    pub(crate) fn alloc_iter<I: Iterator<Item = T>>(&mut self, values: I) -> Range<Id<T>> {
        let start = Id::from(self.data.len() as u32);
        values.for_each(|v| {
            self.alloc(v);
        });
        let end = Id::from(self.data.len() as u32);
        Range { start, end }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;
    fn index(&self, id: Id<T>) -> &T {
        &self.data[id.raw as usize]
    }
}

impl<T> Index<Range<Id<T>>> for Arena<T> {
    type Output = [T];
    fn index(&self, id: Range<Id<T>>) -> &[T] {
        &self.data[(id.start.raw as usize)..(id.end.raw as usize)]
    }
}

/// Yet another index-based arena. This one de-duplicates entries by hashing.
///
/// An arena is a kind of simple grow-only allocator, backed by a `Vec`
/// where all items have the same lifetime, making it easier
/// to have references between those items.
/// In this case the `Vec` is inside a `IndexSet` allowing fast lookup by value not just index.
/// They are all dropped at once when the arena is dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct HashArena<T: Hash + Eq> {
    data: FnvIndexSet<T>,
}

impl<T: Hash + Eq + fmt::Debug> fmt::Debug for HashArena<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Arena")
            .field("len", &self.data.len())
            .field("data", &self.data)
            .finish()
    }
}

impl<T: Hash + Eq> HashArena<T> {
    pub fn new() -> Self {
        HashArena {
            data: FnvIndexSet::default(),
        }
    }

    pub fn alloc(&mut self, value: T) -> Id<T> {
        let (raw, _) = self.data.insert_full(value);
        Id::from(raw as u32)
    }
}

impl<T: Hash + Eq> Index<Id<T>> for HashArena<T> {
    type Output = T;
    fn index(&self, id: Id<T>) -> &T {
        &self.data[id.raw as usize]
    }
}
