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
pub struct MZSP<'a> {
    values: &'a [isize],
    memo: MemoTables,
    remainder: BitSet64,
}
impl<'a> MZSP<'a> {
    /// Find a maximum zero-sum partitioning of the given values.
    pub fn compute(values: &'a[isize]) -> MZSP<'a> {
        MZSP {
            values: values,
            memo: MemoTables::new(values),
            remainder: BitSet64::full_set(values.len() as u64),
        }
    }
}
impl<'a> Iterator for MZSP<'a> {
    type Item = BitSet64;
    fn next(&mut self) -> Option<BitSet64> {
        let (n, partition) = if self.remainder == BitSet64::full_set(self.values.len() as u64) {
            let mut set = self.remainder.clone();
            let max = set.take_max().unwrap();
            max_zero_sum_partitions(&self.memo, self.values, set, max)
        } else {
            self.memo.get_mzsp(self.remainder)
        };
        if n == 0 {
            None
        } else {
            self.remainder = self.remainder.minus(partition);
            Some(partition)
        }
    }
}

struct MemoTables {
    mzsp_table: Vec<(usize, BitSet64)>,
    sum_table: Vec<isize>,
}

impl MemoTables {
    fn new(values: &[isize]) -> MemoTables {
        let mut tables = MemoTables {
            mzsp_table: vec![],
            sum_table: vec![],
        };

        for mut set in BitSet64::enumerate(values.len() as u64 - 1) {
            // Remove the max. element from `set`
            let max = match set.take_max() { Some(x) => x, None => {
                // Oh... `set` is empty.  Never mind!
                tables.sum_table.push(0);
                tables.mzsp_table.push((0, BitSet64::empty_set()));
                continue;
            }};
            // Compute the sum of `set ∪ {max}` (we'll need this later)
            let sum = values[max as usize] + tables.get_sum(set);
            tables.sum_table.push(sum);
            // Compute the mzsp of `set ∪ {max}`
            let mzsp = max_zero_sum_partitions(&tables, values, set, max);
            tables.mzsp_table.push(mzsp);
        }

        tables
    }

    /// Panics if `subset.max() > values.len()`.
    fn get_mzsp(&self, subset: BitSet64) -> (usize, BitSet64) {
        self.mzsp_table[subset.0 as usize]
    }

    /// Panics if `subset.max() > values.len()`.
    fn get_sum(&self, subset: BitSet64) -> isize {
        self.sum_table[subset.0 as usize]
    }
}

/// The maximum number of zero-sum partitions of `set ∪ {x}`, and a bitset representing the
/// partition which contains x.
fn max_zero_sum_partitions(memo: &MemoTables, values: &[isize], set: BitSet64, x: u64) -> (usize, BitSet64) {
    let mut best = (0, BitSet64::empty_set());
    // For all subsets i of `set`, check whether i ∪ {x} forms a zero-sum partition.  If it does,
    // check how many zero-sum partitions can be formed from set \ i.
    let neg_val = -(values[x as usize]);
    for i in set.subsets() {
        if memo.get_sum(i) == neg_val {
            // This subset cancels out our element exactly!  i ∪ {x} forms a zsp.
            let remainder = set.minus(i);
            let c = memo.get_mzsp(remainder);
            // c is the maximum number of partitions which the remainder can form.
            if c.0 >= best.0 {
                best = (c.0 + 1, i);
            }
        }
    }

    (best.0, best.1.insert(x))
}
