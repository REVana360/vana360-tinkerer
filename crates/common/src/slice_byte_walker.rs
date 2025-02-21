use anyhow::Result;
use std::cmp::min;

use crate::byte_walker::{ByteWalker, ByteWalkerError};

pub struct SliceByteWalker<'a> {
    slice: &'a [u8],
    offset: usize,
}

impl<'a> SliceByteWalker<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self { slice, offset: 0 }
    }
}

impl<'a> ByteWalker for SliceByteWalker<'a> {
    fn goto_usize(&mut self, offset: usize) {
        self.offset = offset;
    }

    fn skip(&mut self, count: usize) {
        self.offset += count;
    }

    fn offset(&self) -> usize {
        self.offset
    }

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn read_bytes_at(&mut self, offset: usize, amount: usize) -> Result<&[u8]> {
        let len = self.len();
        if len <= offset {
            debug_assert!(false, "Out of range");
            return Err(ByteWalkerError::OutOfRange {
                buffer_length: len,
                requested_index: offset,
            }
            .into());
        }
        let amount = min(len - offset, amount);

        let result = &self.slice[offset..offset + amount];

        Ok(result)
    }

    fn take_bytes(&mut self, amount: usize) -> Result<&[u8]> {
        let len = self.len();
        let amount = min(len - self.offset, amount);

        let result = &self.slice[self.offset..self.offset + amount];
        self.offset += amount;

        Ok(result)
    }
}
