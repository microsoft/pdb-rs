# Hash functions

## PDB Hash Function: `LHashPbCb` (32-bit)

Several PDB tables use a hash function. The hash function is simplistic, and is
based on XOR’ing input bytes in groups of 4. For inputs whose length is not a
multiple of 4, the hash function XOR’s the last 1, 2, or 3 bytes, but there is
an accidental special-case that must be considered carefully.


The hash function takes a byte array and produces a uint32. It then divides that
value by a modulus (usually a hash bucket count) and returns the remainder.

To compute the hash function:

1. Let B be the input byte array, and L be its length. Let M be the modulus
   parameter (hash bucket count).
2. Let H be a uint32 value, and initialize it to zero.
3. Let L4 = L / 4, with integer division, i.e. drop the remainder. This gives
   the number of whole uint32 units that we will read.
4. Read the first L4 * 4 bytes from the input array, and read each group of 4
   bytes as a uint32 value (in little-endian byte order). For each uint32 value,
   XOR the value into H.
5. Examine the remaining bytes, after the L4 * 4 prefix.
   a. If there are no remaining bytes, do nothing.
   b. If there are 2 or 3 remaining bytes, then read the first 2 as a uint16
      value (in little-endian byte order), cast it to uint32, and XOR it into H.
   c. If there are 3 remaining bytes or 1 remaining byte, then read the last
      byte and cast it to a uint32 value, in the low bits. XOR this value into H.
6. Set bit 5 in each byte of H to 1. That is, set bits 5, 13, 21, and 29 to 1.
   This is done to support case-insensitive hashing.
7. Set H = H XOR (H >> 11).
8. Set H = H XOR (H >> 16).
9. Divide H by M and return the remainder as the result of the has function.

## Hash function: `SigForPbCb`

PDB uses CRC-32 for hashing some strings. It is the standard, well-known CRC-32 algorithm.

## References

* [`HashPbCb`](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/langapi/shared/crc32.h#L8)

* [`SigForPbCb`](https://github.com/microsoft/microsoft-pdb/blob/master/langapi/shared/crc32.h)
