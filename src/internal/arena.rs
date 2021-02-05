use std::{
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Index,
};

/// The index of a value allocated in an arena that holds `T`s.
pub struct Id<T> {
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
    pub fn into_raw(self) -> usize {
        self.raw as usize
    }
}

/// Yet another index-based arena.
#[derive(Clone, PartialEq, Eq)]
pub struct Arena<T> {
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

impl<T> Arena<T> {
    pub fn new() -> Arena<T> {
        Arena { data: Vec::new() }
    }

    pub fn alloc(&mut self, value: T) -> Id<T> {
        let raw = self.data.len() as u32;
        self.data.push(value);
        Id {
            raw,
            _ty: PhantomData,
        }
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;
    fn index(&self, id: Id<T>) -> &T {
        &self.data[id.raw as usize]
    }
}
