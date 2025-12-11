# `LF_VTSHAPE` (0x000a)

```c
struct VTShape {
    uint16_t count;
    // An array of 4-bit descriptors follow, whose size is given by 'count'
};
```

Defines the format of a virtual function table. This record is accessed by the
`vfunctabptr` in the member list of the class which introduces the virtual
function. The `vfunctabptr` is defined either by the `LF_VFUNCTAB` or
`LF_VFUNCOFF` member record. If `LF_VFUNCTAB` record is used, then `vfunctabptr`
is at the address point of the class. If `LF_VFUNCOFF` record is used, then
`vfunctabptr` is at the specified offset from the class address point. The
underlying type of the pointer is a `VTShape` type record. This record describes
how to interpret the memory at the location pointed to by the virtual function
table pointer.

`count` specifies the number of descriptors. Each value in `descriptor`
describes an entry in the virtual function table. Each descriptor is 4 bits and
can take one of the following values:

Value       | Description
------------|------------
0           | Near
1           | Far
2           | Thin
3           | address point displacement to outermost class. This is at `entry[-1]` from table address.
4           | far pointer to metaclass descriptor. This is at `entry[-2]` from table address.
5           | Near32
6           | Far32
7-15        | reserved
