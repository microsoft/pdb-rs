#![allow(missing_docs)]

use super::*;
use crate::names::NameIndexLe;
use bstr::BStr;
use zerocopy::U64;

bitfield::bitfield! {
    /// Bit field structure describing class/struct/union/enum properties
    ///
    /// See `CV_prop_t` in `cvinfo.h`.
    pub struct UdtProperties(u16);
    impl Debug;

    pub packed,        set_packed:        0;      // true if structure is packed
    pub ctor,          set_ctor:          1;      // true if constructors or destructors present
    pub ovlops,        set_ovlops:        2;      // true if overloaded operators present
    pub isnested,      set_isnested:      3;      // true if this is a nested class
    pub cnested,       set_cnested:       4;      // true if this class contains nested types
    pub opassign,      set_opassign:      5;      // true if overloaded assignment (=)
    pub opcast,        set_opcast:        6;      // true if casting methods
    pub fwdref,        set_fwdref:        7;      // true if forward reference (incomplete defn)
    pub scoped,        set_scoped:        8;      // scoped definition
    pub hasuniquename, set_hasuniquename: 9;      // true if there is a decorated name following the regular name
    pub sealed,        set_sealed:        10;     // true if class cannot be used as a base class
    pub hfa,           set_hfa:           11, 12; // CV_HFA
    pub intrinsic,     set_intrinsic:     13;     // true if class is an intrinsic type (e.g. __m128d)
    pub mocom,         set_mocom:         14;     // CV_MOCOM_UDT
}

#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned)]
#[repr(transparent)]
pub struct UdtPropertiesLe(pub U16<LE>);

impl Debug for UdtPropertiesLe {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        Debug::fmt(&self.get(), fmt)
    }
}

impl UdtPropertiesLe {
    #[inline(always)]
    pub fn get(&self) -> UdtProperties {
        UdtProperties(self.0.get())
    }
}

#[derive(Clone, Debug)]
pub struct Enum<'a> {
    pub fixed: &'a EnumFixed,
    pub name: &'a BStr,
    pub unique_name: Option<&'a BStr>,
}

#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[repr(C)]
pub struct EnumFixed {
    pub count: U16<LE>,
    pub property: UdtPropertiesLe,
    pub underlying_type: TypeIndexLe,
    pub fields: TypeIndexLe,
}

impl<'a> Parse<'a> for Enum<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let fixed: &EnumFixed = p.get()?;
        let name = p.strz()?;
        let property = fixed.property.get();
        let unique_name = if property.hasuniquename() {
            Some(p.strz()?)
        } else {
            None
        };
        Ok(Self {
            fixed,
            name,
            unique_name,
        })
    }
}

/// For `LF_ARRAY`
#[derive(Clone, Debug)]
pub struct Array<'a> {
    pub fixed: &'a ArrayFixed,
    pub len: Number<'a>,
    pub name: &'a BStr,
}

#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[repr(C)]
pub struct ArrayFixed {
    pub element_type: TypeIndexLe,
    pub index_type: TypeIndexLe,
}

impl<'a> Parse<'a> for Array<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Array {
            fixed: p.get()?,
            len: p.number()?,
            name: p.strz()?,
        })
    }
}

/// For `LF_CLASS`, `LF_STRUCTURE`, and `LF_INTERFACE`.
#[derive(Clone, Debug)]
pub struct Struct<'a> {
    pub fixed: &'a StructFixed,
    pub length: Number<'a>,
    pub name: &'a BStr,
    pub unique_name: Option<&'a BStr>,
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct StructFixed {
    /// Number of elements in the class or structure. This count includes direct, virtual, and
    /// indirect virtual bases, and methods including overloads, data members, static data members,
    /// friends, etc.
    pub num_elements: U16<LE>,

    /// Bit flags
    pub property: UdtPropertiesLe,

    pub field_list: TypeIndexLe,

    // Docs say this should always be zero.
    pub derivation_list: TypeIndexLe,

    pub vtable_shape: TypeIndexLe,
    // numeric leaf
}

