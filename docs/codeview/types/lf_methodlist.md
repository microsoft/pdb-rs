# `LF_METHODLIST` (0x1206) - Method List

```c
struct MethodList {
    MethodEntry methods[];
};

struct MethodEntry {
    uint16_t attribute;
    uint16_t padding;
    TypeIndex type;
    uint32_t vtab_offset;     // This field is only present if 'attribute' introduces a new vtable slot
};
```

The `LF_METHODLIST` record contains a list of method overloads that share the same name. This record is referenced by [`LF_METHOD`](lf_fieldlist.md) fields within `LF_FIELDLIST` records to describe all the different overloads of a method.

## Structure

The record consists of a variable-length array of `MethodEntry` structures. Each entry describes one method overload with the same name but potentially different signatures, access levels, or virtual characteristics.

## Fields

### MethodEntry Fields

`attribute` is a 16-bit field containing method properties and access control. This follows the same bit field structure as other field attributes:

Bits  | Field        | Description
------|--------------|------------------------------------------------------------
0-1   | `access`     | Access protection: 1=private, 2=protected, 3=public
2-4   | `mprop`      | Method properties (see below)
5     | `pseudo`     | Compiler-generated function that doesn't exist
6     | `noinherit`  | Method cannot be inherited
7     | `noconstruct`| Method cannot be used for construction
8     | `compgenx`   | Compiler-generated function that does exist
9     | `sealed`     | Method cannot be overridden
10-15 | `unused`     | Unused bits

The `mprop` field (bits 2-4) encodes method properties:
- 0: Non-virtual method
- 1: Virtual method  
- 2: Static method
- 3: Friend method
- 4: Introducing virtual method (introduces new vtable slot)
- 5: Pure virtual method
- 6: Pure introducing virtual method

`padding` is a 16-bit padding field that must be zero.

`type` is a `TypeIndex` pointing to the method's type. This typically points to an `LF_MFUNCTION` record that describes the method's signature, calling convention, and parameter types.

`vtab_offset` is a 32-bit offset within the virtual function table. This field is **only present** when the method introduces a new virtual function slot (when `mprop` is 4 or 6). For non-virtual methods or virtual methods that override existing slots, this field is omitted entirely.

## Usage

`LF_METHODLIST` is used to group method overloads together. When a class has multiple methods with the same name but different signatures (overloaded methods), a single `LF_METHOD` field in the class's `LF_FIELDLIST` points to an `LF_METHODLIST` that contains all the overloads.

For example, a C++ class with multiple constructors:
```cpp
class MyClass {
public:
    MyClass();                    // Default constructor
    MyClass(int x);              // Int constructor  
    MyClass(const std::string& s); // String constructor
};
```

Would have a single `LF_METHOD` field named "MyClass" that points to an `LF_METHODLIST` containing three `MethodEntry` records, one for each constructor overload.

## Iteration

The Rust implementation provides an iterator interface for parsing method list entries:

```rust
let mut method_list = MethodList::parse(record_data)?;
while let Some(entry) = method_list.next()? {
    println!("Method type: {:?}, Access: {}", entry.ty, entry.attr & 0x3);
    if let Some(offset) = entry.vtab_offset {
        println!("  Virtual table offset: {}", offset);
    }
}
```

The iterator automatically handles the conditional presence of the `vtab_offset` field based on the method attributes.

## Virtual Method Handling

The presence of the `vtab_offset` field depends on whether the method introduces a new virtual function slot. This is determined by examining bits 2-4 of the `attribute` field:

- If `mprop` is 4 (introducing virtual) or 6 (pure introducing virtual), the `vtab_offset` field is present
- For all other method types, the `vtab_offset` field is absent

This conditional field presence is why decoders must check the attribute bits before attempting to read the virtual table offset.
