// This file is part of faster, the SIMD library for humans.
// Copyright 2017 Adam Niederer <adam.niederer@gmail.com>

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use vecs::{Packable, Packed};

pub trait PackedIterator : Sized + ExactSizeIterator {
    type Scalar : Packable;
    type Vector : Packed<Self::Scalar>;

    fn width(&self) -> usize;
    fn scalar_len(&self) -> usize;
    fn scalar_position(&self) -> usize;

    fn next_vector(&mut self) -> Option<Self::Vector>;
    fn simd_map<A, B, F, G>(self, vectorfn: F, scalarfn: G) -> PackedMap<Self, F, G>
        where F : Fn(Self::Vector) -> A, G : Fn(Self::Scalar) -> B, A : Packed<B>, B : Packable;
}

#[derive(Debug)]
pub struct PackedIter<'a, T : 'a + Packable> {
    pub position: usize,
    pub data: &'a [T],
}

#[derive(Debug)]
pub struct PackedMap<I, F, G> {
    pub iter: I,
    pub vectorfn: F,
    pub scalarfn: G,
}

impl<'a, T> Iterator for PackedIter<'a, T> where T : Packable {
    type Item = <PackedIter<'a, T> as PackedIterator>::Scalar;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.data.get(self.position).map(|v| { self.position += 1; *v })
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.data.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl<'a, T> ExactSizeIterator for PackedIter<'a, T>
    where T : Packable {

    #[inline(always)]
    fn len(&self) -> usize {
        self.data.len()
    }
}

impl<'a, T> PackedIterator for PackedIter<'a, T> where T : Packable {
    type Vector = <T as Packable>::Vector;
    type Scalar = T;

    #[inline(always)]
    fn width(&self) -> usize {
        T::Vector::WIDTH
    }

    #[inline(always)]
    fn scalar_len(&self) -> usize {
        self.data.len()
    }

    #[inline(always)]
    fn scalar_position(&self) -> usize {
        self.position
    }

    #[inline(always)]
    fn next_vector(&mut self) -> Option<Self::Vector> {
        if self.position + self.width() <= self.scalar_len() {
            let ret: Option<Self::Vector> = Some(Self::Vector::load(self.data, self.position));
            self.position += Self::Vector::WIDTH;
            ret
        } else {
            None
        }
    }

    #[inline(always)]
    fn simd_map<A, B, F, G>(self, vectorfn: F, scalarfn: G) -> PackedMap<Self, F, G>
        where F : Fn(Self::Vector) -> A, G : Fn(Self::Scalar) -> B, A : Packed<B>, B : Packable {
        PackedMap {
            iter: self,
            vectorfn: vectorfn,
            scalarfn: scalarfn,
        }
    }
}

impl<T: PackedIterator> IntoPackedIterator for T {
    type Iter = T;

    #[inline(always)]
    fn into_simd_iter(self) -> T {
        self
    }
}

pub trait IntoPackedIterator {
    type Iter: PackedIterator;

    fn into_simd_iter(self) -> Self::Iter;
}

pub trait IntoPackedRefIterator<'a> {
    type Iter: PackedIterator;

    fn simd_iter(&'a self) -> Self::Iter;
}

pub trait IntoPackedRefMutIterator<'a> {
    type Iter: PackedIterator;

    fn simd_iter_mut(&'a mut self) -> Self::Iter;
}

// Impl ref & ref mut iterators for moved iterator
impl<'a, I: 'a + ?Sized> IntoPackedRefIterator<'a> for I
    where &'a I: IntoPackedIterator {
    type Iter = <&'a I as IntoPackedIterator>::Iter;

    fn simd_iter(&'a self) -> Self::Iter {
        self.into_simd_iter()
    }
}

impl<'a, I: 'a + ?Sized> IntoPackedRefMutIterator<'a> for I
    where &'a mut I: IntoPackedIterator {
    type Iter = <&'a mut I as IntoPackedIterator>::Iter;

    fn simd_iter_mut(&'a mut self) -> Self::Iter {
        self.into_simd_iter()
    }
}

impl<A, B, I, F, G> Iterator for PackedMap<I, F, G>
    where I : PackedIterator, F : Fn(I::Vector) -> A, G : Fn(<I as Iterator>::Item) -> B, A : Packed<B>, B : Packable{
    type Item = B;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        // TODO: Apply vectorfn instead and cache results
        self.iter.next().map(&self.scalarfn)
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.len() - self.iter.scalar_position() * self.width()) / self.width();
        (remaining, Some(remaining))
    }
}

impl<'a, I, F, G> ExactSizeIterator for PackedMap<I, F, G>
    where Self : PackedIterator, I : PackedIterator {
    #[inline(always)]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, A, B, I, F, G> PackedIterator for PackedMap<I, F, G>
    where I : PackedIterator, F : Fn(I::Vector) -> A, G : Fn(<I as Iterator>::Item) -> B, A : Packed<B>, B : Packable {
    type Vector = A;
    type Scalar = B;

    #[inline(always)]
    fn width(&self) -> usize {
        Self::Vector::WIDTH
    }

    #[inline(always)]
    fn scalar_len(&self) -> usize {
        self.iter.scalar_len()
    }

    #[inline(always)]
    fn scalar_position(&self) -> usize {
        self.iter.scalar_position()
    }

    #[inline(always)]
    fn next_vector(&mut self) -> Option<Self::Vector> {
        self.iter.next_vector().map(&self.vectorfn)
    }

    #[inline(always)]
    fn simd_map<AA, BB, AF, BG>(self, vectorfn: AF, scalarfn: BG) -> PackedMap<Self, AF, BG>
        where AF : Fn(Self::Vector) -> AA, BG : Fn(Self::Scalar) -> BB, AA : Packed<BB>, BB : Packable {
        PackedMap {
            iter: self,
            vectorfn: vectorfn,
            scalarfn: scalarfn,
        }
    }
}

pub trait IntoScalar<T> where T : Packable {
    type Scalar : Packable;
    type Vector : Packed<Self::Scalar>;
    fn scalar_collect(&mut self) -> Vec<T>;
    fn scalar_fill<'a>(&mut self, fill: &'a mut [T]) -> &'a mut [T];
}

impl<'a, T, I> IntoScalar<T> for I
    where I : PackedIterator<Scalar = T, Item = T>, I::Vector : Packed<T>, T : Packable {
    type Scalar = I::Scalar;
    type Vector = I::Vector;

    #[inline(always)]
    fn scalar_collect(&mut self) -> Vec<Self::Scalar> {
        let mut offset = 0;
        let mut ret = Vec::with_capacity(self.len());

        unsafe {
            ret.set_len(self.len());
            while let Some(vec) = self.next_vector() {
                vec.store(ret.as_mut_slice(), offset);
                offset += Self::Vector::WIDTH;
            }
        }
        ret
    }

    #[inline(always)]
    fn scalar_fill<'b>(&mut self, fill: &'b mut [Self::Scalar]) -> &'b mut [Self::Scalar] {
        let mut offset = 0;

        while let Some(vec) = self.next_vector() {
            vec.store(fill, offset);
            offset += Self::Vector::WIDTH;
        }

        while let Some(scl) = self.next() {
            fill[offset] = scl;
            offset += 1;
        }
        fill
    }
}
