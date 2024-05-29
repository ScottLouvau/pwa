#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BitVectorSlice {
    bits: u64
}

impl BitVectorSlice {
    pub fn new() -> BitVectorSlice {
        BitVectorSlice { bits: 0u64 }
    }

    pub fn new_all(limit: usize) -> BitVectorSlice {
        let mask = (!0u64) >> (64 - limit);
        BitVectorSlice { bits: mask }
    }

    pub fn from_vec(indices: Vec<usize>) -> BitVectorSlice {
        let mut result = BitVectorSlice::new();

        for index in indices {
            result.add(index);
        }

        result
    }

    pub fn add(&mut self, index: usize) {
        if index > 63 { panic!("BitVectorSlice only supports 64 bits"); }
        self.bits |= 1u64 << index;
    }

    pub fn remove(&mut self, index: usize) {
        if index > 63 { panic!("BitVectorSlice only supports 64 bits"); }
        self.bits &= !(1u64 << index);
    }

    pub fn contains(&self, index: usize) -> bool {
        if index > 63 { panic!("BitVectorSlice only supports 64 bits"); }
        (self.bits & (1u64 << index)) != 0
    }

    pub fn clear(&mut self) {
        self.bits = 0u64;
    }

    pub fn all(&mut self, limit: usize) {
        let mask = (!0u64) >> (64 - limit);
        self.bits = mask;
    }

    pub fn not(&mut self, limit: usize) {
        let mask = (!0u64) >> (64 - limit);
        self.bits = !self.bits;
        self.bits &= mask;
    }

    pub fn count(&self) -> u32 {
        self.bits.count_ones()
    }

    pub fn iter(&self) -> BitVectorSliceIterator {
        BitVectorSliceIterator { bits: self.bits }
    }

    pub fn union_with(&mut self, other: &BitVectorSlice) {
        self.bits |= other.bits;
    }

    pub fn intersect_with(&mut self, other: &BitVectorSlice) {
        self.bits &= other.bits;
    }

    pub fn except_with(&mut self, other: &BitVectorSlice) {
        self.bits &= !other.bits;
    }
}

pub struct BitVectorSliceIterator {
    bits: u64
}

impl Iterator for BitVectorSliceIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 { return None; }

        let index = self.bits.trailing_zeros();
        self.bits &= !(1u64 << index);
        Some(index as usize)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_vector_slice_basics() {
        // Slice starts empty
        let mut slice = BitVectorSlice::new();
        assert_eq!(slice.count(), 0);
        assert_eq!(format!("{:?}", slice.iter().collect::<Vec<_>>()), "[]");

        // Verify add, count, iterator
        slice.add(4);
        assert_eq!(slice.count(), 1);
        assert_eq!(format!("{:?}", slice.iter().collect::<Vec<_>>()), "[4]");

        // Add more, verify contains
        slice.add(8);
        slice.add(10);
        assert_eq!(slice.count(), 3);
        assert_eq!(slice.contains(10), true);
        assert_eq!(slice.contains(8), true);
        assert_eq!(slice.contains(2), false);
        assert_eq!(format!("{:?}", slice.iter().collect::<Vec<_>>()), "[4, 8, 10]");

        // Remove, verify does not contain, iterator
        slice.remove(10);
        assert_eq!(slice.count(), 2);
        assert_eq!(slice.contains(10), false);
        assert_eq!(format!("{:?}", slice.iter().collect::<Vec<_>>()), "[4, 8]");
        slice.add(10);

        let slice2 = BitVectorSlice::from_vec(vec![4, 8, 9]);

        // Test AND
        let mut result = slice;
        result.intersect_with(&slice2);
        assert_eq!(format!("{:?}", result.iter().collect::<Vec<_>>()), "[4, 8]");

        // Test OR
        result = slice;
        result.union_with(&slice2);
        assert_eq!(format!("{:?}", result.iter().collect::<Vec<_>>()), "[4, 8, 9, 10]");

        // Test AND NOT
        result = slice;
        result.except_with(&slice2);
        assert_eq!(format!("{:?}", result.iter().collect::<Vec<_>>()), "[10]");

        // Test Clear
        result.clear();
        assert_eq!(result.count(), 0);
        assert_eq!(format!("{:?}", result.iter().collect::<Vec<_>>()), "[]");

        // Test All
        result.all(9);
        assert_eq!(result.count(), 9);
        assert_eq!(format!("{:?}", result.iter().collect::<Vec<_>>()), "[0, 1, 2, 3, 4, 5, 6, 7, 8]");
        result.all(64);
        assert_eq!(result.count(), 64);

        // Test Not
        slice.clear();
        slice.add(2);
        slice.add(3);
        slice.not(6);
        assert_eq!(slice.count(), 4);
        assert_eq!(format!("{:?}", slice.iter().collect::<Vec<_>>()), "[0, 1, 4, 5]");

        // Test new_all
        slice = BitVectorSlice::new_all(6);
        assert_eq!(slice.count(), 6);
        assert_eq!(format!("{:?}", slice.iter().collect::<Vec<_>>()), "[0, 1, 2, 3, 4, 5]");
    }
}