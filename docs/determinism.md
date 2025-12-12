# Determinism and Normalization 

There are many benefits to determinism (aka reproducible builds).

> See: [Reproducible builds on Wikipedia](https://en.wikipedia.org/wiki/Reproducible_builds).

When a compiler produces an executable (assuming the compiler is deterministic),
the corresponding PDB should also be deterministic.

Throughout this document, as the file structures are described, many sections
will also specify what rules could be used to impose a deterministic order on a
given file structure. For example, a table might contain records that could
occur in any order. A determinism requirement would state one arbitrary sorting
order that could be applied to the table. The sorting order does not affect the
semantics of the information, but it does remove a degree of freedom from its
encoding.

These determinism requirements will be called out in each section that describes
a file structure. Again, determinism requirements are stronger than invariants.
For example, let’s assume we are dealing with a hash table, where each hash
record contains a hash code and a reference count field. Our correctness
requirement only governs the ordering of the records with respect to their hash
codes:

> Invariant: The `hash_records` array must be sorted in increasing order by hash
> code.

Unfortunately, this leaves us with an unwanted degree of freedom because two
consecutive hash records could have the same hash code but have different
reference counts. We specify this as a determinism requirement:

> Determinism: The `hash_records` array must also be sorted in increasing order
> by reference count.
