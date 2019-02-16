repay: make efficient repayments
================================

Repay is a simple tool which computes the most efficient way for everyone
to repay their debts.  You feed it a list of historical transactions:

    { "from":"Jane", "to":"Fred", "amt":7200 }
    { "from":"Rémy", "to":"Alex", "amt":7300 }
    { "from":"Fred", "to":"Mike", "amt":7800 }
    { "from":"Fred", "to":"Mike", "amt":4500 }
    { "from":"Mike", "to":"Alex", "amt":5600 }
    { "from":"Jane", "to":"Fred", "amt":9300 }
    { "from":"Mike", "to":"Rémy", "amt":6100 }
    { "from":"Alex", "to":"Jane", "amt":2400 }
    { "from":"Mike", "to":"Jane", "amt":6400 }
    { "from":"Jane", "to":"Rémy", "amt":9300 }

And it spits out a minimal list of transactions which will make everyone
square:

    { "from":"Fred", "to":"Mike", "amt":4200 }
    { "from":"Rémy", "to":"Jane", "amt":8100 }
    { "from":"Alex", "to":"Mike", "amt":1600 }
    { "from":"Alex", "to":"Jane", "amt":8900 }

INSTALLATION
------------

Repay is written in Rust.  Install it using cargo:

    cargo install --git https://git.sr.ht/~asayers/repay

USAGE
-----

    repay [-x|-a] [-v] <PATH>

        <PATH>    The ledger containing historical transactions (see INPUT FORMAT)
        -x        Guarantees a minimal solution (see EXACT MODE)
        -a        Use a faster algorithm (see APPROXIMATE MODE)
        -v        Verbose output

If neither -x nor -a are specified, repay tries to guess the best
algorithm to use based on the ledger (see PERFORMANCE CONSIDERATIONS
for the heuristic).

INPUT FORMAT
------------

As you can see, transactions are newline-delimited JSON objects.
Each entry must contain the keys "from", "to", and "amt"; they may
contain additional fields which repay will ignore:

    { "date":"2018-02-17", "from":"Jane", "to":"Fred", "amt":7200, "rsn":"Paid for lunch" }
    { "date":"2018-02-19", "from":"Rémy", "to":"Alex", "amt":7300, "rsn":"Bet on football" }
    { "date":"2018-02-19", "from":"Fred", "to":"Mike", "amt":7800, "rsn":"Undisclosed" }
    { "date":"2018-02-20", "from":"Jane", "to":"Fred", "amt":9300, "rsn":"Delivery man" }
    { "date":"2018-02-19", "from":"Alex", "to":"Rémy", "amt":7300, "rsn":"Settlement" }

The output format is the same (of course, it only contains the keys
"from", "to", and "amt").  The overhead of additional fields is minimal,
so feel free to go crazy:

    { "from":"Mike", "to":"Jane", "amt":6400, "sig":"RWS1l0sm+eG0IZ7/JZ7ELwB932C1rdllJwQ=" }

"amt" must be an integer, so you'll probably want to record all amounts
in a small currency such as cents, pence, or yen.

PERFORMANCE CONSIDERATIONS
--------------------------

The time taken to parse the ledger is linear in its length.  On my
machine, a million entries takes ~30s to parse.  If your ledger grows to
the point where this becomes a problem, you might consider compacting it.
(You can do this by running repay, negating all the amounts, and using
this as the start of your new ledger.)

Repay implements two algorithms for building a repayment plan: "exact
mode", which is guaranteed to give an optimal plan, but which becomes
very slow when there are many people involved; and "approximate mode"
which scales better but may return a non-optimal plan.  By default repay
switches from exact mode to approx mode when the number of people passes
20, but you can control it using flags (see USAGE).

In exact mode, solving time scales exponentially with the number of
people involved - specifically, each additional person multiplies the run
time by 3.  Things can get bad very quickly:  on my machine, a ledger
mentioning 20 people takes half a second to solve;  with 25 people it
takes around 2 minutes.  If you have lots of people with interconnected
debts, you're stuck using approximate mode.

In approximate mode, solving time scales with the total amount of debt
(ie.  the abs-sum of everyone's balances).  This shouldn't be a problem
in practice.

APPROXIMATE MODE
----------------

We have a multiset of integers, representing people's outstanding
balances.  We form a graph by drawing edges from the positive nodes to the
negative nodes.  (Trivia: graph theorists call this shape a "biclique".)
We then solve for minimum-cost-maximum-flow, where the cost of each edge
is the same.  The non-zero edges, and the amount which flows across them,
form our repayment plan.

In some cases, this algorithm will yield a plan with too many repayments:

    $ repay pathological.ledger -va
    INFO: Computed repayment plan in 0.000s: 16 repayments required
    $ repay pathological.ledger -vx
    INFO: Computed repayment plan in 0.006s: 9 repayments required

There's no efficient way to tell whether a repayment plan is optimal
or not.

EXACT MODE
----------

For an arbitrary multiset, you will need between n/2 and (n-1) edges to
facilitate a complete flow.  Our task is to minimise the number of edges,
without reducing the total flow.

This problem is equivalent to the "maximal zero-sum partitioning"
(MZSP) problem:

> Given a multiset of integers X, such that ∑(X)=0, partition X into
> the maximum number of subsets so that every subset also sums to zero.

Why is it equivalent?  If you partition your original multiset into
zero-sum subsets, you can independently draw a graph for each subset - a
graph containing at most n-1 edges.  Therefore, if you can form m subsets
then you can bring the number of edges in the combined graph down to n-m.
On the other hand, suppose there is a solution with n-m edges; then
there must exist a partitioning with m zero-sum subsets.  (TODO: prove)
Therefore, if you find a MZSP, and for each partition you construct a
flow-maximising graph, then the combined graph must be optimal.

Given a zero-sum multiset, a flow-maximising graph with n-1 edges can
be constructed in O(n) using a simple greedy algorithm.  This means that
the real problem is finding the MZSP.

repay's implementation of MZSP is O(3^n).  This is painful, but given
that MZSP is NP-hard, I don't expect it's possible to do much better.
The algorithm we use is due to Akio Takano.

See also:

 * https://en.wikipedia.org/wiki/Subset_sum_problem
 * https://math.stackexchange.com/questions/339148/maximal-zero-sums-partition

PRIOR ART
---------

* Tom VERHOEFF
  Settling Multiple Debts Efficiently: An Invitation to Computing Science
  https://pdfs.semanticscholar.org/4f51/25bc51b61052370bbd73297f83d248545856.pdf

It seems to contain plenty of relevant investigation.  Verhoeff also
relates efficient repayment to the subset sum problem.

LICENCE
-------

This software is in the public domain.  See UNLICENSE for details.
