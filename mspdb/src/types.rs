//! Code for decoding type record streams (the TPI and IPI streams).

mod iter;
#[doc(inline)]
pub use iter::*;

mod kind;
#[doc(inline)]
pub use kind::*;

pub mod fields;
pub mod number;
pub mod primitive;
pub mod visitor;

mod records;
#[doc(inline)]
pub use records::*;

pub use fields::FieldList;

use self::primitive::dump_primitive_type_index;
use crate::parser::{Number, Parse, Parser, ParserError};
use bitfield::bitfield;
use bstr::BStr;
use std::fmt::Debug;
use zerocopy::{AsBytes, FromBytes, FromZeroes, Unaligned, LE, U16, U32};

/// A type index refers to another type record, or to a primitive type.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TypeIndex(pub u32);

impl TypeIndex {
    /// The minimum value for a `type_index_begin` value.
    ///
    /// This value comes from the fact that the first 0x1000 values are reserved for primitive
    /// types.  See `primitive_types.md` in the specification.
    pub const MIN_BEGIN: TypeIndex = TypeIndex(0x1000);
}

impl std::fmt::Debug for TypeIndex {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0 < TypeIndex::MIN_BEGIN.0 {
            dump_primitive_type_index(fmt, *self)
        } else {
            write!(fmt, "T#0x{:x}", self.0)
        }
    }
}

/// The serialized form of [`TypeIndex`]. This can be embedded directly in data structures
/// stored on disk.
#[derive(Copy, Clone, Eq, PartialEq, Hash, FromBytes, FromZeroes, AsBytes, Unaligned)]
#[repr(transparent)]
pub struct TypeIndexLe(pub U32<LE>);

impl From<TypeIndex> for TypeIndexLe {
    #[inline(always)]
    fn from(value: TypeIndex) -> TypeIndexLe {
        TypeIndexLe(U32::new(value.0))
    }
}

impl Debug for TypeIndexLe {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ti = self.get();
        Debug::fmt(&ti, fmt)
    }
}

impl TypeIndexLe {
    /// Converts this value to host byte-order.
    #[inline(always)]
    pub fn get(self) -> TypeIndex {
        TypeIndex(self.0.get())
    }
}

/// Parsed details of a type record.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum TypeData<'a> {
    Array(Array<'a>),
    Struct(Struct<'a>),
    Union(Union<'a>),
    Enum(Enum<'a>),
    Proc(&'a Proc),
    MemberFunc(&'a MemberFunc),
    VTableShape(VTableShapeData<'a>),
    Pointer(Pointer<'a>),
    Modifier(TypeModifier),
    FieldList(FieldList<'a>),
    MethodList(MethodListData<'a>),
    ArgList(ArgList<'a>),
    Alias(Alias<'a>),
    UdtSrcLine(&'a UdtSrcLine),
    UdtModSrcLine(&'a UdtModSrcLine),
    FuncId(FuncId<'a>),
    MFuncId(MFuncId<'a>),
    StringId(StringId<'a>),
    SubStrList(SubStrList<'a>),
    BuildInfo(BuildInfo<'a>),
    VFTable(&'a VFTable),
    Unknown,
}

impl<'a> TypeData<'a> {
    /// Parses the payload of a type record.
    pub fn parse_bytes(kind: Leaf, bytes: &'a [u8]) -> Result<Self, ParserError> {
        let mut p = Parser::new(bytes);
        Self::parse(kind, &mut p)
    }

    /// Parses the payload of a type record, using a [`Parser`].
    pub fn parse(kind: Leaf, p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(match kind {
            Leaf::LF_ARRAY => Self::Array(p.parse()?),
            Leaf::LF_CLASS | Leaf::LF_STRUCTURE | Leaf::LF_INTERFACE => Self::Struct(p.parse()?),
            Leaf::LF_UNION => Self::Union(p.parse()?),
            Leaf::LF_ENUM => Self::Enum(p.parse()?),
            Leaf::LF_PROCEDURE => Self::Proc(p.get()?),
            Leaf::LF_MEMBER => Self::MemberFunc(p.get()?),

            Leaf::LF_VTSHAPE => {
                let fixed: &VTableShapeFixed = p.get()?;
                Self::VTableShape(VTableShapeData {
                    count: fixed.count.get(),
                    descriptors: p.take_rest(),
                })
            }

            Leaf::LF_VFTABLE => Self::VFTable(p.get()?),

            Leaf::LF_POINTER => {
                let fixed = p.get()?;
                let variant = p.take_rest();
                Self::Pointer(Pointer { fixed, variant })
            }

            Leaf::LF_MFUNCTION => Self::MemberFunc(p.get()?),
            Leaf::LF_MODIFIER => Self::Modifier(p.copy()?),

            Leaf::LF_FIELDLIST => Self::FieldList(FieldList {
                bytes: p.take_rest(),
            }),

            Leaf::LF_METHODLIST => Self::MethodList(MethodListData {
                bytes: p.take_rest(),
            }),

            Leaf::LF_ARGLIST => Self::ArgList(p.parse()?),
            Leaf::LF_ALIAS => Self::Alias(Alias::from_parser(p)?),
            Leaf::LF_UDT_SRC_LINE => Self::UdtSrcLine(p.get()?),
            Leaf::LF_UDT_MOD_SRC_LINE => Self::UdtModSrcLine(p.get()?),
            Leaf::LF_FUNC_ID => Self::FuncId(p.parse()?),
            Leaf::LF_MFUNC_ID => Self::MFuncId(p.parse()?),
            Leaf::LF_STRING_ID => Self::StringId(p.parse()?),
            Leaf::LF_SUBSTR_LIST => Self::SubStrList(p.parse()?),
            Leaf::LF_BUILDINFO => Self::BuildInfo(p.parse()?),

            _ => Self::Unknown,
        })
    }

    /// If this record has a primary "name" field, return it. Else, return `None`.
    pub fn name(&self) -> Option<&'a BStr> {
        match self {
            // From TPI
            Self::Struct(t) => Some(t.name),
            Self::Union(t) => Some(t.name),
            Self::Enum(t) => Some(t.name),
            Self::Alias(t) => Some(t.name),

            // From IPI
            Self::FuncId(t) => Some(t.name),
            Self::StringId(t) => Some(t.name),

            _ => None,
        }
    }

    /// Returns the name of this type definition, if it is a UDT (user-defined type) definition.
    pub fn udt_name(&self) -> Option<&'a BStr> {
        match self {
            Self::Struct(t) => Some(t.name),
            Self::Union(t) => Some(t.name),
            Self::Enum(t) => Some(t.name),
            Self::Alias(t) => Some(t.name),
            _ => None,
        }
    }
}
