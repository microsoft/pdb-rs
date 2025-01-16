/// Identifies type records. Also called "leaf" records.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Leaf(pub u16);

macro_rules! cv_leaf {
    (
        $(
            $code:expr, $name:ident ;
        )*
    ) => {
        #[allow(non_upper_case_globals)]
        #[allow(missing_docs)]
        impl Leaf {
            $(
                pub const $name: Leaf = Leaf($code);
            )*
        }

        static LEAF_NAMES: &[(Leaf, &str)] = &[
            $(
                (Leaf($code), stringify!($name)),
            )*
        ];
    }
}

cv_leaf! {
    0x0001, LF_MODIFIER_16t;
    0x0002, LF_POINTER_16t;
    0x0003, LF_ARRAY_16t;
    0x0004, LF_CLASS_16t;
    0x0005, LF_STRUCTURE_16t;
    0x0006, LF_UNION_16t;
    0x0007, LF_ENUM_16t;
    0x0008, LF_PROCEDURE_16t;
    0x0009, LF_MFUNCTION_16t;
    0x000a, LF_VTSHAPE;
    0x000c, LF_COBOL1;
    0x000e, LF_LABEL;
    0x000f, LF_NULL;
    0x0014, LF_ENDPRECOMP;
    0x020c, LF_REFSYM;
    0x040b, LF_FRIENDCLS;   // (in field list) friend class
    0x1001, LF_MODIFIER;
    0x1002, LF_POINTER;
    0x1008, LF_PROCEDURE;
    0x1009, LF_MFUNCTION;
    0x100a, LF_COBOL0;
    0x100b, LF_BARRAY;
    0x100d, LF_VFTPATH;
    0x100f, LF_OEM;
    0x1011, LF_OEM2;
    0x1200, LF_SKIP;
    0x1201, LF_ARGLIST;
    0x1203, LF_FIELDLIST;
    0x1204, LF_DERIVED;
    0x1205, LF_BITFIELD;
    0x1206, LF_METHODLIST;
    0x1207, LF_DIMCONU;
    0x1208, LF_DIMCONLU;
    0x1209, LF_DIMVARU;
    0x120a, LF_DIMVARLU;
    0x1400, LF_BCLASS;      // (in field list) real (non-virtual) base class
    0x1401, LF_VBCLASS;     // (in field list) direct virtual base class
    0x1402, LF_IVBCLASS;    // (in field list) indirect virtual base class
    0x1404, LF_INDEX;       // (in field list) index to another type record
    0x1409, LF_VFUNCTAB;    // (in field list) virtual function table pointer
    0x140c, LF_VFUNCOFF;    // (in field list) virtual function offset
    0x1502, LF_ENUMERATE;   // (in field list) an enumerator value
    0x1503, LF_ARRAY;
    0x1504, LF_CLASS;
    0x1505, LF_STRUCTURE;
    0x1506, LF_UNION;
    0x1507, LF_ENUM;
    0x1508, LF_DIMARRAY;
    0x1509, LF_PRECOMP;
    0x150a, LF_ALIAS;
    0x150b, LF_DEFARG;
    0x150c, LF_FRIENDFCN;   // (in field list) friend function
    0x150d, LF_MEMBER;      // (in field list) data member
    0x150e, LF_STMEMBER;    // (in field list) static data member
    0x150f, LF_METHOD;      // (in field list) method group (overloaded methods), not single method
    0x1510, LF_NESTEDTYPE;  // (in field list) nested type definition
    0x1511, LF_ONEMETHOD;   // (in field list) a single method
    0x1512, LF_NESTEDTYPEEX;// (in field list) nested type extended definition
    0x1514, LF_MANAGED;
    0x1515, LF_TYPESERVER2;
    0x1519, LF_INTERFACE;
    0x151d, LF_VFTABLE;

    // --- end of types ---

    // 0x1601..=0x1607 are only present in IPI stream, not TPI stream.

    0x1601, LF_FUNC_ID;         // global func ID
    0x1602, LF_MFUNC_ID;        // member func ID
    0x1603, LF_BUILDINFO;       // build info: tool, version, command line, src/pdb file
    0x1604, LF_SUBSTR_LIST;     // similar to LF_ARGLIST, for list of sub strings
    0x1605, LF_STRING_ID;       // string ID

    // source and line on where an UDT is defined
    // only generated by compiler
    0x1606, LF_UDT_SRC_LINE;

    // module, source and line on where an UDT is defined
    // only generated by linker
    0x1607, LF_UDT_MOD_SRC_LINE;

    // The following four kinds were added to the wrong place in this enumeration.
    // They should have been added befor LF_TYPE_LAST.
    // But now it has been too late to change this :-(

    0x1608, LF_CLASS2;       // LF_CLASS with 32bit property field
    0x1609, LF_STRUCTURE2;   // LF_STRUCTURE with 32bit property field
    0x160a, LF_UNION2;       // LF_UNION with 32bit property field
    0x160b, LF_INTERFACE2;   // LF_INTERFACE with 32bit property field

    // These values are used for encoding numeric constants.
//    0x8000, LF_NUMERIC;
    0x8000, LF_CHAR;            // i8
    0x8001, LF_SHORT;           // i16
    0x8002, LF_USHORT;          // u16
    0x8003, LF_LONG;            // i32
    0x8004, LF_ULONG;           // u32
    0x8005, LF_REAL32;          // f32
    0x8006, LF_REAL64;          // f64
    0x8007, LF_REAL80;
    0x8008, LF_REAL128;
    0x8009, LF_QUADWORD;        // i64
    0x800a, LF_UQUADWORD;       // u64
    0x800b, LF_REAL48;
    0x800c, LF_COMPLEX32;
    0x800d, LF_COMPLEX64;
    0x800e, LF_COMPLEX80;
    0x800f, LF_COMPLEX128;
    0x8010, LF_VARSTRING;       // string prefixed with u16 length
    0x8017, LF_OCTWORD;         // i128
    0x8018, LF_UOCTWORD;        // u128
    0x8019, LF_DECIMAL;
    0x801a, LF_DATE;            // 8 bytes
    0x801b, LF_UTF8STRING;      // NUL-terminated UTF-8 string
    0x801c, LF_REAL16;
}

