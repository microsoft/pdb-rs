//! Symbol kind enumeration

#[cfg(doc)]
use super::BlockHeader;

/// Identifies symbol records.
///
/// Symbol records are stored in the Global Symbol Stream and in each per-module symbol stream.
///
/// Many symbols can only appear in the Global Symbol Stream or in a per-module symbol stream.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SymKind(pub u16);

macro_rules! sym_kinds {
    (
        $( $code:expr, $name:ident; )*
    ) => {
        #[allow(missing_docs)]
        impl SymKind {
            $(
                pub const $name: SymKind = SymKind($code);
            )*
        }

        static SYM_NAMES: &[(SymKind, &str)] = &[
            $(
                (SymKind($code), stringify!($name)),
            )*
        ];
    }
}

sym_kinds! {
    0x0001, S_COMPILE;
    0x0006, S_END;
    0x0007, S_SKIP;
    0x0008, S_CVRESERVE;
    0x0009, S_OBJNAME_ST;
    0x000d, S_RETURN;
    // 0x0100..0x0400 is for 16-bit types
    0x0400, S_PROCREF_ST;
    0x0401, S_DATAREF_ST;
    0x0402, S_ALIGN;
    0x0403, S_LPROCREF_ST;
    0x0404, S_OEM;

    0x1000, S_TI16_MAX;
    0x1001, S_REGISTER_ST;
    0x1002, S_CONSTANT_ST;
    0x1003, S_UDT_ST;
    0x1004, S_COBOLUDT_ST;
    0x1005, S_MANYREG_ST;
    0x1006, S_BPREL32_ST;
    0x1007, S_LDATA32_ST;
    0x1008, S_GDATA32_ST;
    0x1009, S_PUB32_ST;
    0x100a, S_LPROC32_ST;
    0x100b, S_GPROC32_ST;
    0x100c, S_VFTABLE32;
    0x100d, S_REGREL32_ST;
    0x100e, S_LTHREAD32_ST;
    0x100f, S_GTHREAD32_ST;
    0x1012, S_FRAMEPROC;
    0x1019, S_ANNOTATION;

    0x1101, S_OBJNAME;
    0x1102, S_THUNK32;
    0x1103, S_BLOCK32;
    0x1104, S_WITH32;
    0x1105, S_LABEL32;
    0x1106, S_REGISTER;
    0x1107, S_CONSTANT;
    0x1108, S_UDT;
    0x1109, S_COBOLUDT;
    0x110a, S_MANYREG;
    0x110b, S_BPREL32;
    0x110c, S_LDATA32;
    0x110d, S_GDATA32;
    0x110e, S_PUB32;
    0x110f, S_LPROC32;

    0x1110, S_GPROC32;
    0x1111, S_REGREL32;
    0x1112, S_LTHREAD32;
    0x1113, S_GTHREAD32;
    0x1116, S_COMPILE2;
    0x1117, S_MANYREG2;
    0x1118, S_LPROCIA64;
    0x1119, S_GPROCIA64;
    0x111a, S_LOCALSLOT;
    0x111b, S_PARAMSLOT;
    0x111c, S_LMANDATA;
    0x111d, S_GMANDATA;
    0x111e, S_MANFRAMEREL;
    0x111f, S_MANREGISTER;

    0x1120, S_MANSLOT;
    0x1121, S_MANMANYREG;
    0x1122, S_MANREGREL;
    0x1123, S_MANMANYREG2;
    0x1124, S_UNAMESPACE;
    0x1125, S_PROCREF;
    0x1126, S_DATAREF;
    0x1127, S_LPROCREF;
    0x1128, S_ANNOTATIONREF;
    0x1129, S_TOKENREF;
    0x112a, S_GMANPROC;
    0x112b, S_LMANPROC;
    0x112c, S_TRAMPOLINE;
    0x112d, S_MANCONSTANT;
    0x112e, S_ATTR_FRAMEREL;
    0x112f, S_ATTR_REGISTER;

    0x1130, S_ATTR_REGREL;
    0x1131, S_ATTR_MANYREG;
    0x1132, S_SEPCODE;
    0x1133, S_LOCAL_2005;
    0x1134, S_DEFRANGE_2005;
    0x1135, S_DEFRANGE2_2005;
    0x1136, S_SECTION;
    0x1137, S_COFFGROUP;
    0x1138, S_EXPORT;
    0x1139, S_CALLSITEINFO;
    0x113a, S_FRAMECOOKIE;
    0x113b, S_DISCARDED;
    0x113c, S_COMPILE3;
    0x113d, S_ENVBLOCK;
    0x113e, S_LOCAL;
    0x113f, S_DEFRANGE;

    0x1140, S_DEFRANGE_SUBFIELD;
    0x1141, S_DEFRANGE_REGISTER;
    0x1142, S_DEFRANGE_FRAMEPOINTER_REL;
    0x1143, S_DEFRANGE_SUBFIELD_REGISTER;
    0x1144, S_DEFRANGE_FRAMEPOINTER_REL_FULL_SCOPE;
    0x1145, S_DEFRANGE_REGISTER_REL;
    0x1146, S_LPROC32_ID;
    0x1147, S_GPROC32_ID;
    0x1148, S_LPROCMIPS_ID;
    0x1149, S_GPROCMIPS_ID;
    0x114a, S_LPROCIA64_ID;
    0x114b, S_GPROCIA64_ID;
    0x114c, S_BUILDINFO;
    0x114d, S_INLINESITE;
    0x114e, S_INLINESITE_END;
    0x114f, S_PROC_ID_END;

    0x1150, S_DEFRANGE_HLSL;
    0x1151, S_GDATA_HLSL;
    0x1152, S_LDATA_HLSL;
    0x1153, S_FILESTATIC;
    0x1154, S_LOCAL_DPC_GROUPSHARED;
    0x1155, S_LPROC32_DPC;
    0x1156, S_LPROC32_DPC_ID;
    0x1157, S_DEFRANGE_DPC_PTR_TAG;
    0x1158, S_DPC_SYM_TAG_MAP;
    0x1159, S_ARMSWITCHTABLE;
    0x115a, S_CALLEES;
    0x115b, S_CALLERS;
    0x115c, S_POGODATA;
    0x115d, S_INLINESITE2;
    0x115e, S_HEAPALLOCSITE;
    0x115f, S_MOD_TYPEREF;

    0x1160, S_REF_MINIPDB;
    0x1161, S_PDBMAP;
    0x1162, S_GDATA_HLSL32;
    0x1163, S_LDATA_HLSL32;
    0x1164, S_GDATA_HLSL32_EX;
    0x1165, S_LDATA_HLSL32_EX;
    0x1167, S_FASTLINK;
    0x1168, S_INLINEES;
}

