//! Primitive types

use super::TypeIndex;

#[allow(missing_docs)]
pub const PRIMITIVE_TYPE_SPECIAL: u32 = 0;
#[allow(missing_docs)]
pub const PRIMITIVE_TYPE_SIGNED_INT: u32 = 1;
#[allow(missing_docs)]
pub const PRIMITIVE_TYPE_UNSIGNED_INT: u32 = 2;
#[allow(missing_docs)]
pub const PRIMITIVE_TYPE_BOOL: u32 = 3;
#[allow(missing_docs)]
pub const PRIMITIVE_TYPE_REAL: u32 = 4;
#[allow(missing_docs)]
pub const PRIMITIVE_TYPE_COMPLEX: u32 = 5;
#[allow(missing_docs)]
pub const PRIMITIVE_TYPE_SPECIAL2: u32 = 6;
#[allow(missing_docs)]
pub const PRIMITIVE_TYPE_REALLY_INT: u32 = 7;

macro_rules! primitives {
    (
        $(
            (
                $value:expr,
                $name:ident,
                $description:expr
            ),
        )*
    ) => {
        /// Contains the names and descriptions of all primitive types
        pub static PRIMITIVES: &[(u32, &str, &str)] = &[
            $(
                ($value, stringify!($name), $description),
            )*
        ];

        impl TypeIndex {
            $(
                #[doc = concat!("Primitive type: `", $description, "`")]
                pub const $name: TypeIndex = TypeIndex($value);
            )*
        }
    }
}

primitives! {
    // number, spec name, C/C++ name
    (0x0000, T_NOTYPE, "none"),
    (0x0003, T_VOID, "void"),
    (0x0008, T_HRESULT, "HRESULT"),
    (0x0007, T_NOTTRANS, "<type-not-translated>"),
    (0x0010, T_CHAR, "char"),
    (0x0011, T_SHORT, "short"),
    (0x0012, T_LONG, "long"),
    (0x0013, T_QUAD, "long long"),
    (0x0014, T_OCT, "__int128"),
    (0x0020, T_UCHAR, "unsigned char"),
    (0x0021, T_USHORT, "unsigned short"),
    (0x0022, T_ULONG, "unsigned long"),
    (0x0023, T_UQUAD, "unsigned long long"),
    (0x0024, T_UOCT, "unsigned __int128"),
    (0x0030, T_BOOL8, "bool"),
    (0x0031, T_BOOL16, "bool16"),
    (0x0032, T_BOOL32, "bool32"),
    (0x0033, T_BOOL64, "bool64"),
    (0x0040, T_REAL32, "float"),
    (0x0041, T_REAL64, "double"),
    (0x0068, T_INT1, "__int8"),
    (0x0066, T_UINT1, "unsigned __int8"),
    (0x0070, T_RCHAR, "char"), // really a character. This is "char", which is distinct from "signed char" and "unsigned char"
    (0x0071, T_WCHAR, "wchar_t"),
    (0x0072, T_INT2, "__int16"),
    (0x0073, T_UINT2, "unsigned __int16"),
    (0x0074, T_INT4, "__int32"), // really 32-bit
    (0x0075, T_UINT4, "unsigned __int32"),
    (0x0076, T_INT8, "__int64"),
    (0x0077, T_UINT8, "unsigned __int64"),
    (0x007a, T_CHAR16, "char16"),
    (0x007b, T_CHAR32, "char32"),
    // 32-bit pointer types
    (0x0403, T_32PVOID, "void *"),
    (0x0408, T_32PHRESULT, "HRESULT *"),
    (0x0410, T_32PCHAR, "char *"),
    (0x0411, T_32PSHORT, "short *"),
    (0x0412, T_32PLONG, "long *"),
    (0x0413, T_32PQUAD, "long long *"),
    (0x0414, T_32POCT, "__int128 *"),
    (0x0420, T_32PUCHAR, "unsigned char *"),
    (0x0421, T_32PUSHORT, "unsigned short *"),
    (0x0422, T_32PULONG, "unsigned __int32 *"),
    (0x0423, T_32PUQUAD, "long long *"),
    (0x0424, T_32PUOCT, "unsigned __int128 *"),
    (0x0430, T_32PBOOL08, "bool *"),
    (0x0431, T_32PBOOL16, "bool16 *"),
    (0x0432, T_32PBOOL32, "bool32 *"),
    (0x0433, T_32PBOOL64, "bool64 *"),
    (0x0440, T_32PREAL32, "float *"),
    (0x0441, T_32PREAL64, "double *"),
    (0x0466, T_32PUINT1, "unsigned __int8 *"),
    (0x0468, T_32PINT1, "__int8 *"),
    (0x0470, T_32PRCHAR, "char *"), // really a character
    (0x0471, T_32PWCHAR, "wchar_t *"),
    (0x0472, T_32PINT2, "__int16 *"),
    (0x0473, T_32PUINT2, "unsigned __int16 *"),
    (0x0474, T_32PINT4, "__int32 *"),
    (0x0475, T_32PUINT4, "unsigned __int32 *"),
    (0x0476, T_32PINT8, "__int64 *"),
    (0x0477, T_32PUINT8, "unsigned __int64 *"),
    (0x047a, T_32PCHAR16, "char16 *"),
    (0x047b, T_32PCHAR32, "char32 *"),
    // 64-bit pointer types
    (0x0603, T_64PVOID, "void *"),
    (0x0608, T_64PHRESULT, "HRESULT *"),
    (0x0610, T_64PCHAR, "char *"),
    (0x0611, T_64PSHORT, "short *"),
    (0x0612, T_64PLONG, "long *"),
    (0x0613, T_64PQUAD, "long long *"),
    (0x0614, T_64POCT, "__int128 *"),
    (0x0620, T_64PPUCHAR, "unsigned char *"),
    (0x0621, T_64PUSHORT, "unsigned short *"),
    (0x0622, T_64PULONG, "unsigned __int32 *"),
    (0x0623, T_64PUQUAD, "long long *"),
    (0x0624, T_64PUOCT, "unsigned __int128 *"),
    (0x0630, T_64PBOOL08, "bool *"),
    (0x0631, T_64PBOOL16, "bool16 *"),
    (0x0632, T_64PBOOL32, "bool32 *"),
    (0x0633, T_64PBOOL64, "bool64 *"),
    (0x0640, T_64PREAL32, "float *"),
    (0x0641, T_64PREAL64, "double *"),
    (0x0666, T_64PUINT1, "unsigned __int8 *"),
    (0x0668, T_64PINT1, "__int8 *"),
    (0x0670, T_64PRCHAR, "char *"), // really a character
    (0x0671, T_64PWCHAR, "wchar_t *"),
    (0x0672, T_64PINT2, "__int16 *"),
    (0x0673, T_64PUINT2, "unsigned __int16 *"),
    (0x0674, T_64PINT4, "__int32 *"),
    (0x0675, T_64PUINT4, "unsigned __int32 *"),
    (0x0676, T_64PINT8, "__int64 *"),
    (0x0677, T_64PUINT8, "unsigned __int64 *"),
    (0x067a, T_64PCHAR16, "char16 *"),
    (0x067b, T_64PCHAR32, "char32 *"),
}

