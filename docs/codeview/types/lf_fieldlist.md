# `LF_FIELDLIST` (0x1203) - Field List

The `LF_FIELDLIST` record contains a list of fields defined on a type. The type
can be `LF_CLASS`, `LF_STRUCTURE`, `LF_INTERFACE`, `LF_UNION`, or `LF_ENUM`.

Each field within an `LF_FIELDLIST` record uses a leaf value to identify the
kind of field. However, the leaf values are disjoint from those used for type
records. Also, the field records do not use the same header as that used for
type records. The length of each field is not stored; it is implied by the leaf
value. For this reason, decoders must know how to decode all possible fields,
because if the decoder does not recognize a field then it cannot know the size
of the field.

This table summarizes the different field kinds. They use the same `LF_XXX`
naming convention, but these values are disjoint from the `LF_XXX` values used
for type records and should not be confused with them.

After the summary, each field is described in detail.

Value  | Name               | Description
-------|--------------------|-----------------------------------------------------------------
0x040b | `LF_FRIENDCLS`     | A friend class
0x1400 | `LF_BCLASS`        | A non-virtual (real) base class of a class.
0x1401 | `LF_VBCLASS`       | A directly-inherited virtual base class.
0x1402 | `LF_IVBCLASS`      | An indirectly-inherited virtual base class.
0x1404 | `LF_INDEX`         | An index to another `LF_FIELDLIST` which contains more fields
0x1409 | `LF_VFUNCTAB`      | A virtual function table pointer
0x140c | `LF_VFUNCOFF`      | A virtual function table pointer at a non-zero offset
0x1502 | `LF_ENUMERATE`     | An enumerator (named constant) defined on an `LF_ENUM` type
0x150c | `LF_FRIENDFCN`     | A friend function
0x150d | `LF_MEMBER`        | A non-static data member (field)
0x150e | `LF_STMEMBER`      | A static data member (static field)
0x150f | `LF_METHOD`        | A single method of a class, or a pointer to a `LF_METHODLIST`
0x1510 | `LF_NESTEDTYPE`    | A nested type
0x1511 | `LF_ONEMETHOD`     | A single non-overloaded method
0x1512 | `LF_NESTEDTYPEX`   | Nested type extended definition

## Field Attribute Structure

Many field types include an `attr` field that describes access protection and other properties. This is a 16-bit value with the following bit fields:

Bits  | Field        | Description
------|--------------|-------------------------------------------------------------
0-1   | `access`     | Access protection: 1=private, 2=protected, 3=public
2-4   | `mprop`      | Method properties (see below)
5     | `pseudo`     | Compiler-generated function that doesn't exist
6     | `noinherit`  | Class cannot be inherited
7     | `noconstruct`| Class cannot be constructed
8     | `compgenx`   | Compiler-generated function that does exist
9     | `sealed`     | Method cannot be overridden
10-15 | `unused`     | Unused bits

The `mprop` field (bits 2-4) encodes method properties:
- 0: Non-virtual method
- 1: Virtual method
- 2: Static method
- 3: Friend method
- 4: Introducing virtual method
- 5: Pure virtual method
- 6: Pure introducing virtual method

## Field Type Details

### `LF_BCLASS` (0x1400) - Base Class

```c
struct BClass {
    uint16_t attr;        // Field attributes
    TypeIndex type;       // Type index of the base class
    Number offset;        // Offset to base class subobject within derived class
};
```

Describes a non-virtual base class of a class. The `offset` specifies where the
base class subobject appears within instances of the derived class.

### `LF_VBCLASS` (0x1401) - Virtual Base Class (Direct)

```c
struct VBClass {
    uint16_t attr;        // Field attributes
    TypeIndex btype;      // Type of the virtual base class
    TypeIndex vbtype;     // Type of virtual base pointer
    Number vbpoff;        // Offset to virtual base pointer within derived class
    Number vboff;         // Offset within virtual base pointer table
};
```

Describes a virtual base class that is directly inherited by this class.

### `LF_IVBCLASS` (0x1402) - Virtual Base Class (Indirect)

```c
struct IVBClass {
    uint16_t attr;        // Field attributes
    TypeIndex btype;      // Type of the virtual base class
    TypeIndex vbtype;     // Type of virtual base pointer
    Number vbpoff;        // Offset to virtual base pointer within derived class
    Number vboff;         // Offset within virtual base pointer table
};
```

