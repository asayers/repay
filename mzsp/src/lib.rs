/*!
The "maximal zero-sum partitioning" problem may be stated thus:

> Given a multiset of numbers *X*, such that ∑(*X*) =0, partition *X* into the maximum number of
> subsets so that each subset sums to zero

This crate implements a dynamic programming-based solution to this problem.

```
# use mzsp::*;
let partitionable   = vec![10, -10, 15, -15];
let unpartitionable = vec![10, 20, -15, -15];

assert_eq!(mzsp(&partitionable),   vec![vec![15, -15], vec![10, -10]]);
assert_eq!(mzsp(&unpartitionable), vec![vec![10, 20, -15, -15]]);
```

The most flexible and efficient way to use this crate is to use the `MZSP` iterator.  The `mzsp`
function is a convenience function.
*/

extern crate bitset64;
use bitset64::*;

// For each element of the multiset, we try to match it with one of the other members.  If there's
// a match, we eliminate both elements and return 1 + mzsp(remainder).  If there's no match, then
// for every other member, we add the value of our node to it and compute mzsp for the resulting
// multiset.

/// Maximal zero-sum partitioning of a multiset.  This is a handy wrapper around `MZSP`.
pub fn mzsp(values: &[isize]) -> Vec<Vec<isize>> {
    MZSP::compute(values).map(|partition|
        partition.elements().map(|idx|
            values[idx as usize]
        ).collect()
    ).collect()
}

/// A partitioning of a multiset of integers, such that every partition sums to zero.
///
/// A partitioning given by `MZSP::compute` is guaranteed to be maximal, in the sense that is no
/// zero-sum partitioning with more partitions.
///
/// `MZSP` allows you to iterate over the partitions, which are represented by guaranteed-non-empty
/// `BitSet64`s.  The elements of the bitsets are indices into the original multiset.  Use it like
/// this:
///
/// ```
/// # use mzsp::*;
/// # let values = vec![];
/// for partition in MZSP::compute(&values) {
///     for idx in partition.elements() {
///         let x = values[idx as usize];
///         /* do something with x */
///     }
/// }
/// ```
pub struct MZSP {
    mzsp_table: MZSPMemoized,
    remainder: BitSet64,
}
impl MZSP {
    /// Find a maximum zero-sum partitioning of the given values.
    pub fn compute(values: &[isize]) -> MZSP {
        MZSP {
            mzsp_table: MZSPMemoized::precompute(values),
            remainder: BitSet64::full_set(values.len() as u64),
        }
    }
}
impl Iterator for MZSP {
    type Item = BitSet64;
    fn next(&mut self) -> Option<BitSet64> {
        let (n, partition) = self.mzsp_table.get(self.remainder);
        if n == 0 {
            None
        } else {
            self.remainder = self.remainder.minus(partition);
            Some(partition)
        }
    }
}

struct MZSPMemoized(Vec<(usize, BitSet64)>);
impl MZSPMemoized {
    pub fn precompute(values: &[isize]) -> MZSPMemoized {
        let subset_sum_table = SubsetSumMemoized::precompute(values);
        let mut table = MZSPMemoized(vec![]);
        for mut set in BitSet64::enumerate(values.len() as u64) {
            let x = max_zero_sum_partitions(&table, values, &subset_sum_table, set);
            table.0.push(x);
        }
        table
    }

    /// Panics if `subset` includes an element not contained in `values`.
    pub fn get(&self, subset: BitSet64) -> (usize, BitSet64) {
        self.0[subset.0 as usize]
    }
}

/// Get the max. number of zero-sum partitions for the given set, and a bitset representing one of
/// those partitions.
fn max_zero_sum_partitions(memo_table: &MZSPMemoized, values: &[isize], subset_sum_table: &SubsetSumMemoized, mut set: BitSet64) -> (usize, BitSet64) {
    // Take the top element from the set
    let x = match set.take_max() { Some(x) => x, None => return (0, BitSet64::empty_set()), };

    let mut best = (0, BitSet64::empty_set());
    for i in set.subsets() {
        if subset_sum_table.get(i) == -(values[x as usize]) {
            // This subset cancels out our element exactly!  i ∪ {x} forms a partition.
            let remainder = set.minus(i);
            let c = memo_table.get(remainder);
            // c is the maximum number of partitions which the remainder can form.
            if c.0 >= best.0 {
                best = (c.0 + 1, i);
            }
        }
    }

    (best.0, best.1.insert(x))
}



/// A lookup table for the function `subset_sum`
struct SubsetSumMemoized(Vec<isize>);
impl SubsetSumMemoized {
    pub fn precompute(values: &[isize]) -> SubsetSumMemoized {
        let mut table = SubsetSumMemoized(vec![]);
        for set in BitSet64::enumerate(values.len() as u64) {
            let val = subset_sum(&table, values, set);
            table.0.push(val);
        }
        table
    }

    /// Panics if `subset` includes an element not contained in `values`.
    pub fn get(&self, subset: BitSet64) -> isize {
        self.0[subset.0 as usize]
    }
}

fn subset_sum(memo_table: &SubsetSumMemoized, values: &[isize], mut subset: BitSet64) -> isize {
    match subset.take_max() {
        None => 0,
        Some(max) => values[max as usize] + memo_table.get(subset),
    }
}
