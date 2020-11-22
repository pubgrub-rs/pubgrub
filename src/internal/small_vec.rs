use std::fmt;

#[derive(Clone)]
pub enum SmallVec<T> {
    Empty,
    One([T; 1]),
    Two([T; 2]),
    Flexible(Vec<T>),
}

impl<T> SmallVec<T> {
    pub fn empty() -> Self {
        Self::Empty
    }

    pub fn one(t: T) -> Self {
        Self::One([t])
    }

    pub fn as_slice(&self) -> &[T] {
        match self {
            Self::Empty => &[],
            Self::One(v) => v,
            Self::Two(v) => v,
            Self::Flexible(v) => v,
        }
    }

    pub fn push(&mut self, new: T) {
        *self = match std::mem::take(self) {
            Self::Empty => Self::One([new]),
            Self::One([v1]) => Self::Two([v1, new]),
            Self::Two([v1, v2]) => Self::Flexible(vec![v1, v2, new]),
            Self::Flexible(mut v) => {
                v.push(new);
                Self::Flexible(v)
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_slice().iter()
    }
}

impl<T> Default for SmallVec<T> {
    fn default() -> Self {
        Self::Empty
    }
}

impl<T: PartialEq> Eq for SmallVec<T> {}

impl<T: PartialEq> PartialEq<SmallVec<T>> for SmallVec<T> {
    fn eq(&self, other: &SmallVec<T>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: fmt::Debug> fmt::Debug for SmallVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for SmallVec<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(self.as_slice(), s)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for SmallVec<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let items: Vec<T> = serde::Deserialize::deserialize(d)?;

        let mut v = Self::empty();
        for item in items {
            v.push(item);
        }
        Ok(v)
    }
}
