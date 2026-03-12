use std::marker::PhantomData;
use std::ops::Index;

#[derive(Debug, PartialEq, Eq)]
pub struct Id<T>(u32, PhantomData<T>);

impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub trait Indexer {
    fn index(&self) -> usize;
}

impl<T> Indexer for Id<T> {
    #[inline]
    fn index(&self) -> usize {
        self.0 as usize
    }
}

pub struct Arena<T>(Vec<T>);

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> Arena<T> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn alloc(&mut self, t: T) -> Id<T> {
        let id = Id(self.0.len() as u32, PhantomData);
        self.0.push(t);
        id
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &T)> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, t)| (Id(i as u32, PhantomData), t))
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, index: Id<T>) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}