impl std::fmt::Debug for Leaf {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Ok(index) = LEAF_NAMES.binary_search_by_key(self, |ii| ii.0) {
            fmt.write_str(LEAF_NAMES[index].1)
        } else {
            let b0 = (self.0 & 0xff) as u8;
            let b1 = (self.0 >> 8) as u8;
            fn to_c(b: u8) -> char {
                if (32..=126).contains(&b) {
                    char::from(b)
                } else {
                    '_'
                }
            }

            write!(fmt, "Leaf(??{:04x} {}{})", self.0, to_c(b0), to_c(b1))
        }
    }
}

impl Leaf {
    /// True if this `Leaf` codes for an immediate numeric constant.
    pub fn is_immediate_numeric(self) -> bool {
        self.0 < 0x8000
    }

    /// Checks whether this `Leaf` can be used as a type record.
    #[rustfmt::skip]
    pub fn can_start_record(self) -> bool {
        match self {
            | Leaf::LF_MODIFIER
            | Leaf::LF_POINTER
            | Leaf::LF_ARRAY
            | Leaf::LF_CLASS
            | Leaf::LF_STRUCTURE
            | Leaf::LF_UNION
            | Leaf::LF_ENUM
            | Leaf::LF_PROCEDURE
            | Leaf::LF_MFUNCTION
            | Leaf::LF_VTSHAPE
            | Leaf::LF_COBOL0
            | Leaf::LF_COBOL1
            | Leaf::LF_BARRAY
            | Leaf::LF_LABEL
            | Leaf::LF_NULL
            | Leaf::LF_DIMARRAY
            | Leaf::LF_VFTPATH
            | Leaf::LF_PRECOMP
            | Leaf::LF_ENDPRECOMP
            | Leaf::LF_OEM
            | Leaf::LF_OEM2
            | Leaf::LF_ALIAS
            | Leaf::LF_MANAGED
            | Leaf::LF_TYPESERVER2 => true,
            _ => false,
        }
    }

    /// Checks whether this `Leaf` can be used within a field list record.
    #[rustfmt::skip]
    pub fn is_nested_leaf(self) -> bool {
        match self {
            | Leaf::LF_SKIP
            | Leaf::LF_ARGLIST
            | Leaf::LF_DEFARG
            | Leaf::LF_FIELDLIST
            | Leaf::LF_DERIVED
            | Leaf::LF_BITFIELD
            | Leaf::LF_METHODLIST
            | Leaf::LF_DIMCONU
            | Leaf::LF_DIMCONLU
            | Leaf::LF_DIMVARU
            | Leaf::LF_DIMVARLU
            | Leaf::LF_REFSYM => true,
            _ => false,
        }
    }

    /// Indicates whether a given type record can contain references to other type records.
    // TODO: obviously, this is kind of dumb
    pub fn can_reference_types(self) -> bool {
        match self {
            Leaf::LF_MODIFIER
            | Leaf::LF_POINTER
            | Leaf::LF_ARRAY
            | Leaf::LF_CLASS
            | Leaf::LF_UNION
            | Leaf::LF_ENUM
            | Leaf::LF_PROCEDURE => true,
            _ => true,
        }
    }
}