Describes a virtual base class that is indirectly inherited by this class
(inherited by a base class of this class).

### `LF_FRIENDFCN` (0x150c) - Friend Function

```c
struct FriendFcn {
    uint16_t padding;     // Padding (must be zero)
    TypeIndex type;       // Type of the friend function
    strz name;            // Name of the friend function
};
```

Describes a friend function declaration. Friend functions have access to private
and protected members of the class.

### `LF_INDEX` (0x1404) - Continuation Index

```c
struct Index {
    uint16_t padding;     // Padding (must be zero)
    TypeIndex index;      // Type index of another LF_FIELDLIST
};
```

Points to another `LF_FIELDLIST` record that contains additional fields. This
allows field lists to be chained together when they exceed the size limits of a
single record.

### `LF_MEMBER` (0x150d) - Data Member

```c
struct Member {
    uint16_t attr;        // Field attributes
    TypeIndex type;       // Type of the member
    Number offset;        // Offset within the containing type
    strz name;            // Name of the member
};
```

Describes a non-static data member of a class or structure. The `offset`
specifies where this member appears within instances of the containing type.

### `LF_STMEMBER` (0x150e) - Static Member

```c
struct STMember {
    uint16_t attr;        // Field attributes
    TypeIndex type;       // Type of the static member
    strz name;            // Name of the static member
};
```

Describes a static data member of a class or structure. Static members are
shared among all instances of the type and do not have an offset within
individual instances.

### `LF_METHOD` (0x150f) - Method List

```c
struct Method {
    uint16_t count;       // Number of overloads of this method
    TypeIndex mList;      // Type index of LF_METHODLIST containing the overloads
    strz name;            // Name of the method
};
```

Describes a method that may have multiple overloads. The `mList` field points to
an `LF_METHODLIST` record that contains information about each overload.

### `LF_ONEMETHOD` (0x1511) - Single Method

```c
struct OneMethod {
    uint16_t attr;        // Field attributes
    TypeIndex type;       // Type index of the method (LF_MFUNCTION or LF_PROCEDURE)
    uint32_t vbaseoff;    // Virtual base offset (present only if introduces virtual)
    strz name;            // Name of the method
};
```

Describes a single method without overloads. The `vbaseoff` field is present
only if the method introduces a new virtual function slot (when `mprop` is 4 or
6).

### `LF_NESTEDTYPE` (0x1510) - Nested Type

```c
struct NestedType {
    uint16_t padding;     // Padding (must be zero)
    TypeIndex type;       // Type index of the nested type
    strz name;            // Name of the nested type
};
```

Describes a type that is nested within another type. This is commonly used for
classes defined within other classes.

### `LF_NESTEDTYPEX` (0x1512) - Extended Nested Type

```c
struct NestedTypeEx {
    uint16_t attr;        // Field attributes
    TypeIndex type;       // Type index of the nested type
    strz name;            // Name of the nested type
};
```

An extended version of `LF_NESTEDTYPE` that includes attribute information.

### `LF_VFUNCTAB` (0x1409) - Virtual Function Table

```c
struct VFuncTab {
    uint16_t padding;     // Padding (must be zero)
    TypeIndex type;       // Type index of the virtual function table shape
};
```

Describes the virtual function table (vtable) for a class. The `type` field
points to an `LF_VFTSHAPE` record that describes the layout of the vtable.

### `LF_VFUNCOFF` (0x140c) - Virtual Function Offset

```c
struct VFuncOff {
    uint16_t padding;     // Padding (must be zero)
    TypeIndex type;       // Type index of the virtual function table type
    uint32_t offset;      // Offset within the virtual function table
};
```

Describes a virtual function table pointer that appears at a non-zero offset
within the containing type.

### `LF_ENUMERATE` (0x1502) - Enumerator

```c
struct Enumerate {
    uint16_t attr;        // Field attributes
    Number value;         // Value of this enumerator
    strz name;            // Name of this enumerator
};
```

Describes a named constant within an enumeration type. The `value` field
contains the integer value associated with this enumerator name.

### `LF_FRIENDCLS` (0x040b) - Friend Class

```c
struct FriendCls {
    uint16_t padding;     // Padding (must be zero)
    TypeIndex type;       // Type index of the friend class
};
```

Describes a friend class declaration. Friend classes have access to private and
protected members of the class that declares them as friends.
