- [TPI Stream: Type Database (Fixed Stream 2)](#tpi-stream-type-database-fixed-stream-2)
- [Type Stream Header](#type-stream-header)
- [Type Index Range](#type-index-range)
- [Type Records Substream](#type-records-substream)
- [Hash Stream](#hash-stream)
- [Hash Value Substream](#hash-value-substream)
  - [Example](#example)
- [Type Record Hash Function](#type-record-hash-function)
- [Hash Index Substream](#hash-index-substream)
- [Example](#example-1)
- [Hash Adjustment Substream](#hash-adjustment-substream)

# TPI Stream: Type Database (Fixed Stream 2)

Each PDB contains a TPI Stream, also called the Type Database.  The TPI Stream is always stored in stream 2. The TPI is one of the central data structures of the PDB; many other structures refer to it, using `TypeIndex` values.

Symbol records stored in the Global Symbol Stream (GSS) and in module symbol streams refer to type definitions stored in the TPI Stream using `TypeIndex` values.

The TPI permits only this restricted set of type records:

Record Kind<br>Value (hex) | Record Kind Name | Description
-------|-----------------|----------------------------------
0x000A | `LF_VTSHAPE`    | Virtual function table shape
0x1001 | `LF_MODIFIER`   | Modifies another type
0x1002 | `LF_POINTER`    | Pointer to another type
0x1008 | `LF_PROCEDURE`  | Procedure type
0x1009 | `LF_MFUNCTION`  | Member function (of class/struct)
0x1201 | `LF_ARGLIST`    | Argument list for LF_PROCEDURE, LF_MFUNCTION, LF_METHODLIST
0x1203 | `LF_FIELDLIST`  | Complex list describing the fields within an LF_CLASS, LF_STRUCTURE, LF_UNION, or LF_ENUM.
0x1205 | `LF_BITFIELD`   | Bitfield of an LF_UNION, LF_STRUCTURE, or LF_CLASS
0x1206 | `LF_METHODLIST` | Specifies a list of methods defined on a type
0x1503 | `LF_ARRAY`      | A fixed-size array of elements
0x1504 | `LF_CLASS`      | A "class" type
0x1505 | `LF_STRUCTURE`  | A "struct" type
0x1506 | `LF_UNION`      | A "union" type
0x1507 | `LF_ENUM`       | An "enum" type

# Type Stream Header

Each Type Stream (both TPI and IPI) consists of a header, followed by a series of variable-length type records. 

The header:

```
// sizeof = 56
struct TypeStreamHeader {
  uint32_t version;
  uint32_t header_size;

  // Fields for Type Records
  uint32_t type_index_begin;
  uint32_t type_index_end;
  uint32_t type_record_bytes;

  // Fields for Type Hash Stream
  uint16_t hash_stream_index;
  uint16_t hash_aux_stream_index;
  uint32_t hash_key_size;
  uint32_t num_hash_buckets;
  int32_t  hash_value_buffer_offset;
  uint32_t hash_value_buffer_length;
  int32_t  index_offset_buffer_offset;
  uint32_t index_offset_buffer_length;
  int32_t  hash_adj_buffer_offset;
  uint32_t hash_adj_buffer_length;
}
```

An example of a Type Stream Header:

```
00000000 : 0b ca 31 01 38 00 00 00 00 10 00 00 af 1a 04 00 : ..1.8...........
00000010 : 2c 6d ed 00 25 09 ff ff 04 00 00 00 ff ff 03 00 : ,m..%...........
00000020 : 00 00 00 00 bc 2a 10 00 bc 2a 10 00 80 3a 00 00 : .....*...*...:..
00000030 : 3c 65 10 00 00 00 00 00 06 00 01 12 00 00 00 00 : <e..............
00000040 : 0e 00 08 10 03 00 00 00 00 00 00 00 00 10 00 00 : ................
00000050 : 7e 00 03 12 02 15 03 00 00 00 50 6f 77 65 72 55 : ~.........PowerU
00000060 : 73 65 72 50 72 65 73 65 6e 74 00 f1 02 15 03 00 : serPresent......
00000070 : 01 00 50 6f 77 65 72 55 73 65 72 4e 6f 74 50 72 : ..PowerUserNotPr
00000080 : 65 73 65 6e 74 00 f2 f1 02 15 03 00 02 00 50 6f : esent.........Po
00000090 : 77 65 72 55 73 65 72 49 6e 61 63 74 69 76 65 00 : werUserInactive.
000000a0 : 02 15 03 00 03 00 50 6f 77 65 72 55 73 65 72 4d : ......PowerUserM
000000b0 : 61 78 69 6d 75 6d 00 f1 02 15 03 00 03 00 50 6f : aximum........Po
000000c0 : 77 65 72 55 73 65 72 49 6e 76 61 6c 69 64 00 f1 : werUserInvalid..
```

The `header_size` field specifies the actual size of this header. The Type Records immediately follow the Type Stream Header, at the offset given by `header_size`. Readers should use the `header_size` value, rather than assuming that the header has a fixed (compile-time) size.

> Invariant: The `header_size` field cannot be less than 56.

> Invariant: The `header_size` field must be a multiple of 4. This is required so that the type records begin on an aligned boundary.

> Invariant: Although the `hash_value_buffer_length`, `index_offset_buffer_length`, and `hash_adj_buffer_length` fields are typed as signed integers, these fields should never be negative.
Version

The version field specifies the structure of the Type Stream. These are the defined values for version, in decimal. The names and values come from the `/PDB/dbi/tpi.h` header file [MS-PDB].

Name             | Value (decimal) | Description
-----------------|----------|-----------
`impv40`         | 19950410 |
`impv41`         | 19951122 |
`impv50Interim`  | 19960307 |
`impv50`         | 19961031 |
`impv70`         | 19990903 |
`impv80`         | 20040203 | The current version. Uses `uint32_t` for the hash keys, and uses CRC-32 for the hashes.

All decoders should expect `impv80` version. All encoders should generate `impv80`.

# Type Index Range

The `type_index_begin` and `type_index_end` fields specify the range of Type Index values within the type stream, and implicitly specify the number of type records in the Type Stream.

The `type_index_begin` value specifies the Type Index of the first type record stored in the stream. Type Index values that are less than `type_index_begin` are reserved for primitive types, such as void, unsigned int, etc.  The value of `type_index_begin` must be greater than or equal to 4096 (0x1000), because the first 4096 values are reserved for encoding primitive types. In fact, all observed PDBs use a value of 4096 for `type_index_begin`; encoders should always use a value of 4096, because many decoders may assume a fixed value.

`type_index_begin` is a very important parameter. Interpreting many data structures within the PDB requires knowing the value of `type_index_begin`.

> Invariant: `type_index_begin` must be greater than or equal to 4096.

The `type_index_end` field specifies the exclusive upper bound of Type Index values. Thus, the number of type records is equal to `type_index_end - type_index_begin`.

> Invariant: `type_index_end` must be greater than or equal to `type_index_begin`.

> Invariant: The number of type records stored in the Type Stream must be equal to `type_index_end - type_index_begin`.

# Type Records Substream

After the Type Stream Header, the rest of the data in the Type Stream stores Type Records. The `type_record_bytes` field specifies the length of the type records, in bytes. The type records are stored within the Type Stream, starting at the byte offset given by `header_size`.

> Invariant: type_record_bytes must be a multiple of 2.

> Invariant: All type records begin on a 2-byte aligned boundary, within the stream.

> Invariant: header_size + type_record_bytes must be less than or equal to the size of the Type Stream

In all PDBs observed, `header_size + type_record_bytes` is equal to the size of the Type Stream. It would be possible to store additional data at the end of the Type Stream after the Type Records, but this has not been observed in practice.

> Determinism: No extra data should be written to the Type Stream after the Type Records. The size of the Type Stream must be equal to header_size + type_record_bytes.

Type records are variable-length records. Each record begins with a 4-byte header, which specifies the length and the "kind" of the record.

```
struct TypeRecordHeader {
  uint16_t size;
  uint16_t kind;
  // followed by kind - 2 bytes of kind-specific data
}
```

The size field specifies the size in bytes of the record. The size field does not count the size field itself, but it does count the kind field and the payload bytes.

> Invariant: The TypeRecordHeader.size field must be a multiple of 2, and must be greater than or equal to 2.
Type records are aligned at 2-byte boundaries. Unfortunately, many type records contain fields that have 4-byte alignment, such as uint32_t. Encoders and decoders must handle misaligned access to those fields, either using unaligned memory accesses or must copy the entire record to a buffer that has a guaranteed alignment.

The kind field specifies how to interpret a type record. In the PDB documentation, this kind field uses the "Leaf Type" enumeration. The details of these records are outside of the scope of this document. See these references:

* LLVM: CodeView Type Records
* `cvinfo.h` in MS public and internal sources, specifically the `LEAF_ENUM_e` type. (`cvinfo.h` in microsoft-pdb public GitHub repo)

This is an example of a single type record:

```
000000b0 : 61 78 69 6d 75 6d 00 f1 02 15 03 00 03 00 50 6f : aximum........Po
000000c0 : 77 65 72 55 73 65 72 49 6e 76 61 6c 69 64 00 f1 : werUserInvalid..
000000d0 : 46 00 07 15 05 00 00 02 74 00 00 00 02 10 00 00 : F.......t.......
000000e0 : 5f 55 53 45 52 5f 41 43 54 49 56 49 54 59 5f 50 : _USER_ACTIVITY_P
000000f0 : 52 45 53 45 4e 43 45 00 2e 3f 41 57 34 5f 55 53 : RESENCE..?AW4_US
00000100 : 45 52 5f 41 43 54 49 56 49 54 59 5f 50 52 45 53 : ER_ACTIVITY_PRES
00000110 : 45 4e 43 45 40 40 00 f1 ba 00 03 12 02 15 03 00 : ENCE@@..........
00000120 : 01 00 4a 4f 42 5f 4f 42 4a 45 43 54 5f 4e 45 54 : ..JOB_OBJECT_NET
00000130 : 5f 52 41 54 45 5f 43 4f 4e 54 52 4f 4c 5f 45 4e : _RATE_CONTROL_EN
```

It begins with the length (0x0046) and the kind (0x1507). In the CodeView documentation, 0x1507 identifies an `LF_ENUM` type record. Decoding that record shows that it points to a field list (another type record) using type index 0x1002, and that its name is `_USER_ACTIVITY_PRESENCE`.

To ensure that records are padded to 2-byte alignment, MS PDB writers often insert 0xF1 bytes at the end of type records. This is visible at offset 0x117 in the example above. The 0xF1 byte is included in the record length, so the code that finds record boundaries does not need to know about the 0xF1 byte. When decoding records, implementations must ignore these 0xF1 bytes. When encoding records, implementations should use a value of 0xF1 to pad record payloads to an alignment boundary of 2. 

# Hash Stream

The TPI Stream may have a corresponding TPI Hash Stream. The TPI Hash Stream contains several forms of lookup tables that point into the TPI Stream, which enables faster searching.

The TPI Stream Header specifies the stream number of the TPI Hash Stream in the `hash_stream_index` field of the TPI Stream Header. The reserved value 0xFFFF means that there is no hash stream. All observed PDBs have a valid hash stream.

The Type Hash Stream contains the following substreams:

* Hash Value Buffer
* Index Offset Buffer
* Hash Adjustment Buffer

The byte offset within the Hash Stream and length of each of these substreams is specified in the TPI Stream Header, not in the Type Hash Stream itself.

Because the Type Stream Header explicitly specifies the starting offset of each of the substreams, encoders have a degree of freedom in the order of the substreams. Observationally, the MSVC linker writes these substreams in the order given above (Hash Value Buffer, Index Offset Buffer, Hash Adjustment Buffer), with no gaps between them. Also, if a given substream has a length of zero (which is common for the Hash Adjustment Buffer), then the MSVC linker will still write the offset where the substream would have been written, rather than writing a meaningless value for the offset.

PDB encoders should use the same behavior as the MSVC linker:

> Determinism: Encoders should write the substreams in the order specified above, with no gaps between the substreams.

> Determinism: If a substream is zero-length, then encoders should still write an offset (into the Type Stream Header) where the substream would logically be (i.e. the end of the previous substream).

# Hash Value Substream

The Hash Value Substream is a substream of the Type Hash Stream. It contains hash values for all type records. It is not clear what the purpose of the Hash Value Substream is.

The Type Stream Header specifies the stream offset and size of the Hash Value Substream.  The Hash Value Substream is optional; it may be empty (zero-length) even if the Type Stream contains type records. However, if the Hash Value Substream is present, then it must contain the same number of entries as the number of type records in the Type Stream.

> Invariant: If `TypeStreamHeader.hash_value_buffer_length` is non-zero then it must be a multiple of `hash_key_size` and its value must be `num_type_records * hash_key_size`, where `num_type_records` is `type_index_end – type_index_end`.

The number of hash values in the Hash Value Substream is equal to `hash_value_buffer_length / hash_key_size`. This value should either be zero, or should be equal to the number of types in the Type Record Stream. Each value corresponds to a type record, by ordering hash values and type records sequentially. The hash values are computed using CRC-32 (with an initializer of zero) over the bytes of the entire type record. This includes the type record's size field, kind field, and record payload.

## Example

Here are examples of the first three type records and their matching hashes from the Type Hash Stream.

Record #0 (TypeIndex = 0x1000):
 
The matching value from the Type Hash Stream:
 
Record #1 (TypeIndex = 0x1001):
 
The matching value from the Type Hash Stream:
 
Record #2 (TypeIndex = 0x1002):
 
The matching value from the Type Hash Stream:

> TODO: Observationally, we find many records whose computed CRC-32 value does not match their stored value, but many that do match. Figure out why.

# Type Record Hash Function

The Hash Value Substream contains hash values computed for each type record. The hash function is complex because the function depends on the type record kind. (See `TPI1::hashPrec` defined in `tpi.cpp` in [MS-PDB].)

* For global UDT definitions, use the `LHashPbCb` function over the `name` field of the UDT type. The definition of a "global UDT definition" is moderately complicated; see `REC::fIsGlobalDefnUdt` in `tpi.cpp`.
  + The type kind must be one of `LF_ALIAS`, `LF_CLASS`, `LF_STRUCTURE`, `LF_UNION`, `LF_ENUM`, or `LF_INTERFACE`.
  + If the type kind is LF_ALIAS, then the record is a global UDT definition.
  + If the type kind is other than LF_ALIAS, then these requirements apply:
* The type record must not be a forward declaration. Forward declarations are identified by a bit set in the property field of the type record. All of the relevant types have the same property bitmask set the same offset, which makes testing this requirement easy.
* The type record must not have the scoped bit set in its property bitmask.
* The type record must not be an "anonymous UDT".
   + The type name must not be `<unnamed-tag>` or `__unnamed`.
   + The type name must not end with `::<unnamed-tag>` or `::__unnamed`.

> TODO: Expand this section and fully specify the behavior of `TPI1::hashPrec`. That function also uses CRC-32 for hashing some names.

# Hash Index Substream

The Hash Index Substream is stored in the Type Hash Stream. It consists of an array of fixed-size records with this definition:

```
// sizeof = 8
struct HashIndexPair {
  uint32_t type_index;
  uint32_t stream_offset;
};
```

This array of `HashIndexPair` structures is sorted in strictly-increasing order by `type_index`. Each entry specifies the offset within the Type Records Substream of the corresponding type record.  That is, you will need to add the header_size field to stream_offset to get the absolute byte offset within the Type Stream.

Observationally, the Hash Index Substream is usually fairly small. There is no clear pattern to the increases in the type_index of `stream_offset` fields. Typical values observed are an increase of 80 to 100 Type Index values, and 8000 to 10000 bytes for stream_offset.

The first entry in the array must have `type_index == type_index_begin`, and must have `stream_offset == 0`.

> Invariant: The byte size of the Hash Index Substream is a multiple of 8.

> Invariant: The entries in Hash Index Substream are ordered with type_index strictly increasing, and stream_offset strictly increasing. 

# Example

This is an example of the Hash Index Substream. The Hash Index Substream is highlighted; it is preceded (in this example) by the Hash Value Substream.
 
The first few entries of the Hash Index Substream decode as:

Type Index  | Stream Offset
------------|--------------
0x0000_1000 | 0x0000_0000
0x0000_101F | 0x0000_4022
0x0000_102F | 0x0000_8450
0x0000_1056 | 0x0000_604C
0x0000_10D0 | 0x0000_81CC
0x0000_10FC | 0x0000_A034
0x0000_1157 | 0x0000_C04C

# Hash Adjustment Substream

The purpose and structure of the Hash Adjustment Substream is not well-understood. [LLVM] suggests that it is related to Edit-and-Continue.
