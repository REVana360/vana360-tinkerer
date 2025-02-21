use crate::{
    byte_functions::HasByteFunctions, byte_walker::BufferedByteWalker,
    writing_byte_walker::WritingByteWalker,
};

pub type VecByteWalker = BufferedByteWalker<Vec<u8>>;

impl VecByteWalker {
    pub fn new() -> Self {
        Self {
            data: vec![],
            offset: 0,
        }
    }

    pub fn with_size(size: usize) -> Self {
        Self {
            data: vec![0; size],
            offset: 0,
        }
    }

    #[inline]
    pub fn swap_8_bytes(&mut self, a: usize, b: usize) {
        debug_assert!(
            self.data.len() > a + 8,
            "a is {} and len is {}",
            a,
            self.data.len()
        );
        debug_assert!(
            self.data.len() > b + 8,
            "b is {} and len is {}",
            b,
            self.data.len()
        );
        unsafe {
            let ptr = self.data.as_mut_ptr();
            let ptr_a = ptr.add(a) as *mut u64;
            let ptr_b = ptr.add(b) as *mut u64;
            let tmp = ptr_a.read_unaligned();
            ptr_a.write_unaligned(ptr_b.read_unaligned());
            ptr_b.write_unaligned(tmp);
        }
    }

    #[inline]
    pub fn swap_8_bytes_xor(&mut self, a: usize, b: usize) {
        unsafe {
            let ptr = self.data.as_mut_ptr();
            let ptr_a = ptr.add(a) as *mut u64;
            let ptr_b = ptr.add(b) as *mut u64;
            ptr_a.write_unaligned(ptr_a.read_unaligned() ^ ptr_b.read_unaligned());
            ptr_b.write_unaligned(ptr_a.read_unaligned() ^ ptr_b.read_unaligned());
            ptr_a.write_unaligned(ptr_a.read_unaligned() ^ ptr_b.read_unaligned());
        }
    }
}

impl WritingByteWalker for VecByteWalker {
    fn write_bytes_at(&mut self, offset: usize, bytes: &[u8]) {
        let end = offset + bytes.len();
        if end > self.data.len() {
            self.data.resize(end, 0);
        }

        self.data[offset..end].copy_from_slice(bytes);
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.write_bytes_at(self.offset, bytes);
        self.offset += bytes.len();
    }

    fn write_be<T: HasByteFunctions>(&mut self, value: T) {
        let end = self.offset + std::mem::size_of::<T>();
        if end > self.data.len() {
            self.data.resize(end, 0);
        }

        value.insert_into_be(&mut self.data[self.offset..end]);
        self.offset = end;
    }

    fn write_le<T: HasByteFunctions>(&mut self, value: T) {
        let end = self.offset + std::mem::size_of::<T>();
        if end > self.data.len() {
            self.data.resize(end, 0);
        }

        value.insert_into_le(&mut self.data[self.offset..end]);
        self.offset = end;
    }

    fn write_be_at<T: HasByteFunctions>(&mut self, offset: usize, value: T) {
        let end = offset + std::mem::size_of::<T>();
        if end > self.data.len() {
            self.data.resize(end, 0);
        }

        value.insert_into_be(&mut self.data[offset..end]);
    }

    fn write_le_at<T: HasByteFunctions>(&mut self, offset: usize, value: T) {
        let end = offset + std::mem::size_of::<T>();
        if end > self.data.len() {
            self.data.resize(end, 0);
        }

        value.insert_into_le(&mut self.data[offset..end]);
    }

    fn set_size(&mut self, size: usize) {
        self.data.resize(size, 0);
    }

    fn into_vec(self) -> Vec<u8> {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use crate::byte_walker::ByteWalker;

    use super::VecByteWalker;

    #[test]
    pub fn swapping_8_bytes() {
        let data = Vec::from_iter(0..100u8);
        let mut walker = VecByteWalker::on(data);

        let bytes_at_6 = u64::from_le_bytes([6, 7, 8, 9, 10, 11, 12, 13]);
        let bytes_at_20 = u64::from_le_bytes([20, 21, 22, 23, 24, 25, 26, 27]);

        assert_eq!(walker.read_le_at::<u64>(6).unwrap(), bytes_at_6);
        assert_eq!(walker.read_le_at::<u64>(20).unwrap(), bytes_at_20);

        walker.swap_8_bytes(6, 20);

        assert_eq!(walker.read_le_at::<u64>(20).unwrap(), bytes_at_6);
        assert_eq!(walker.read_le_at::<u64>(6).unwrap(), bytes_at_20);

        walker.swap_8_bytes_xor(6, 20);

        assert_eq!(walker.read_le_at::<u64>(6).unwrap(), bytes_at_6);
        assert_eq!(walker.read_le_at::<u64>(20).unwrap(), bytes_at_20);
    }
}