impl std::fmt::Debug for SymKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Ok(index) = SYM_NAMES.binary_search_by_key(self, |ii| ii.0) {
            <str as std::fmt::Display>::fmt(SYM_NAMES[index].1, f)
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

            write!(f, "S_(??{:04x} {}{})", self.0, to_c(b1), to_c(b0))
        }
    }
}

#[test]
fn test_sym_kind_debug() {
    assert_eq!(format!("{:?}", SymKind::S_GPROC32), "S_GPROC32");
    assert_eq!(format!("{:?}", SymKind(0x31aa)), "S_(??31aa 1_)");
}

impl SymKind {
    /// True if this `SymKind` starts a scope. All symbols that start a block begin with
    /// [`BlockHeader`].
    pub fn starts_scope(self) -> bool {
        matches!(
            self,
            SymKind::S_GPROC32
                | SymKind::S_LPROC32
                | SymKind::S_LPROC32_DPC
                | SymKind::S_LPROC32_DPC_ID
                | SymKind::S_GPROC32_ID
                | SymKind::S_BLOCK32
                | SymKind::S_THUNK32
                | SymKind::S_INLINESITE
                | SymKind::S_INLINESITE2
                | SymKind::S_SEPCODE
                | SymKind::S_GMANPROC
                | SymKind::S_LMANPROC
        )
    }

    /// Indicates whether this `SymKind` ends a scope.
    ///
    /// There are no `SymKind` values that both start and end a scope.
    ///
    /// In all well-formed symbol streams, every symbol that starts a scope has a matching symbol
    /// that ends that scope.
    pub fn ends_scope(self) -> bool {
        matches!(
            self,
            SymKind::S_END | SymKind::S_PROC_ID_END | SymKind::S_INLINESITE_END
        )
    }

    /// Returns `true` if this symbol can be the _target_ of a "reference to symbol" in the
    /// Global Symbol Stream.
    pub fn is_refsym_target(self) -> bool {
        matches!(
            self,
            SymKind::S_GPROC32
                | SymKind::S_LPROC32
                | SymKind::S_GMANPROC
                | SymKind::S_LMANPROC
                | SymKind::S_GDATA32
                | SymKind::S_LDATA32
                | SymKind::S_ANNOTATION
        )
    }

    /// Returns `true` if this symbol can be the _source_ of a "reference to symbol"
    /// in the Global Symbol Stream.
    pub fn is_refsym_source(self) -> bool {
        matches!(
            self,
            SymKind::S_LPROCREF
                | SymKind::S_PROCREF
                | SymKind::S_ANNOTATIONREF
                | SymKind::S_TOKENREF
                | SymKind::S_DATAREF
        )
    }
}