impl<'a> Parse<'a> for Struct<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let fixed: &StructFixed = p.get()?;
        let length = p.number()?;
        let name = p.strz()?;
        let unique_name = if fixed.property.get().hasuniquename() {
            Some(p.strz()?)
        } else {
            None
        };
        Ok(Struct {
            fixed,
            length,
            name,
            unique_name,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Union<'a> {
    pub fixed: &'a UnionFixed,
    pub length: Number<'a>,
    pub name: &'a BStr,
    pub unique_name: Option<&'a BStr>,
}

#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[repr(C)]
pub struct UnionFixed {
    pub count: U16<LE>,
    pub property: UdtPropertiesLe,
    pub fields: TypeIndexLe,
}

impl<'a> Parse<'a> for Union<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let fixed: &UnionFixed = p.get()?;
        let length = p.number()?;
        let name = p.strz()?;
        let unique_name = if fixed.property.get().hasuniquename() {
            Some(p.strz()?)
        } else {
            None
        };
        Ok(Union {
            fixed,
            length,
            name,
            unique_name,
        })
    }
}

/// Type modifier record (`LF_MODIFIER`)
///
/// This record defines a qualified variation of another type. Bits indicate whether the qualifier
/// uses `const`, `volatile`, `unaligned`, or a combination of these flags.
#[derive(IntoBytes, FromBytes, Immutable, KnownLayout, Unaligned, Clone, Debug)]
#[repr(C)]
pub struct TypeModifier {
    pub underlying_type: TypeIndexLe,
    pub attributes: U16<LE>,
}

impl<'a> Parse<'a> for TypeModifier {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        p.copy()
    }
}

impl TypeModifier {
    pub fn attributes(&self) -> TypeModifierBits {
        TypeModifierBits(self.attributes.get())
    }

    pub fn is_const(&self) -> bool {
        self.attributes().is_const()
    }

    pub fn is_volatile(&self) -> bool {
        self.attributes().is_volatile()
    }

    pub fn is_unaligned(&self) -> bool {
        self.attributes().is_unaligned()
    }
}

bitfield! {
    #[repr(transparent)]
    #[derive(Clone)]
    pub struct TypeModifierBits(u16);
    impl Debug;

    pub is_const, set_is_const: 0;
    pub is_volatile, set_is_volatile: 1;
    pub is_unaligned, set_is_unaligned: 2;
    pub reserved, set_reserved: 3, 15;
}

/// `LF_PROCEDURE`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct Proc {
    pub return_value: TypeIndexLe,
    pub call: u8,
    pub reserved: u8,
    pub num_params: U16<LE>,
    pub arg_list: TypeIndexLe,
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct MemberFunc {
    pub return_value: TypeIndexLe,
    pub class: TypeIndexLe,
    pub this: TypeIndexLe,
    pub call: u8,
    pub reserved: u8,
    pub num_params: U16<LE>,
    pub arg_list: TypeIndexLe,
    pub this_adjust: U32<LE>,
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned)]
pub struct VTableShapeFixed {
    pub count: U16<LE>,
}

#[derive(Clone, Debug)]
pub struct VTableShapeData<'a> {
    pub count: u16,
    pub descriptors: &'a [u8],
}

pub struct MethodList<'a> {
    pub rest: &'a [u8],
}

