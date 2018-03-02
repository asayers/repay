#![feature(conservative_impl_trait)]

/*!
A simple bitset stored in a single u64.

The downside of such a basic representation is, of course, that sets can only store the numbers
0..64.  The major upside it that it implements `Copy`.  There's also less bookkeeping.  If need to
store elements from 0..64 in a set, `BitSet64` is a very fast way to do it.
*/

#[derive(Copy, Clone, Debug)]
pub struct BitSet64(pub u64);
impl BitSet64 {
    const MAX_IDX: u64 = 63;
    /// None of the bits are set.
    #[inline]
    pub fn empty_set() -> BitSet64 {
        BitSet64(0)
    }
    /// All bits in 0..n are set.  (2 instructions)
    #[inline]
    pub fn full_set(n: u64) -> BitSet64 {
        assert!(n < 64);
        BitSet64((1u64 << n) - 1)  // (2 ** n) - 1
    }
    /// Only the nth bit is set.  (1 instruction)
    pub fn singleton(x: u64) -> BitSet64 {
        assert!(x < 64);
        BitSet64((1u64 << x))
    }

    /// Set the `idx`th bit.  (2 instructions)
    #[inline]
    pub fn insert(&self, idx: u64) -> BitSet64 {
        assert!(idx < 64);
        BitSet64(self.0 | 1u64 << idx)
    }
    /// Unset the `idx`th bit.  (3 instructions)
    #[inline]
    pub fn remove(&self, idx: u64) -> BitSet64 {
        assert!(idx < 64);
        BitSet64(self.0 & !(1u64 << idx))
    }
    /// Flip the `idx`th bit.  (2 instructions)
    #[inline]
    pub fn toggle(&self, idx: u64) -> BitSet64 {
        assert!(idx < 64);
        BitSet64(self.0 ^ (1u64 << idx))
    }
    /// Remove the elements of `other` from `self`.
    #[inline]
    pub fn minus(&self, other: BitSet64) -> BitSet64 {
        BitSet64(self.0 & !other.0)
    }

    /// True iff the `idx`th bit is set.  (3 instructions)
    // TODO: cast to a bool, save an instruction
    #[inline]
    pub fn contains(&self, idx: u64) -> bool {
        assert!(idx < 64);
        self.0 & (1u64 << idx) != 0
    }
    /// The total number of bits which are set.
    #[inline]
    pub fn size(&self) -> u32 {
        self.0.count_ones()
    }
    /// The smallest idx of a set bit.
    #[inline]
    pub fn min(&self) -> Option<u64> {
        if self.0 == 0 { None } else {
            Some(self.0.trailing_zeros() as u64)
        }
    }
    /// The largest idx of a set bit.
    #[inline]
    pub fn max(&self) -> Option<u64> {
        if self.0 == 0 { None } else {
            Some(Self::MAX_IDX - self.0.leading_zeros() as u64)
        }
    }
    /// The largest idx of a set bit.
    #[inline]
    pub fn take_max(&mut self) -> Option<u64> {
        match self.max() {
            None => None,
            Some(max) => { *self = self.remove(max); Some(max) }
        }
    }

    /// Iterate over all possible sets where 0 < size <= n.  Equivalent to
    /// `BitSet64::full_set(n).subsets()`
    #[inline]
    pub fn enumerate(n: u64) -> impl Iterator<Item=BitSet64> {
        (0..(BitSet64::full_set(n).0 + 1)).map(BitSet64)
    }

    /// Iterate over all elements
    #[inline]
    pub fn elements(&self) -> Elements {
        match (self.min(), self.max()) {
            (Some(min), Some(max)) => Elements {
                set: *self,
                cur: min,
                max: max,
                done: false,
            },
            _ => Elements {
                set: *self,
                cur: 0,
                max: 0,
                done: true,
            },
        }
    }

    /// Iterate over all subsets
    #[inline]
    pub fn subsets(&self) -> Subsets {
        Subsets {
            set: *self,
            cur: BitSet64::empty_set(),
            done: false,
        }
    }
}

pub struct Elements {
    set: BitSet64,
    cur: u64,
    max: u64,
    done: bool,
}
impl Iterator for Elements {
    type Item = u64;
    #[inline]
    fn next(&mut self) -> Option<u64> {
        if self.done { return None; }
        if self.cur == self.max { self.done = true; }
        let ret = self.cur;
        let cur_lower_bits = BitSet64::full_set(self.cur + 1);
        let next_lower_bits = !self.set.0 | cur_lower_bits.0;
        self.cur = next_lower_bits.wrapping_add(1).trailing_zeros() as u64;
        Some(ret)
    }
}
// impl FusedIterator for Elements {}

pub struct Subsets {
    set: BitSet64,
    cur: BitSet64,
    done: bool,
}
impl Iterator for Subsets {
    type Item = BitSet64;
    #[inline]
    fn next(&mut self) -> Option<BitSet64> {
        if self.done { return None; }
        if self.cur.0 == self.set.0 { self.done = true; }
        let ret = self.cur;
        self.cur = BitSet64(self.set.0 & (self.cur.0.wrapping_sub(self.set.0)));
        Some(ret)
    }
}
// impl FusedIterator for Subsets {}

impl ::std::fmt::Display for BitSet64 {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{:b}", self.0)
    }
}


#[test]
fn test_bitset64() {
    let set = BitSet64::empty_set()
        .insert(2)
        .insert(5)
        .insert(6)
        .insert(8);
    assert_eq!(set.min(), Some(2));
    assert_eq!(set.max(), Some(8));
    assert_eq!(format!("{}", set), "101100100");
    println!("> {:>10}", set);
    for i in set.subsets() {
        println!("# {:>10}", i);
    }
    println!("> {:>10}", set);
    for i in set.elements() {
        println!("# {}", i);
    }
}

#[test]
fn test_bitset64_empty() {
    let set = BitSet64::empty_set();
    println!("{:?} {:?}", set.min(), set.max());
    println!("> {}", set);
    for i in set.subsets() {
        println!("# {}", i);
    }
}

#[test]
fn test_bitset64_full() {
    let set = BitSet64::full_set(3);
    println!("{:?} {:?}", set.min(), set.max());
    println!("> {}", set);
    for i in set.subsets() {
        println!("# {}", i);
    }
}
