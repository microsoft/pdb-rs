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
    (0x0001, T_ABS, "absolute symbol"),
    (0x0002, T_SEGMENT, "segment type"),
    (0x0003, T_VOID, "void"),
    (0x0004, T_CURRENCY, "BASIC 8 byte currency value"),
    (0x0005, T_NBASICSTR, "Near BASIC string"),
    (0x0006, T_FBASICSTR, "Far BASIC string"),
    (0x0007, T_NOTTRANS, "<type-not-translated>"),
    (0x0008, T_HRESULT, "HRESULT"),
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
    (0x0042, T_REAL80, "80 bit real"),
    (0x0043, T_REAL128, "128 bit real"),
    (0x0044, T_REAL48, "48 bit real"),
    (0x0045, T_REAL32PP, "32 bit PP real"),
    (0x0046, T_REAL16, "16 bit real"),
    (0x0050, T_CPLX32, "32 bit complex"),
    (0x0051, T_CPLX64, "64 bit complex"),
    (0x0052, T_CPLX80, "80 bit complex"),
    (0x0053, T_CPLX128, "128 bit complex"),
    (0x0060, T_BIT, "bit"),
    (0x0061, T_PASCHAR, "Pascal CHAR"),
    (0x0062, T_BOOL32FF, "32-bit BOOL where true is 0xffffffff"),
    (0x0068, T_INT1, "__int8"),
    (0x0069, T_UINT1, "unsigned __int8"),
    (0x0070, T_RCHAR, "char"), // really a character. This is "char", which is distinct from "signed char" and "unsigned char"
    (0x0071, T_WCHAR, "wchar_t"),
    (0x0072, T_INT2, "__int16"),
    (0x0073, T_UINT2, "unsigned __int16"),
    (0x0074, T_INT4, "__int32"), // really 32-bit
    (0x0075, T_UINT4, "unsigned __int32"),
    (0x0076, T_INT8, "__int64"),
    (0x0077, T_UINT8, "unsigned __int64"),
    (0x0078, T_INT16, "128 bit signed int"),
    (0x0079, T_UINT16, "128 bit unsigned int"),
    (0x007a, T_CHAR16, "char16"),
    (0x007b, T_CHAR32, "char32"),
    // 32-bit pointer types
    (0x0103, T_PVOID, "near pointer to void"),
    (0x0110, T_PCHAR, "16 bit pointer to 8 bit signed"),
    (0x0111, T_PSHORT, "16 bit pointer to 16 bit signed"),
    (0x0112, T_PLONG, "16 bit pointer to 32 bit signed"),
    (0x0113, T_PQUAD, "16 bit pointer to 64 bit signed"),
    (0x0114, T_POCT, "16 bit pointer to 128 bit signed"),
    (0x0120, T_PUCHAR, "16 bit pointer to 8 bit unsigned"),
    (0x0121, T_PUSHORT, "16 bit pointer to 16 bit unsigned"),
    (0x0122, T_PULONG, "16 bit pointer to 32 bit unsigned"),
    (0x0123, T_PUQUAD, "16 bit pointer to 64 bit unsigned"),
    (0x0124, T_PUOCT, "16 bit pointer to 128 bit unsigned"),
    (0x0130, T_PBOOL08, "16 bit pointer to  8 bit boolean"),
    (0x0131, T_PBOOL16, "16 bit pointer to 16 bit boolean"),
    (0x0132, T_PBOOL32, "16 bit pointer to 32 bit boolean"),
    (0x0133, T_PBOOL64, "16 bit pointer to 64 bit boolean"),
    (0x0140, T_PREAL32, "16 bit pointer to 32 bit real"),
    (0x0141, T_PREAL64, "16 bit pointer to 64 bit real"),
    (0x0142, T_PREAL80, "16 bit pointer to 80 bit real"),
    (0x0143, T_PREAL128, "16 bit pointer to 128 bit real"),
    (0x0144, T_PREAL48, "16 bit pointer to 48 bit real"),
    (0x0145, T_PREAL32PP, "16 bit pointer to 32 bit PP real"),
    (0x0146, T_PREAL16, "16 bit pointer to 16 bit real"),
    (0x0150, T_PCPLX32, "16 bit pointer to 32 bit complex"),
    (0x0151, T_PCPLX64, "16 bit pointer to 64 bit complex"),
    (0x0152, T_PCPLX80, "16 bit pointer to 80 bit complex"),
    (0x0153, T_PCPLX128, "16 bit pointer to 128 bit complex"),
    (0x0168, T_PINT1, "16 bit pointer to 8 bit signed int"),
    (0x0169, T_PUINT1, "16 bit pointer to 8 bit unsigned int"),
    (0x0170, T_PRCHAR, "16 bit pointer to a real char"),
    (0x0171, T_PWCHAR, "16 bit pointer to a wide char"),
    (0x0172, T_PINT2, "16 bit pointer to 16 bit signed int"),
    (0x0173, T_PUINT2, "16 bit pointer to 16 bit unsigned int"),
    (0x0174, T_PINT4, "16 bit pointer to 32 bit signed int"),
    (0x0175, T_PUINT4, "16 bit pointer to 32 bit unsigned int"),
    (0x0176, T_PINT8, "16 bit pointer to 64 bit signed int"),
    (0x0177, T_PUINT8, "16 bit pointer to 64 bit unsigned int"),
    (0x0178, T_PINT16, "16 bit pointer to 128 bit signed int"),
    (0x0179, T_PUINT16, "16 bit pointer to 128 bit unsigned int"),
    (0x0203, T_PFVOID, "far pointer to void"),
    (0x0210, T_PFCHAR, "16:16 far pointer to 8 bit signed"),
    (0x0211, T_PFSHORT, "16:16 far pointer to 16 bit signed"),
    (0x0212, T_PFLONG, "16:16 far pointer to 32 bit signed"),
    (0x0213, T_PFQUAD, "16:16 far pointer to 64 bit signed"),
    (0x0214, T_PFOCT, "16:16 far pointer to 128 bit signed"),
    (0x0220, T_PFUCHAR, "16:16 far pointer to 8 bit unsigned"),
    (0x0221, T_PFUSHORT, "16:16 far pointer to 16 bit unsigned"),
    (0x0222, T_PFULONG, "16:16 far pointer to 32 bit unsigned"),
    (0x0223, T_PFUQUAD, "16:16 far pointer to 64 bit unsigned"),
    (0x0224, T_PFUOCT, "16:16 far pointer to 128 bit unsigned"),
    (0x0230, T_PFBOOL08, "16:16 far pointer to  8 bit boolean"),
    (0x0231, T_PFBOOL16, "16:16 far pointer to 16 bit boolean"),
    (0x0232, T_PFBOOL32, "16:16 far pointer to 32 bit boolean"),
    (0x0233, T_PFBOOL64, "16:16 far pointer to 64 bit boolean"),
    (0x0240, T_PFREAL32, "16:16 far pointer to 32 bit real"),
    (0x0241, T_PFREAL64, "16:16 far pointer to 64 bit real"),
    (0x0242, T_PFREAL80, "16:16 far pointer to 80 bit real"),
    (0x0243, T_PFREAL128, "16:16 far pointer to 128 bit real"),
    (0x0244, T_PFREAL48, "16:16 far pointer to 48 bit real"),
    (0x0245, T_PFREAL32PP, "16:16 far pointer to 32 bit PP real"),
    (0x0246, T_PFREAL16, "16:16 far pointer to 16 bit real"),
    (0x0250, T_PFCPLX32, "16:16 far pointer to 32 bit complex"),
    (0x0251, T_PFCPLX64, "16:16 far pointer to 64 bit complex"),
    (0x0252, T_PFCPLX80, "16:16 far pointer to 80 bit complex"),
    (0x0253, T_PFCPLX128, "16:16 far pointer to 128 bit complex"),
    (0x0268, T_PFINT1, "16:16 far pointer to 8 bit signed int"),
    (0x0269, T_PFUINT1, "16:16 far pointer to 8 bit unsigned int"),
    (0x0270, T_PFRCHAR, "16:16 far pointer to a real char"),
    (0x0271, T_PFWCHAR, "16:16 far pointer to a wide char"),
    (0x0272, T_PFINT2, "16:16 far pointer to 16 bit signed int"),
    (0x0273, T_PFUINT2, "16:16 far pointer to 16 bit unsigned int"),
    (0x0274, T_PFINT4, "16:16 far pointer to 32 bit signed int"),
    (0x0275, T_PFUINT4, "16:16 far pointer to 32 bit unsigned int"),
    (0x0276, T_PFINT8, "16:16 far pointer to 64 bit signed int"),
    (0x0277, T_PFUINT8, "16:16 far pointer to 64 bit unsigned int"),
    (0x0278, T_PFINT16, "16:16 far pointer to 128 bit signed int"),
    (0x0279, T_PFUINT16, "16:16 far pointer to 128 bit unsigned int"),
    (0x0303, T_PHVOID, "huge pointer to void"),
    (0x0310, T_PHCHAR, "16:16 huge pointer to 8 bit signed"),
    (0x0311, T_PHSHORT, "16:16 huge pointer to 16 bit signed"),
    (0x0312, T_PHLONG, "16:16 huge pointer to 32 bit signed"),
    (0x0313, T_PHQUAD, "16:16 huge pointer to 64 bit signed"),
    (0x0314, T_PHOCT, "16:16 huge pointer to 128 bit signed"),
    (0x0320, T_PHUCHAR, "16:16 huge pointer to 8 bit unsigned"),
    (0x0321, T_PHUSHORT, "16:16 huge pointer to 16 bit unsigned"),
    (0x0322, T_PHULONG, "16:16 huge pointer to 32 bit unsigned"),
    (0x0323, T_PHUQUAD, "16:16 huge pointer to 64 bit unsigned"),
    (0x0324, T_PHUOCT, "16:16 huge pointer to 128 bit unsigned"),
    (0x0330, T_PHBOOL08, "16:16 huge pointer to  8 bit boolean"),
    (0x0331, T_PHBOOL16, "16:16 huge pointer to 16 bit boolean"),
    (0x0332, T_PHBOOL32, "16:16 huge pointer to 32 bit boolean"),
    (0x0333, T_PHBOOL64, "16:16 huge pointer to 64 bit boolean"),
    (0x0340, T_PHREAL32, "16:16 huge pointer to 32 bit real"),
    (0x0341, T_PHREAL64, "16:16 huge pointer to 64 bit real"),
    (0x0342, T_PHREAL80, "16:16 huge pointer to 80 bit real"),
    (0x0343, T_PHREAL128, "16:16 huge pointer to 128 bit real"),
    (0x0344, T_PHREAL48, "16:16 huge pointer to 48 bit real"),
    (0x0345, T_PHREAL32PP, "16:16 huge pointer to 32 bit PP real"),
    (0x0346, T_PHREAL16, "16:16 huge pointer to 16 bit real"),
    (0x0350, T_PHCPLX32, "16:16 huge pointer to 32 bit complex"),
    (0x0351, T_PHCPLX64, "16:16 huge pointer to 64 bit complex"),
    (0x0352, T_PHCPLX80, "16:16 huge pointer to 80 bit complex"),
    (0x0353, T_PHCPLX128, "16:16 huge pointer to 128 bit real"),
    (0x0368, T_PHINT1, "16:16 huge pointer to 8 bit signed int"),
    (0x0369, T_PHUINT1, "16:16 huge pointer to 8 bit unsigned int"),
    (0x0370, T_PHRCHAR, "16:16 huge pointer to a real char"),
    (0x0371, T_PHWCHAR, "16:16 huge pointer to a wide char"),
    (0x0372, T_PHINT2, "16:16 huge pointer to 16 bit signed int"),
    (0x0373, T_PHUINT2, "16:16 huge pointer to 16 bit unsigned int"),
    (0x0374, T_PHINT4, "16:16 huge pointer to 32 bit signed int"),
    (0x0375, T_PHUINT4, "16:16 huge pointer to 32 bit unsigned int"),
    (0x0376, T_PHINT8, "16:16 huge pointer to 64 bit signed int"),
    (0x0377, T_PHUINT8, "16:16 huge pointer to 64 bit unsigned int"),
    (0x0378, T_PHINT16, "16:16 huge pointer to 128 bit signed int"),
    (0x0379, T_PHUINT16, "16:16 huge pointer to 128 bit unsigned int"),
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
    (0x0442, T_32PREAL80, "32 bit pointer to 80 bit real"),
    (0x0443, T_32PREAL128, "32 bit pointer to 128 bit real"),
    (0x0444, T_32PREAL48, "32 bit pointer to 48 bit real"),
    (0x0445, T_32PREAL32PP, "32 bit pointer to 32 bit PP real"),
    (0x0446, T_32PREAL16, "32 bit pointer to 16 bit real"),
    (0x0450, T_32PCPLX32, "32 bit pointer to 32 bit complex"),
    (0x0451, T_32PCPLX64, "32 bit pointer to 64 bit complex"),
    (0x0452, T_32PCPLX80, "32 bit pointer to 80 bit complex"),
    (0x0453, T_32PCPLX128, "32 bit pointer to 128 bit complex"),
    (0x0468, T_32PINT1, "__int8 *"),
    (0x0469, T_32PUINT1, "unsigned __int8 *"),
    (0x0470, T_32PRCHAR, "char *"), // really a character
    (0x0471, T_32PWCHAR, "wchar_t *"),
    (0x0472, T_32PINT2, "__int16 *"),
    (0x0473, T_32PUINT2, "unsigned __int16 *"),
    (0x0474, T_32PINT4, "__int32 *"),
    (0x0475, T_32PUINT4, "unsigned __int32 *"),
    (0x0476, T_32PINT8, "__int64 *"),
    (0x0477, T_32PUINT8, "unsigned __int64 *"),
    (0x0478, T_32PINT16, "32 bit pointer to 128 bit signed int"),
    (0x0479, T_32PUINT16, "32 bit pointer to 128 bit unsigned int"),
    (0x047a, T_32PCHAR16, "char16 *"),
    (0x047b, T_32PCHAR32, "char32 *"),
    (0x0503, T_32PFVOID, "16:32 pointer to void"),
    (0x0510, T_32PFCHAR, "16:32 pointer to 8 bit signed"),
    (0x0511, T_32PFSHORT, "16:32 pointer to 16 bit signed"),
    (0x0512, T_32PFLONG, "16:32 pointer to 32 bit signed"),
    (0x0513, T_32PFQUAD, "16:32 pointer to 64 bit signed"),
    (0x0514, T_32PFOCT, "16:32 pointer to 128 bit signed"),
    (0x0520, T_32PFUCHAR, "16:32 pointer to 8 bit unsigned"),
    (0x0521, T_32PFUSHORT, "16:32 pointer to 16 bit unsigned"),
    (0x0522, T_32PFULONG, "16:32 pointer to 32 bit unsigned"),
    (0x0523, T_32PFUQUAD, "16:32 pointer to 64 bit unsigned"),
    (0x0524, T_32PFUOCT, "16:32 pointer to 128 bit unsigned"),
    (0x0530, T_32PFBOOL08, "16:32 pointer to 8 bit boolean"),
    (0x0531, T_32PFBOOL16, "16:32 pointer to 16 bit boolean"),
    (0x0532, T_32PFBOOL32, "16:32 pointer to 32 bit boolean"),
    (0x0533, T_32PFBOOL64, "16:32 pointer to 64 bit boolean"),
    (0x0540, T_32PFREAL32, "16:32 pointer to 32 bit real"),
    (0x0541, T_32PFREAL64, "16:32 pointer to 64 bit real"),
    (0x0542, T_32PFREAL80, "16:32 pointer to 80 bit real"),
    (0x0543, T_32PFREAL128, "16:32 pointer to 128 bit real"),
    (0x0544, T_32PFREAL48, "16:32 pointer to 48 bit real"),
    (0x0545, T_32PFREAL32PP, "16:32 pointer to 32 bit PP real"),
    (0x0546, T_32PFREAL16, "16:32 pointer to 16 bit real"),
    (0x0550, T_32PFCPLX32, "16:32 pointer to 32 bit complex"),
    (0x0551, T_32PFCPLX64, "16:32 pointer to 64 bit complex"),
    (0x0552, T_32PFCPLX80, "16:32 pointer to 80 bit complex"),
    (0x0553, T_32PFCPLX128, "16:32 pointer to 128 bit complex"),
    (0x0568, T_32PFINT1, "16:32 pointer to 8 bit signed int"),
    (0x0569, T_32PFUINT1, "16:32 pointer to 8 bit unsigned int"),
    (0x0570, T_32PFRCHAR, "16:32 pointer to a real char"),
    (0x0571, T_32PFWCHAR, "16:32 pointer to a wide char"),
    (0x0572, T_32PFINT2, "16:32 pointer to 16 bit signed int"),
    (0x0573, T_32PFUINT2, "16:32 pointer to 16 bit unsigned int"),
    (0x0574, T_32PFINT4, "16:32 pointer to 32 bit signed int"),
    (0x0575, T_32PFUINT4, "16:32 pointer to 32 bit unsigned int"),
    (0x0576, T_32PFINT8, "16:32 pointer to 64 bit signed int"),
    (0x0577, T_32PFUINT8, "16:32 pointer to 64 bit unsigned int"),
    (0x0578, T_32PFINT16, "16:32 pointer to 128 bit signed int"),
    (0x0579, T_32PFUINT16, "16:32 pointer to 128 bit unsigned int"),
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
    (0x0642, T_64PREAL80, "64 bit pointer to 80 bit real"),
    (0x0643, T_64PREAL128, "64 bit pointer to 128 bit real"),
    (0x0644, T_64PREAL48, "64 bit pointer to 48 bit real"),
    (0x0645, T_64PREAL32PP, "64 bit pointer to 32 bit PP real"),
    (0x0646, T_64PREAL16, "64 bit pointer to 16 bit real"),
    (0x0650, T_64PCPLX32, "64 bit pointer to 32 bit complex"),
    (0x0651, T_64PCPLX64, "64 bit pointer to 64 bit complex"),
    (0x0652, T_64PCPLX80, "64 bit pointer to 80 bit complex"),
    (0x0653, T_64PCPLX128, "64 bit pointer to 128 bit complex"),
    (0x0668, T_64PINT1, "__int8 *"),
    (0x0669, T_64PUINT1, "unsigned __int8 *"),
    (0x0670, T_64PRCHAR, "char *"), // really a character
    (0x0671, T_64PWCHAR, "wchar_t *"),
    (0x0672, T_64PINT2, "__int16 *"),
    (0x0673, T_64PUINT2, "unsigned __int16 *"),
    (0x0674, T_64PINT4, "__int32 *"),
    (0x0675, T_64PUINT4, "unsigned __int32 *"),
    (0x0676, T_64PINT8, "__int64 *"),
    (0x0677, T_64PUINT8, "unsigned __int64 *"),
    (0x0678, T_64PINT16, "64 bit pointer to 128 bit signed int"),
    (0x0679, T_64PUINT16, "64 bit pointer to 128 bit unsigned int"),
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
        write!(out, "{s}")?;
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
            write!(out, "{ty_str}")?;
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