impl<'a> MethodList<'a> {
    pub fn parse(record_data: &'a [u8]) -> Result<Self, ParserError> {
        Ok(Self { rest: record_data })
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<MethodListItem>, ParserError> {
        if self.rest.is_empty() {
            return Ok(None);
        }

        let mut p = Parser::new(self.rest);
        let attr = p.u16()?;
        p.u16()?; // discard padding
        let ty = p.type_index()?;
        let vtab_offset = if introduces_virtual(attr) {
            Some(p.u32()?)
        } else {
            None
        };

        self.rest = p.into_rest();

        Ok(Some(MethodListItem {
            attr,
            ty,
            vtab_offset,
        }))
    }
}

/// Indicates whether a method type introduces a new virtual function slot.
///
/// `attr` is the `attr` field of a `LF_ONEMETHOD`, etc. record.
pub fn introduces_virtual(attr: u16) -> bool {
    // This field is only present if this method introduces a new vtable slot.
    matches!((attr >> 2) & 0xf, 4 | 6)
}

pub struct MethodListItem {
    pub attr: u16,
    pub ty: TypeIndex,
    pub vtab_offset: Option<u32>,
}

#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[repr(C)]
pub struct PointerFixed {
    pub ty: TypeIndexLe,
    pub attr: U32<LE>,
}

impl PointerFixed {
    pub fn attr(&self) -> PointerFlags {
        PointerFlags::from_bits(self.attr.get())
    }
}

#[derive(Clone)]
pub struct Pointer<'a> {
    pub fixed: &'a PointerFixed,
    pub variant: &'a [u8],
}

impl<'a> Parse<'a> for Pointer<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let fixed = p.get()?;
        let variant = p.take_rest();
        Ok(Self { fixed, variant })
    }
}

impl<'a> Debug for Pointer<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let attr = self.fixed.attr();
        write!(fmt, "ty: {:?}", self.fixed.ty.get())?;
        write!(fmt, " attr: 0x{:08x} {:?}", attr.0, attr)?;
        write!(fmt, " mode: {}", attr.mode())?;
        Ok(())
    }
}

bitfield::bitfield! {
    pub struct PointerFlags(u32);
    impl Debug;
    pub pointer_kind, set_pointer_kind: 4, 0;
    pub mode, set_mode: 7, 5;
    pub flat32, set_flat32: 8;
    pub volatile, set_volatile: 9;
    pub r#const, set_const: 10;
    pub unaligned, set_unaligned: 11;
    pub restrict, set_restrict: 12;
    pub size, set_size: 13, 18;
    pub ismocom, set_ismocom: 19;
    pub islref, set_islref: 20;
    pub isrref, set_isrref: 21;
    pub unused, set_unused: 31, 22;
}

impl PointerFlags {
    #[allow(missing_docs)]
    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
}

/// Payload for `LF_METHODLIST`
#[derive(Clone, Debug)]
pub struct MethodListData<'a> {
    /// Contains a repeated sequence of:
    ///
    /// ```text
    /// struct {
    ///   attr: u16,
    ///   pad0: u16,
    ///   ty: TypeIndex,
    ///   vtab_offset: u32,         // optional, present only if attr indicates it starts a vtable slot
    /// }
    /// ```
    pub bytes: &'a [u8],
}

/// `LF_ARGLIST`
#[derive(Clone, Debug)]
pub struct ArgList<'a> {
    /// Arguments of the function signature
    pub args: &'a [TypeIndexLe],
}

impl<'a> Parse<'a> for ArgList<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let arg_count = p.u32()?;
        let args = p.slice(arg_count as usize)?;
        Ok(Self { args })
    }
}

/// `LF_ALIAS` record
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Alias<'a> {
    pub utype: TypeIndex,
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for Alias<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            utype: p.type_index()?,
            name: p.strz()?,
        })
    }
}

/// `LF_UDT_SRC_LINE`
///
/// See `lfUdtSrcLine` in `cvinfo.h`
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Clone, Debug)]
#[repr(C)]
pub struct UdtSrcLine {
    /// UDT's type index
    pub ty: TypeIndexLe,

    /// The source file which contains this UDT definition.
    /// This is a `NameIndex` value in the `/names` stream.
    pub src: NameIndexLe,

    /// Line number
    pub line: U32<LE>,
}