/// Dumps a `TypeIndex`. For use only with primitive types.
pub fn dump_primitive_type_index(
    out: &mut dyn std::fmt::Write,
    type_index: TypeIndex,
) -> std::fmt::Result {
    let mode = (type_index.0 >> 8) & 7;
    let prim_ty = (type_index.0 >> 4) & 0xf;
    let size = type_index.0 & 7;

    if let Ok(i) = PRIMITIVES.binary_search_by_key(&type_index.0, |entry| entry.0) {
        let s = PRIMITIVES[i].1;
        write!(out, "{}", s)?;
    } else {
        write!(out, "??PRIM(0x{:04x}) {{ ty: ", type_index.0)?;

        'a: {
            let ty_str = match prim_ty {
                PRIMITIVE_TYPE_SPECIAL => "special",
                PRIMITIVE_TYPE_SIGNED_INT => "signed_integer",
                PRIMITIVE_TYPE_UNSIGNED_INT => "unsigned_integer",
                PRIMITIVE_TYPE_BOOL => "bool",
                PRIMITIVE_TYPE_REAL => "real",
                PRIMITIVE_TYPE_COMPLEX => "complex",
                PRIMITIVE_TYPE_SPECIAL2 => "special2",
                PRIMITIVE_TYPE_REALLY_INT => "really_integer",
                _ => {
                    write!(out, "??{prim_ty}")?;
                    break 'a;
                }
            };
            write!(out, "{}", ty_str)?;
        }

        write!(out, ", mode: {mode}, size: {size} }}")?;
    }

    Ok(())
}

#[test]
fn test_dump() {
    let mut s = String::new();
    dump_primitive_type_index(&mut s, TypeIndex::T_REAL32).unwrap();
    assert_eq!(s, "T_REAL32");

    s.clear();
    dump_primitive_type_index(&mut s, TypeIndex(0x067c)).unwrap();
}
