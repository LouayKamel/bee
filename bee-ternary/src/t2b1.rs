// Copyright 2020 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::{Btrit, RawEncoding, RawEncodingBuf, ShiftTernary, Utrit};
use std::ops::Range;

// Trits Per Byte
const TPB: usize = 2;
// Number required to push a byte between balanced and unbalanced representations
const BAL: i8 = 4;

/// An encoding scheme slice that uses a single byte to represent two trits.
#[repr(transparent)]
pub struct T2B1([()]);

impl T2B1 {
    unsafe fn make(ptr: *const i8, offset: usize, len: usize) -> *const Self {
        let len = (len << 2) | (offset % TPB);
        std::mem::transmute((ptr.add(offset / TPB), len))
    }

    unsafe fn ptr(&self, index: usize) -> *const i8 {
        let byte_offset = (self.len_offset().1 + index) / TPB;
        (self.0.as_ptr() as *const i8).add(byte_offset)
    }

    fn len_offset(&self) -> (usize, usize) {
        (self.0.len() >> 2, self.0.len() & 0b11)
    }
}

fn extract(x: i8, elem: usize) -> Btrit {
    debug_assert!(
        elem < TPB,
        "Attempted to extract invalid element {} from balanced T2B1 trit",
        elem
    );
    Utrit::from_u8((((x + BAL) / 3i8.pow(elem as u32)) % 3) as u8).shift()
}

fn insert(x: i8, elem: usize, trit: Btrit) -> i8 {
    debug_assert!(
        elem < TPB,
        "Attempted to insert invalid element {} into balanced T2B1 trit",
        elem
    );
    let utrit = trit.shift();
    let ux = x + BAL;
    let ux = ux + (utrit.into_u8() as i8 - (ux / 3i8.pow(elem as u32)) % 3) * 3i8.pow(elem as u32);
    ux - BAL
}

impl RawEncoding for T2B1 {
    type Trit = Btrit;
    type Buf = T2B1Buf;

    fn empty() -> &'static Self {
        unsafe { &*Self::make(&[] as *const _, 0, 0) }
    }

    fn len(&self) -> usize {
        self.len_offset().0
    }

    fn as_i8_slice(&self) -> &[i8] {
        assert!(self.len_offset().1 == 0);
        unsafe { std::slice::from_raw_parts(self as *const _ as *const _, (self.len() + TPB - 1) / TPB) }
    }

    unsafe fn as_i8_slice_mut(&mut self) -> &mut [i8] {
        assert!(self.len_offset().1 == 0);
        std::slice::from_raw_parts_mut(self as *mut _ as *mut _, (self.len() + TPB - 1) / TPB)
    }

    unsafe fn get_unchecked(&self, index: usize) -> Self::Trit {
        let b = self.ptr(index).read();
        extract(b, (self.len_offset().1 + index) % TPB)
    }

    unsafe fn set_unchecked(&mut self, index: usize, trit: Self::Trit) {
        let b = self.ptr(index).read();
        let b = insert(b, (self.len_offset().1 + index) % TPB, trit);
        (self.ptr(index) as *mut i8).write(b);
    }

    unsafe fn slice_unchecked(&self, range: Range<usize>) -> &Self {
        &*Self::make(
            self.ptr(range.start),
            (self.len_offset().1 + range.start) % TPB,
            range.end - range.start,
        )
    }

    unsafe fn slice_unchecked_mut(&mut self, range: Range<usize>) -> &mut Self {
        &mut *(Self::make(
            self.ptr(range.start),
            (self.len_offset().1 + range.start) % TPB,
            range.end - range.start,
        ) as *mut Self)
    }

    fn is_valid(b: i8) -> bool {
        b >= -BAL && b <= BAL
    }

    unsafe fn from_raw_unchecked(b: &[i8], num_trits: usize) -> &Self {
        assert!(num_trits <= b.len() * TPB);
        &*Self::make(b.as_ptr() as *const _, 0, num_trits)
    }

    unsafe fn from_raw_unchecked_mut(b: &mut [i8], num_trits: usize) -> &mut Self {
        assert!(num_trits <= b.len() * TPB);
        &mut *(Self::make(b.as_ptr() as *const _, 0, num_trits) as *mut _)
    }
}

/// An encoding scheme buffer that uses a single byte to represent two trits.
#[derive(Clone)]
pub struct T2B1Buf(Vec<i8>, usize);

impl RawEncodingBuf for T2B1Buf {
    type Slice = T2B1;

    fn new() -> Self {
        Self(Vec::new(), 0)
    }

    fn with_capacity(cap: usize) -> Self {
        let cap = (cap / TPB) + (cap % TPB);
        Self(Vec::with_capacity(cap), 0)
    }

    fn push(&mut self, trit: <Self::Slice as RawEncoding>::Trit) {
        if self.1 % TPB == 0 {
            self.0.push(insert(0, 0, trit));
        } else {
            let last_index = self.0.len() - 1;
            let b = unsafe { self.0.get_unchecked_mut(last_index) };
            *b = insert(*b, self.1 % TPB, trit);
        }
        self.1 += 1;
    }

    fn pop(&mut self) -> Option<<Self::Slice as RawEncoding>::Trit> {
        let val = if self.1 == 0 {
            return None;
        } else if self.1 % TPB == 1 {
            self.0.pop().map(|b| extract(b, 0))
        } else {
            let last_index = self.0.len() - 1;
            unsafe { Some(extract(*self.0.get_unchecked(last_index), (self.1 + TPB - 1) % TPB)) }
        };
        self.1 -= 1;
        val
    }

    fn as_slice(&self) -> &Self::Slice {
        unsafe { &*Self::Slice::make(self.0.as_ptr() as _, 0, self.1) }
    }

    fn as_slice_mut(&mut self) -> &mut Self::Slice {
        unsafe { &mut *(Self::Slice::make(self.0.as_ptr() as _, 0, self.1) as *mut _) }
    }
}