/// `LF_UDT_MOD_SRC_LINE`
///
/// See `lfUdtModSrcLine` in `cvinfo.h`
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Clone, Debug)]
#[repr(C)]
pub struct UdtModSrcLine {
    /// UDT's type index
    pub ty: TypeIndexLe,

    /// The source file which contains this UDT definition.
    /// This is a `NameIndex` value in the `/names` stream.
    pub src: NameIndexLe,

    /// Line number, 1-based
    pub line: U32<LE>,

    /// Module that contributes this UDT definition
    pub imod: U16<LE>,
}

/// CV_ItemId
pub type ItemIdLe = U32<LE>;

/// Identifies a record within the IPI Stream.
pub type ItemId = u32;

/// `LF_FUNC_ID`
#[derive(Clone, Debug)]
pub struct FuncId<'a> {
    pub fixed: &'a FuncIdFixed,
    pub name: &'a BStr,
    pub decorated_name_hash: Option<&'a U64<LE>>,
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct FuncIdFixed {
    /// Parent scope of the ID, 0 if global. This is used for the namespace that contains a symbol.
    /// The value points into the IPI.
    pub scope: ItemIdLe,

    /// The type of the function.
    pub func_type: TypeIndexLe,
}

impl<'a> Parse<'a> for FuncId<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
            decorated_name_hash: if p.len() >= 8 { Some(p.get()?) } else { None },
        })
    }
}

/// `LF_MFUNC_ID`
#[derive(Clone, Debug)]
pub struct MFuncId<'a> {
    pub fixed: &'a MFuncIdFixed,
    pub name: &'a BStr,
    pub decorated_name_hash: Option<&'a U64<LE>>,
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct MFuncIdFixed {
    /// type index of parent
    pub parent_type: TypeIndexLe,
    /// function type
    pub func_type: TypeIndexLe,
}

impl<'a> Parse<'a> for MFuncId<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
            decorated_name_hash: if p.len() >= 8 { Some(p.get()?) } else { None },
        })
    }
}

/// `LF_STRING_ID`
#[derive(Clone, Debug)]
pub struct StringId<'a> {
    pub id: ItemId,
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for StringId<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let id = p.u32()?;
        let name = p.strz()?;
        Ok(Self { id, name })
    }
}

/// `LF_SUBSTR_LIST` - A list of substrings
#[derive(Clone, Debug)]
pub struct SubStrList<'a> {
    pub ids: &'a [ItemIdLe],
}

impl<'a> Parse<'a> for SubStrList<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let n = p.u32()?;
        let ids = p.slice(n as usize)?;
        Ok(Self { ids })
    }
}

/// `LF_BUILDINFO`
#[derive(Clone, Debug)]
pub struct BuildInfo<'a> {
    pub args: &'a [ItemIdLe],
}

impl<'a> BuildInfo<'a> {
    pub fn arg(&self, index: BuildInfoIndex) -> Option<ItemId> {
        let a = self.args.get(index as usize)?;
        Some(a.get())
    }
}

impl<'a> Parse<'a> for BuildInfo<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let n = p.u16()?;
        let args = p.slice(n as usize)?;
        Ok(Self { args })
    }
}

/// Identifies indexes into `BuildInfo::args`.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u32)]
pub enum BuildInfoIndex {
    CurrentDirectory = 0,
    BuildTool = 1,           // Cl.exe
    SourceFile = 2,          // foo.cpp
    ProgramDatabaseFile = 3, // foo.pdb
    CommandArguments = 4,    // -I etc
}

/// Short strings for `BuildInfoIndex`
pub const BUILD_INFO_ARG_NAMES: [&str; 5] = ["cwd", "tool", "source_file", "pdb", "args"];

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct VFTable {
    /// type index of the root of path
    pub root: TypeIndexLe,
    /// type index of the path record
    pub path: TypeIndexLe,
    /// offset of virtual function table
    pub off: U32<LE>,
    /// segment of virtual function table
    pub seg: U16<LE>,
}
