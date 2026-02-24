//! Decodes symbols records. Reads the "Global Symbols" stream and per-module symbol streams.
//!
//! # References
//!
//! * [`cvinfo.h`](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/include/cvinfo.h)
//! * [CodeView Symbols](https://llvm.org/docs/PDB/CodeViewSymbols.html)

pub mod builder;
mod iter;
mod kind;
mod offset_segment;

#[doc(inline)]
pub use self::{iter::*, kind::SymKind, offset_segment::*};

use crate::parser::{Number, Parse, Parser, ParserError, ParserMut};
use crate::types::{ItemId, ItemIdLe, TypeIndex, TypeIndexLe};
use bitflags::bitflags;
use bstr::BStr;
use std::fmt::Debug;
use std::mem::size_of;
use zerocopy::{FromBytes, I32, Immutable, IntoBytes, KnownLayout, LE, U16, U32, Unaligned};

/// This header is shared by many records that can start a symbol scope.
#[derive(IntoBytes, FromBytes, Unaligned, Immutable, KnownLayout, Default, Clone, Debug)]
#[repr(C)]
#[allow(missing_docs)]
pub struct BlockHeader {
    /// If the record containing this `BlockHeader` is a top-level symbol record (not nested within
    /// another symbol), then this value is 0.
    ///
    /// If the record containing this `BlockHeader` is nested within another symbol, then this
    /// value is the offset in the symbol stream of the parent record.
    pub p_parent: U32<LE>,

    /// Offset in symbol stream of the `P_END` which terminates this block scope.
    pub p_end: U32<LE>,
}

/// Used for the header of procedure symbols. This is used for `S_LPROC32`, `S_GPROC32`,
/// `S_LPROC32_ID`, etc.
///
/// See `PROCSYM32` in `cvinfo.h`.
#[derive(IntoBytes, FromBytes, Immutable, KnownLayout, Unaligned, Debug)]
#[repr(C)]
#[allow(missing_docs)]
pub struct ProcFixed {
    /// This field is always zero; procedure symbols never have parents.
    pub p_parent: U32<LE>,

    /// The byte offset, relative to the start of this procedure record, of the `S_END` symbol that
    /// closes the scope of this symbol record.
    pub p_end: U32<LE>,

    pub p_next: U32<LE>,

    /// The length in bytes of the procedure instruction stream.
    pub proc_len: U32<LE>,

    /// The offset in bytes from the start of the procedure to the point where the stack frame has
    /// been set up. Parameter and frame variables can be viewed at this point.
    pub debug_start: U32<LE>,

    /// The offset in bytes from the start of the procedure to the point where the procedure is
    /// ready to return and has calculated its return value, if any. Frame and register variables
    /// can still be viewed.
    pub debug_end: U32<LE>,

    /// This field is either a `TypeIndex` that points into the TPI or is an `ItemId` that
    /// points into the IPI.
    ///
    /// This field is a `TypeIndex` for the following symbols: `S_GPROC32`, `S_LPROC32`,
    /// `S_LPROC32EX`, `S_LPROC32_DPC`, `S_GPROC32EX`.
    ///
    /// This field is a `ItemId` for `S_LPROC32_ID`, `S_GPROC32_ID`, `S_LPROC32_DPC_ID`,
    /// `S_GPROC32EX_ID`, `S_LPROC32EX_ID`.
    pub proc_type: TypeIndexLe,

    pub offset_segment: OffsetSegment,
    pub flags: u8,
}

bitflags! {
    /// Flags describing a procedure symbol.
    ///
    /// See: `CV_PROCFLAGS` in `cvinfo.h`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct ProcFlags: u8 {
        /// Frame pointer present.
        const NOFPO = 1 << 0;

        /// Interrupt return.
        const INT = 1 << 1;

        /// Far return.
        const FAR = 1 << 2;

        /// Does not return.
        const NEVER = 1 << 3;

        /// Label isn't fallen into.
        const NOTREACHED = 1 << 4;

        /// Custom calling convention.
        const CUST_CALL = 1 << 5;

        /// Marked as `noinline`.
        const NOINLINE = 1 << 6;

        /// Has debug information for optimized code.
        const OPTDBGINFO = 1 << 7;
    }
}

/// Used for `S_LPROC32` and `S_GPROC32`.
///
/// These records are found in Module Symbol Streams. They are very important; they describe the
/// beginning of a function (procedure), and they contain other symbols recursively (are a
/// "symbol scope"). The end of the sequence is terminated with an `S_END` symbol.
///
/// This is equivalent to the `PROCSYM32` type defined in `cvinfo.h`. This symbol begins with a
/// `BLOCKSYM` header
///
/// # References
/// * See `PROCSYM32` in `cvinfo.h`
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Proc<'a> {
    pub fixed: &'a ProcFixed,
    pub name: &'a BStr,
}

impl<'a> Proc<'a> {
    /// View the procedure `flags` field as bit flags.
    pub fn flags(&self) -> ProcFlags {
        ProcFlags::from_bits_retain(self.fixed.flags)
    }
}

impl<'a> Parse<'a> for Proc<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

// Basic framing and decoding test
#[test]
fn test_parse_proc() {
    #[rustfmt::skip]
    let data = &[
        /* 0x0000 */ 0x2e, 0, 0x10, 0x11,       // size and S_GPROC32
        /* 0x0004 */ 0, 0, 0, 0,                // p_parent
        /* 0x0008 */ 0x40, 0, 0, 0,             // p_end
        /* 0x000c */ 0, 0, 0, 0,                // p_next
        /* 0x0010 */ 42, 0, 0, 0,               // proc_len
        /* 0x0014 */ 10, 0, 0, 0,               // debug_start
        /* 0x0018 */ 20, 0, 0, 0,               // debug_end
        /* 0x001c */ 0xee, 0x10, 0, 0,          // proc_type
        /* 0x0020 */ 0xcc, 0x1, 0, 0,           // offset
        /* 0x0024 */ 1, 0, 0x50, b'm',          // segment, flags, beginning of name
        /* 0x0028 */ b'e', b'm', b's', b'e',    // name
        /* 0x002c */ b't', 0, 0xf1, 0xf2,       // end and padding
        /* 0x0030 */ 2, 0, 6, 0                 // size = 2 and S_END
        /* 0x0034 */
    ];

    let mut i = SymIter::new(data);

    let s0 = i.next().unwrap();
    assert_eq!(s0.kind, SymKind::S_GPROC32);
    assert_eq!(s0.data.len(), 0x2c);

    match s0.parse().unwrap() {
        SymData::Proc(proc) => {
            assert_eq!(proc.fixed.p_parent.get(), 0);
            assert_eq!(proc.fixed.p_end.get(), 0x40);
            assert_eq!(proc.name, "memset");
        }
        _ => panic!(),
    }

    let s1 = i.next().unwrap();
    assert_eq!(s1.kind, SymKind::S_END);
    assert!(s1.data.is_empty());
}

/// `S_GMANPROC`, `S_LMANPROC` - Managed Procedure Start
///
/// See `MANPROCSYM` in `cvinfo.h`.
#[derive(IntoBytes, FromBytes, Immutable, KnownLayout, Unaligned, Debug)]
#[repr(C)]
#[allow(missing_docs)]
pub struct ManagedProcFixed {
    pub p_parent: U32<LE>,
    pub p_end: U32<LE>,
    pub p_next: U32<LE>,
    pub proc_len: U32<LE>,
    pub debug_start: U32<LE>,
    pub debug_end: U32<LE>,
    pub token: U32<LE>,
    pub offset_segment: OffsetSegment,
    pub flags: u8,
    pub return_reg: U16<LE>,
}

/// `S_GMANPROC`, `S_LMANPROC` - Managed Procedure Start
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct ManagedProc<'a> {
    pub fixed: &'a ManagedProcFixed,
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for ManagedProc<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Clone, Debug, Default)]
#[allow(missing_docs)]
pub struct ThunkFixed {
    pub block: BlockHeader,
    pub p_next: U32<LE>,
    pub offset_segment: OffsetSegment,
    pub length: U16<LE>,
    pub thunk_ordinal: u8,
    // name: strz
    // variant: [u8]
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct Thunk<'a> {
    pub fixed: &'a ThunkFixed,
    pub name: &'a BStr,
    pub variant: &'a [u8],
}

impl<'a> Parse<'a> for Thunk<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
            variant: p.take_rest(),
        })
    }
}

/// Describes the start of every symbol record.
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Clone, Default)]
pub struct SymHeader {
    /// The length in bytes of the record.
    ///
    /// This length _does not_ count the length itself, but _does_ count the `kind` that follows it.
    /// Therefore, all well-formed symbol records have `len >= 2`.
    pub len: U16<LE>,

    /// The kind of the symbol.  See `SymKind`.
    pub kind: U16<LE>,
}

/// Points to one symbol record in memory and gives its kind.
#[derive(Clone)]
pub struct Sym<'a> {
    /// The kind of the symbol.
    pub kind: SymKind,
    /// The contents of the record. This slice does _not_ include the `len` or `kind` fields.
    pub data: &'a [u8],
}

impl<'a> Sym<'a> {
    /// Parse the payload of the symbol.
    pub fn parse(&self) -> Result<SymData<'a>, ParserError> {
        SymData::parse(self.kind, self.data)
    }

    /// Parses the payload of the symbol with a type chosen by the caller.
    ///
    /// This is useful when the caller has already tested `Sym::kind` and knows the type of the
    /// payload.
    pub fn parse_as<T>(&self) -> Result<T, ParserError>
    where
        T: Parse<'a>,
    {
        let mut p = Parser::new(self.data);
        p.parse::<T>()
    }
}

impl<'a> Debug for Sym<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{:?}", self.kind)
    }
}

/// Points to one symbol record in memory and gives its kind. Allows mutation of the contents of
/// the symbol record.
pub struct SymMut<'a> {
    /// The kind of the symbol.
    pub kind: SymKind,
    /// The contents of the record. This slice does _not_ include the `len` or `kind` fields.
    pub data: &'a mut [u8],
}

/// PUBSYM32
///
/// ```text
/// typedef struct PUBSYM32 {
///     unsigned short  reclen;     // Record length
///     unsigned short  rectyp;     // S_PUB32
///     CV_PUBSYMFLAGS  pubsymflags;
///     CV_uoff32_t     off;
///     unsigned short  seg;
///     unsigned char   name[1];    // Length-prefixed name
/// } PUBSYM32;
/// ```
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Pub<'a> {
    pub fixed: &'a PubFixed,
    pub name: &'a BStr,
}

impl<'a> Pub<'a> {
    /// Gets the `segment:offset` of this symbol.
    pub fn offset_segment(&self) -> OffsetSegment {
        self.fixed.offset_segment
    }
}

#[allow(missing_docs)]
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct PubFixed {
    pub flags: U32<LE>,
    pub offset_segment: OffsetSegment,
    // name: &str
}

#[allow(missing_docs)]
impl<'a> Parse<'a> for Pub<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

impl<'a> Pub<'a> {
    /// Parses `S_PUB32_ST`
    pub fn parse_st(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strt_raw()?,
        })
    }
}

/// Parsed form of `S_CONSTANT`
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Constant<'a> {
    pub type_: TypeIndex,
    pub value: Number<'a>,
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for Constant<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            type_: p.type_index()?,
            value: p.number()?,
            name: p.strz()?,
        })
    }
}

/// Parsed form of `S_CONSTANT`
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct ManagedConstant<'a> {
    pub token: u32,
    pub value: Number<'a>,
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for ManagedConstant<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            token: p.u32()?,
            value: p.number()?,
            name: p.strz()?,
        })
    }
}

/// Several symbols use this structure: `S_PROCREF`, `S_LPROCREF`, `S_DATAREF`. These symbols
/// are present in the Global Symbol Stream, not in module symbol streams.
///
/// These `S_*REF` symbols tell you where to find a specific global symbol, but they do not directly
/// describe the symbol. Instead, you have to load the corresponding module
///
///
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct RefSym2<'a> {
    pub header: &'a RefSym2Fixed,
    pub name: &'a BStr,
}

#[allow(missing_docs)]
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct RefSym2Fixed {
    /// Checksum of the name (called `SUC` in C++ code)
    ///
    /// This appears to be set to zero.
    pub name_checksum: U32<LE>,

    /// Offset of actual symbol in $$Symbols
    ///
    /// This is the byte offset into the module symbol stream for this symbol. The `module_index`
    /// field tells you which symbol stream to load, to resolve this value.
    pub symbol_offset: U32<LE>,

    /// The 1-based index of the module containing the actual symbol.
    ///
    /// This value is 1-based. Subtract 1 from this value before indexing into a zero-based module array.
    pub module_index: U16<LE>,
    // pub name: strz, // hidden name made a first class member
}

impl<'a> Parse<'a> for RefSym2<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            header: p.get()?,
            name: p.strz()?,
        })
    }
}

#[allow(missing_docs)]
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct ThreadStorageFixed {
    pub type_: TypeIndexLe,
    pub offset_segment: OffsetSegment,
}

/// Record data for `S_LTHREAD32` and `S_GTHREAD32`. These describes thread-local storage.
///
/// Thread-local storage is declared using `__declspec(thread)` or `thread_static`, in C++.
#[derive(Clone, Debug)]
pub struct ThreadStorageData<'a> {
    #[allow(missing_docs)]
    pub header: &'a ThreadStorageFixed,
    #[allow(missing_docs)]
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for ThreadStorageData<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            header: p.get()?,
            name: p.strz()?,
        })
    }
}

#[allow(missing_docs)]
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct DataFixed {
    pub type_: TypeIndexLe,
    pub offset_segment: OffsetSegment,
}

/// Record data for `S_LDATA32` and `S_GDATA32`. These describe global storage.
#[allow(missing_docs)]
#[derive(Clone)]
pub struct Data<'a> {
    pub header: &'a DataFixed,
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for Data<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            header: p.get()?,
            name: p.strz()?,
        })
    }
}

impl<'a> Debug for Data<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Data: {} {:?} {}",
            self.header.offset_segment,
            self.header.type_.get(),
            self.name
        )
    }
}

/// Record data for `S_UDT` symbols
#[derive(Clone, Debug)]
pub struct Udt<'a> {
    /// The type of the UDT
    pub type_: TypeIndex,
    /// Name
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for Udt<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            type_: p.type_index()?,
            name: p.strz()?,
        })
    }
}

/// `S_OBJNAME`
#[derive(Clone, Debug)]
pub struct ObjectName<'a> {
    /// A robust signature that will change every time that the module will be compiled or
    /// different in any way. It should be at least a CRC32 based upon module name and contents.
    pub signature: u32,
    /// Full path of the object file.
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for ObjectName<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            signature: p.u32()?,
            name: p.strz()?,
        })
    }
}

/// `S_COMPILE3`
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[repr(C)]
#[allow(missing_docs)]
pub struct Compile3Fixed {
    pub flags: U32<LE>,
    pub machine: U16<LE>,
    pub frontend_major: U16<LE>,
    pub frontend_minor: U16<LE>,
    pub frontend_build: U16<LE>,
    pub frontend_qfe: U16<LE>,
    pub ver_major: U16<LE>,
    pub ver_minor: U16<LE>,
    pub ver_build: U16<LE>,
    pub ver_qfe: U16<LE>,
    // name: strz
}

/// `S_COMPILE3`
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Compile3<'a> {
    pub fixed: &'a Compile3Fixed,
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for Compile3<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

/// `S_FRAMEPROC`: This symbol is used for indicating a variety of extra information regarding a
/// procedure and its stack frame. If any of the flags are non-zero, this record should be added
/// to the symbols for that procedure.
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct FrameProc {
    /// Count of bytes in the whole stack frame.
    frame_size: U32<LE>,
    /// Count of bytes in the frame allocated as padding.
    pad_size: U32<LE>,
    /// Offset of pad bytes from the base of the frame.
    pad_offset: U32<LE>,
    /// Count of bytes in frame allocated for saved callee-save registers.
    save_regs_size: U32<LE>,
    offset_exception_handler: U32<LE>,
    exception_handler_section: U16<LE>,
    padding: U16<LE>,
    flags: U32<LE>,
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct RegRelFixed {
    pub offset: U32<LE>,
    pub ty: TypeIndexLe,
    pub register: U16<LE>,
    // name: strz
}

/// `S_REGREGL32`: This symbol specifies symbols that are allocated relative to a register.
/// This should be used on all platforms besides x86 and on x86 when the register is not a form of EBP.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct RegRel<'a> {
    pub fixed: &'a RegRelFixed,
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for RegRel<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

/// Block Start: This symbol specifies the start of an inner block of lexically scoped symbols.
/// The lexical scope is terminated by a matching `S_END` symbol.
///
/// This symbol should only be nested (directly or indirectly) within a function symbol
/// (`S_GPROC32`, `S_LPROC32`, etc.).
///
/// See `BLOCKSYM32`
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct Block<'a> {
    pub fixed: &'a BlockFixed,
    pub name: &'a BStr,
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct BlockFixed {
    /// Header of the block
    pub header: BlockHeader,

    /// Length in bytes of the scope of this block within the executable code stream.
    pub length: U32<LE>,

    pub offset_segment: OffsetSegment,
}

impl<'a> Parse<'a> for Block<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

/// Local Symbol: This symbol defines a local variable.
///
/// This symbol must be nested (directly or indirectly) within a function symbol. It must be
/// followed by more range descriptions.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct Local<'a> {
    pub fixed: &'a LocalFixed,
    pub name: &'a BStr,
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct LocalFixed {
    pub ty: TypeIndexLe,
    /// The spec says this is a 32-bit flags field, but the actual records show that this is 16-bit.
    pub flags: U16<LE>,
    // name: strz
}

impl<'a> Parse<'a> for Local<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

/// Represents an address range, used for optimized code debug info
///
/// See `CV_LVAR_ADDR_RANGE`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct LVarAddrRange {
    /// Start of the address range
    pub start: OffsetSegment,
    /// Size of the range in bytes.
    pub range_size: U16<LE>,
}

/// Represents the holes in overall address range, all address is pre-bbt.
/// it is for compress and reduce the amount of relocations need.
///
/// See `CV_LVAR_ADDR_GAP`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct LVarAddrGap {
    /// relative offset from the beginning of the live range.
    pub gap_start_offset: U16<LE>,
    /// length of this gap, in bytes
    pub range_size: U16<LE>,
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct DefRangeFixed {
    /// DIA program to evaluate the value of the symbol
    pub program: U32<LE>,

    pub range: LVarAddrRange,
    // gaps: [LVAddrGap]
}

/// `S_DEFRANGE`
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct DefRange<'a> {
    pub fixed: &'a DefRangeFixed,
    pub gaps: &'a [LVarAddrGap],
}

impl<'a> Parse<'a> for DefRange<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let fixed = p.get()?;
        let gaps = p.slice(p.len() / size_of::<LVarAddrGap>())?;
        Ok(Self { fixed, gaps })
    }
}

/// `S_DEFRANGE_FRAMEPOINTER_REL`: A live range of frame variable
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct DefRangeSymFramePointerRelFixed {
    pub offset_to_frame_pointer: U32<LE>,

    /// Range of addresses where this program is valid
    pub range: LVarAddrRange,
}

/// `S_DEFRANGE_FRAMEPOINTER_REL`: A live range of frame variable
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct DefRangeSymFramePointerRel<'a> {
    pub fixed: &'a DefRangeSymFramePointerRelFixed,
    // The value is not available in following gaps.
    pub gaps: &'a [LVarAddrGap],
}

impl<'a> Parse<'a> for DefRangeSymFramePointerRel<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let fixed = p.get()?;
        let gaps = p.slice(p.len() / size_of::<LVarAddrGap>())?;
        Ok(Self { fixed, gaps })
    }
}

/// Attributes for a register range
///
/// See `CV_RANGEATTR`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct RangeAttrLe {
    // unsigned short  maybe : 1;    // May have no user name on one of control flow path.
    // unsigned short  padding : 15; // Padding for future use.
    pub value: U16<LE>,
}

/// `S_DEFRANGE_REGISTER` - A live range of en-registed variable
///
/// See `DEFRANGESYMREGISTER`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct DefRangeRegisterFixed {
    /// Register to hold the value of the symbol
    pub reg: U16<LE>,
    // Attribute of the register range.
    pub attr: RangeAttrLe,
}

/// `S_DEFRANGE_REGISTER`
///
/// See `DEFRANGESYMREGISTER`
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct DefRangeRegister<'a> {
    pub fixed: &'a DefRangeRegisterFixed,
    pub gaps: &'a [u8],
}

impl<'a> Parse<'a> for DefRangeRegister<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            gaps: p.take_rest(),
        })
    }
}

/// `S_DEFRANGE_REGISTER_REL`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct DefRangeRegisterRel<'a> {
    pub fixed: &'a DefRangeRegisterRelFixed,

    /// The value is not available in following gaps.
    pub gaps: &'a [u8],
}

/// `S_DEFRANGE_REGISTER_REL`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct DefRangeRegisterRelFixed {
    /// Register to hold the base pointer of the symbol
    pub base_reg: U16<LE>,

    /// ```text
    /// unsigned short  spilledUdtMember : 1;   // Spilled member for s.i.
    /// unsigned short  padding          : 3;   // Padding for future use.
    /// unsigned short  offsetParent     : CV_OFFSET_PARENT_LENGTH_LIMIT;  // Offset in parent variable.
    /// ```
    pub flags: U16<LE>,

    /// offset to register
    pub base_pointer_offset: I32<LE>,

    /// Range of addresses where this program is valid
    pub range: LVarAddrRange,
}

impl<'a> Parse<'a> for DefRangeRegisterRel<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            gaps: p.take_rest(),
        })
    }
}

/// `S_DEFRANGE_FRAMEPOINTER_REL_FULL_SCOPE`
///
/// A frame variable valid in all function scope.
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Clone, Debug)]
#[repr(C)]
pub struct DefRangeFramePointerRelFullScope {
    /// offset to frame pointer
    pub frame_pointer_offset: I32<LE>,
}

/// `S_DEFRANGE_SUBFIELD_REGISTER`
///
/// See `DEFRANGESYMSUBFIELDREGISTER`
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct DefRangeSubFieldRegister<'a> {
    pub fixed: &'a DefRangeSubFieldRegisterFixed,
    pub gaps: &'a [u8],
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct DefRangeSubFieldRegisterFixed {
    pub reg: U16<LE>,
    pub attr: RangeAttrLe,
    pub flags: U32<LE>,
    pub range: LVarAddrRange,
}

impl<'a> Parse<'a> for DefRangeSubFieldRegister<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            gaps: p.take_rest(),
        })
    }
}

/// `S_GMANPROC`, `S_LMANPROC`, `S_GMANPROCIA64`, `S_LMANPROCIAC64`
///
/// See `MANPROCSYM`
pub struct ManProcSym<'a> {
    #[allow(missing_docs)]
    pub fixed: &'a ManProcSymFixed,
    #[allow(missing_docs)]
    pub name: &'a BStr,
}

/// MSIL / CIL token value
pub type TokenIdLe = U32<LE>;

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct ManProcSymFixed {
    pub block: BlockHeader,
    /// pointer to next symbol
    pub pnext: U32<LE>,
    /// Proc length
    pub len: U32<LE>,
    /// Debug start offset
    pub dbg_start: U32<LE>,
    /// Debug end offset
    pub dbg_end: U32<LE>,
    // COM+ metadata token for method
    pub token: TokenIdLe,
    pub off: U32<LE>,
    pub seg: U16<LE>,
    pub flags: u8, // CV_PROCFLAGS: Proc flags
    pub padding: u8,
    // Register return value is in (may not be used for all archs)
    pub ret_reg: U16<LE>,
    // name: strz    // optional name field
}

impl<'a> Parse<'a> for ManProcSym<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

/// `S_TRAMPOLINE`
#[derive(Clone, Debug)]
pub struct Trampoline<'a> {
    /// Fixed header
    pub fixed: &'a TrampolineFixed,

    /// Data whose interpretation depends on `tramp_type`
    pub rest: &'a [u8],
}

/// `S_TRAMPOLINE`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Clone, Debug)]
pub struct TrampolineFixed {
    /// trampoline sym subtype
    pub tramp_type: U16<LE>,
    /// size of the thunk
    pub cb_thunk: U16<LE>,
    /// offset of the thunk
    pub off_thunk: U32<LE>,
    /// offset of the target of the thunk
    pub off_target: U32<LE>,
    /// section index of the thunk
    pub sect_thunk: U16<LE>,
    /// section index of the target of the thunk
    pub sect_target: U16<LE>,
}

impl<'a> Parse<'a> for Trampoline<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            rest: p.take_rest(),
        })
    }
}

/// `S_BUILDINFO` - Build info for a module
///
/// This record is present only in module symbol streams.
#[derive(Clone, Debug)]
pub struct BuildInfo {
    /// ItemId points to an `LF_BUILDINFO` record in IPI
    pub item: ItemId,
}

impl<'a> Parse<'a> for BuildInfo {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self { item: p.u32()? })
    }
}

/// `S_UNAMESPACE` - Using Namespace
#[derive(Clone, Debug)]
pub struct UsingNamespace<'a> {
    /// The namespace, e.g. `std`
    pub namespace: &'a BStr,
}

impl<'a> Parse<'a> for UsingNamespace<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            namespace: p.strz()?,
        })
    }
}

/// `S_LABEL32`
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct Label<'a> {
    pub fixed: &'a LabelFixed,
    pub name: &'a BStr,
}

/// `S_LABEL32`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct LabelFixed {
    pub offset_segment: OffsetSegment,
    pub flags: u8,
}

impl<'a> Parse<'a> for Label<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

/// Data for `S_CALLERS`, `S_CALLEES`, `S_INLINEES`.
#[derive(Clone, Debug)]
pub struct FunctionList<'a> {
    /// The list of functions, in the IPI. Each is either `LF_FUNC_ID` or `LF_MFUNC_ID`.
    pub funcs: &'a [ItemIdLe],

    /// Counts for each function.
    ///
    /// The values in `counts` parallel the items in `funcs`, but the length of `invocations` can be
    /// less than the length of `funcs`. Unmatched counts are assumed to be zero.
    ///
    /// This is empty for `S_INLINEES`.
    pub counts: &'a [U32<LE>],
}

impl<'a> Parse<'a> for FunctionList<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let num_funcs = p.u32()? as usize;
        let funcs: &[ItemIdLe] = p.slice(num_funcs)?;
        let num_counts = num_funcs.min(p.len() / size_of::<U32<LE>>());
        let counts = p.slice(num_counts)?;
        Ok(Self { funcs, counts })
    }
}

/// `S_INLINESITE`
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct InlineSite<'a> {
    pub fixed: &'a InlineSiteFixed,
    /// an array of compressed binary annotations.
    pub binary_annotations: &'a [u8],
}

/// `S_INLINESITE`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct InlineSiteFixed {
    pub block: BlockHeader,
    pub inlinee: ItemIdLe,
}

impl<'a> Parse<'a> for InlineSite<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            binary_annotations: p.take_rest(),
        })
    }
}

/// `S_INLINESITE2`
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct InlineSite2<'a> {
    pub fixed: &'a InlineSite2Fixed,
    /// an array of compressed binary annotations.
    pub binary_annotations: &'a [u8],
}

/// `S_INLINESITE`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct InlineSite2Fixed {
    pub block: BlockHeader,
    pub inlinee: ItemIdLe,
    pub invocations: U32<LE>,
}

impl<'a> Parse<'a> for InlineSite2<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            binary_annotations: p.take_rest(),
        })
    }
}

/// `S_FRAMECOOKIE`: Symbol for describing security cookie's position and type
// (raw, xor'd with esp, xor'd with ebp).
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct FrameCookie {
    /// Frame relative offset
    pub offset: I32<LE>,
    pub reg: U16<LE>,
    pub cookie_type: u8,
    pub flags: u8,
}

/// `S_CALLSITEINFO`
///
/// Symbol for describing indirect calls when they are using
/// a function pointer cast on some other type or temporary.
/// Typical content will be an LF_POINTER to an LF_PROCEDURE
/// type record that should mimic an actual variable with the
/// function pointer type in question.
///
/// Since the compiler can sometimes tail-merge a function call
/// through a function pointer, there may be more than one
/// S_CALLSITEINFO record at an address.  This is similar to what
/// you could do in your own code by:
///
/// ```text
///  if (expr)
///      pfn = &function1;
///  else
///      pfn = &function2;
///
///  (*pfn)(arg list);
/// ```
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct CallSiteInfo {
    pub offset: OffsetSegment,
    pub padding: U16<LE>,
    pub func_type: TypeIndexLe,
}

/// `S_HEAPALLOCSITE`
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct HeapAllocSite {
    pub offset: OffsetSegment,
    /// length of heap allocation call instruction
    pub instruction_size: U16<LE>,
    pub func_type: TypeIndexLe,
}

/// `S_ANNOTATION`
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Annotation<'a> {
    pub fixed: &'a AnnotationFixed,
    pub strings: &'a [u8],
}

#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
#[allow(missing_docs)]
pub struct AnnotationFixed {
    pub offset: OffsetSegment,
    pub num_strings: U16<LE>,
}

impl<'a> Parse<'a> for Annotation<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            strings: p.take_rest(),
        })
    }
}

impl<'a> Annotation<'a> {
    /// Iterates the strings stored in the annotation.
    pub fn iter_strings(&self) -> AnnotationIterStrings<'a> {
        AnnotationIterStrings {
            num_strings: self.fixed.num_strings.get(),
            bytes: self.strings,
        }
    }
}

/// Iterator state for [`Annotation::iter_strings`].
#[allow(missing_docs)]
pub struct AnnotationIterStrings<'a> {
    pub num_strings: u16,
    pub bytes: &'a [u8],
}

impl<'a> Iterator for AnnotationIterStrings<'a> {
    type Item = &'a BStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.num_strings == 0 {
            return None;
        }

        self.num_strings -= 1;
        let mut p = Parser::new(self.bytes);
        let s = p.strz().ok()?;
        self.bytes = p.into_rest();
        Some(s)
    }
}

/// Hot-patched function
#[derive(Clone, Debug)]
pub struct HotPatchFunc<'a> {
    /// ID of the function
    pub func: ItemId,

    /// The name of the function
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for HotPatchFunc<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            func: p.u32()?,
            name: p.strz()?,
        })
    }
}

/// Data for `S_ARMSWITCHTABLE`.
///
/// This describes a switch table (jump table).
///
/// MSVC generates this symbol only when targeting ARM64.
/// LLVM generates this symbol for all target architectures.
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Clone)]
pub struct ArmSwitchTable {
    /// Section-relative offset to the base for switch offsets
    pub offset_base: U32<LE>,
    /// Section index of the base for switch offsets
    pub sect_base: U16<LE>,
    /// type of each entry
    pub switch_type: U16<LE>,
    /// Section-relative offset to the table branch instruction
    pub offset_branch: U32<LE>,
    /// Section-relative offset to the start of the table
    pub offset_table: U32<LE>,
    /// Section index of the table branch instruction
    pub sect_branch: U16<LE>,
    /// Section index of the table
    pub sect_table: U16<LE>,
    /// number of switch table entries
    pub num_entries: U32<LE>,
}

impl ArmSwitchTable {
    /// The `[segment:offset]` of the jump base.
    ///
    /// This is the base address of the target of the jump. The value stored within the jump table
    /// entry is added to this base.
    ///
    /// LLVM often generates tables where `base` and `table` have the same address, but this is
    /// not necessarily true for all tables.
    pub fn base(&self) -> OffsetSegment {
        OffsetSegment {
            offset: self.offset_base,
            segment: self.sect_base,
        }
    }

    /// The `[segment:offset]` of the branch instruction.
    pub fn branch(&self) -> OffsetSegment {
        OffsetSegment {
            offset: self.offset_branch,
            segment: self.sect_branch,
        }
    }

    /// The `[segment:offset]` of the jump table.
    pub fn table(&self) -> OffsetSegment {
        OffsetSegment {
            offset: self.offset_table,
            segment: self.sect_table,
        }
    }

    /// The type of switch table (2-byte, 4-byte, etc.).
    pub fn switch_type(&self) -> ArmSwitchType {
        ArmSwitchType(self.switch_type.get())
    }

    /// The number of entries in the jump table.
    pub fn num_entries(&self) -> u32 {
        self.num_entries.get()
    }
}

impl core::fmt::Debug for ArmSwitchTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ArmSwitchTable")
            .field("base", &self.base())
            .field("branch", &self.branch())
            .field("table", &self.table())
            .field("switch_type", &self.switch_type())
            .field("num_entries", &self.num_entries())
            .finish()
    }
}

/// The type of switch table, as defined by `S_ARMSWITCHTABLE`.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ArmSwitchType(pub u16);

impl ArmSwitchType {
    /// Signed 1-byte offset
    pub const INT1: ArmSwitchType = ArmSwitchType(0);
    /// Unsigned 1-byte offset
    pub const UINT1: ArmSwitchType = ArmSwitchType(1);
    /// Signed 2-byte offset
    pub const INT2: ArmSwitchType = ArmSwitchType(2);
    /// Unsigned 2-byte offset
    pub const UINT2: ArmSwitchType = ArmSwitchType(3);
    /// Signed 4-byte offset
    pub const INT4: ArmSwitchType = ArmSwitchType(4);
    /// Unsigned 4-byte offset
    pub const UINT4: ArmSwitchType = ArmSwitchType(5);
    /// Absolute pointer (no base)
    pub const POINTER: ArmSwitchType = ArmSwitchType(6);
    /// Unsigned 1-byte offset, shift left by 1
    pub const UINT1SHL1: ArmSwitchType = ArmSwitchType(7);
    /// Unsigned 2-byte offset, shift left by 2
    pub const UINT2SHL1: ArmSwitchType = ArmSwitchType(8);
    /// Signed 1-byte offset, shift left by 1
    pub const INT1SHL1: ArmSwitchType = ArmSwitchType(9);
    /// Signed 2-byte offset, shift left by 1
    pub const INT2SHL1: ArmSwitchType = ArmSwitchType(10);
    // CV_SWT_TBB          = CV_SWT_UINT1SHL1,
    // CV_SWT_TBH          = CV_SWT_UINT2SHL1,
}

impl core::fmt::Debug for ArmSwitchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        static NAMES: [&str; 11] = [
            "INT1",
            "UINT1",
            "INT2",
            "UINT2",
            "INT4",
            "UINT4",
            "POINTER",
            "UINT1SHL1",
            "UINT2SHL1",
            "INT1SHL1",
            "INT2SHL1",
        ];

        if let Some(&s) = NAMES.get(self.0 as usize) {
            f.write_str(s)
        } else {
            write!(f, "??{}", self.0)
        }
    }
}

// Trampoline subtypes

/// Incremental thunks
pub const TRAMPOLINE_KIND_INCREMENTAL: u16 = 0;
/// Branch island thunks
pub const TRAMPOLINE_KIND_BRANCH_ISLAND: u16 = 1;

/// The fixed header of `S_COFFGROUP` symbols.
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct CoffGroupFixed {
    /// Size in bytes of the coff group
    pub cb: U32<LE>,
    /// Characteristics flags. These are the same as the COFF section characteristics.
    ///
    /// See: <https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-image_section_header>
    pub characteristics: U32<LE>,
    /// Location of the COFF group
    pub off_seg: OffsetSegment,
}

/// For `S_COFFGROUP`.
///
/// `S_COFFGROUP` records are present in the `* Linker *` special module. These records describe
/// contiguous subsections within COFF sections. For example, `.text$mn` is a COFF group within
/// the `.text` segment.
#[derive(Clone, Debug)]
pub struct CoffGroup<'a> {
    /// The fixed-size header
    pub fixed: &'a CoffGroupFixed,
    /// The name of the COFF group
    pub name: &'a BStr,
}

impl<'a> Parse<'a> for CoffGroup<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

/// For `S_SECTION`
#[derive(Clone, Debug)]
pub struct Section<'a> {
    /// The fixed-size header
    pub fixed: &'a SectionFixed,
    /// The name of the section
    pub name: &'a BStr,
}

/// The fixed header of `S_SECTION` symbols.
#[repr(C)]
#[derive(IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned, Debug)]
pub struct SectionFixed {
    /// Section number
    pub section: U16<LE>,
    /// Alignment of this section (power of 2)
    pub align: u8,
    /// Reserved
    pub reserved: u8,
    /// RVA of this section base
    pub rva: U32<LE>,
    /// Size in bytes of this section
    pub cb: U32<LE>,
    /// Section characteristics (bit flags)
    pub characteristics: U32<LE>,
}

impl<'a> Parse<'a> for Section<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(Self {
            fixed: p.get()?,
            name: p.strz()?,
        })
    }
}

/// Parsed data from a symbol record
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum SymData<'a> {
    Unknown,
    ObjName(ObjectName<'a>),
    Compile3(Compile3<'a>),
    Proc(Proc<'a>),
    Thunk(Thunk<'a>),
    Udt(Udt<'a>),
    Constant(Constant<'a>),
    ManagedConstant(ManagedConstant<'a>),
    RefSym2(RefSym2<'a>),
    Data(Data<'a>),
    ThreadData(ThreadStorageData<'a>),
    Pub(Pub<'a>),
    End,
    FrameProc(&'a FrameProc),
    RegRel(RegRel<'a>),
    Block(Block<'a>),
    Local(Local<'a>),
    DefRange(DefRange<'a>),
    DefRangeFramePointerRel(DefRangeSymFramePointerRel<'a>),
    DefRangeRegister(DefRangeRegister<'a>),
    DefRangeRegisterRel(DefRangeRegisterRel<'a>),
    DefRangeFramePointerRelFullScope(&'a DefRangeFramePointerRelFullScope),
    DefRangeSubFieldRegister(DefRangeSubFieldRegister<'a>),
    Trampoline(Trampoline<'a>),
    BuildInfo(BuildInfo),
    UsingNamespace(UsingNamespace<'a>),
    InlineSiteEnd,
    Label(Label<'a>),
    FunctionList(FunctionList<'a>),
    InlineSite(InlineSite<'a>),
    InlineSite2(InlineSite2<'a>),
    FrameCookie(&'a FrameCookie),
    CallSiteInfo(&'a CallSiteInfo),
    HeapAllocSite(&'a HeapAllocSite),
    ManagedProc(ManagedProc<'a>),
    Annotation(Annotation<'a>),
    HotPatchFunc(HotPatchFunc<'a>),
    CoffGroup(CoffGroup<'a>),
    ArmSwitchTable(&'a ArmSwitchTable),
    Section(Section<'a>),
}

impl<'a> SymData<'a> {
    /// Parses a symbol record. The caller has already parsed the length and kind of the record.
    /// The `data` parameter does not include the length or kind.
    pub fn parse(kind: SymKind, data: &'a [u8]) -> Result<Self, ParserError> {
        let mut p = Parser::new(data);
        Self::from_parser(kind, &mut p)
    }

    /// Parses a symbol record. The caller has already parsed the length and kind of the record.
    /// The `p` parameter does not include the length or kind.
    ///
    /// This function allows the caller to observe how many bytes were actually consumed from
    /// the input stream.
    pub fn from_parser(kind: SymKind, p: &mut Parser<'a>) -> Result<Self, ParserError> {
        Ok(match kind {
            SymKind::S_OBJNAME => Self::ObjName(p.parse()?),
            SymKind::S_GPROC32 | SymKind::S_LPROC32 => Self::Proc(p.parse()?),
            SymKind::S_COMPILE3 => Self::Compile3(p.parse()?),
            SymKind::S_UDT => Self::Udt(p.parse()?),
            SymKind::S_CONSTANT => Self::Constant(p.parse()?),
            SymKind::S_MANCONSTANT => Self::Constant(p.parse()?),
            SymKind::S_PUB32 => Self::Pub(p.parse()?),
            SymKind::S_PUB32_ST => Self::Pub(Pub::parse_st(p)?),

            SymKind::S_PROCREF
            | SymKind::S_LPROCREF
            | SymKind::S_DATAREF
            | SymKind::S_ANNOTATIONREF => Self::RefSym2(p.parse()?),

            SymKind::S_LDATA32 | SymKind::S_GDATA32 | SymKind::S_LMANDATA | SymKind::S_GMANDATA => {
                Self::Data(p.parse()?)
            }

            SymKind::S_LTHREAD32 | SymKind::S_GTHREAD32 => Self::ThreadData(p.parse()?),
            SymKind::S_END => Self::End,
            SymKind::S_FRAMEPROC => Self::FrameProc(p.get()?),
            SymKind::S_REGREL32 => Self::RegRel(p.parse()?),
            SymKind::S_BLOCK32 => Self::Block(p.parse()?),
            SymKind::S_LOCAL => Self::Local(p.parse()?),
            SymKind::S_DEFRANGE => Self::DefRange(p.parse()?),
            SymKind::S_DEFRANGE_FRAMEPOINTER_REL => Self::DefRangeFramePointerRel(p.parse()?),
            SymKind::S_DEFRANGE_REGISTER => Self::DefRangeRegister(p.parse()?),
            SymKind::S_DEFRANGE_REGISTER_REL => Self::DefRangeRegisterRel(p.parse()?),
            SymKind::S_DEFRANGE_FRAMEPOINTER_REL_FULL_SCOPE => {
                Self::DefRangeFramePointerRelFullScope(p.get()?)
            }
            SymKind::S_DEFRANGE_SUBFIELD_REGISTER => Self::DefRangeSubFieldRegister(p.parse()?),
            SymKind::S_TRAMPOLINE => Self::Trampoline(p.parse()?),
            SymKind::S_BUILDINFO => Self::BuildInfo(p.parse()?),
            SymKind::S_UNAMESPACE => Self::UsingNamespace(p.parse()?),
            SymKind::S_INLINESITE_END => Self::InlineSiteEnd,
            SymKind::S_LABEL32 => Self::Label(p.parse()?),
            SymKind::S_CALLEES | SymKind::S_CALLERS => Self::FunctionList(p.parse()?),
            SymKind::S_INLINESITE => Self::InlineSite(p.parse()?),
            SymKind::S_INLINESITE2 => Self::InlineSite2(p.parse()?),
            SymKind::S_INLINEES => Self::FunctionList(p.parse()?),
            SymKind::S_FRAMECOOKIE => Self::FrameCookie(p.get()?),
            SymKind::S_CALLSITEINFO => Self::CallSiteInfo(p.get()?),
            SymKind::S_HEAPALLOCSITE => Self::HeapAllocSite(p.get()?),
            SymKind::S_GMANPROC | SymKind::S_LMANPROC => Self::ManagedProc(p.parse()?),
            SymKind::S_ANNOTATION => Self::Annotation(p.parse()?),
            SymKind::S_HOTPATCHFUNC => Self::HotPatchFunc(p.parse()?),
            SymKind::S_ARMSWITCHTABLE => Self::ArmSwitchTable(p.get()?),
            SymKind::S_COFFGROUP => Self::CoffGroup(p.parse()?),
            SymKind::S_SECTION => Self::Section(p.parse()?),

            _ => Self::Unknown,
        })
    }

    /// If this symbol record has a "name" field, return it. Else, `None`.
    pub fn name(&self) -> Option<&'a BStr> {
        match self {
            Self::Proc(proc) => Some(proc.name),
            Self::Data(data) => Some(data.name),
            Self::ThreadData(thread_data) => Some(thread_data.name),
            Self::Udt(udt) => Some(udt.name),
            Self::Local(local) => Some(local.name),
            Self::RefSym2(refsym) => Some(refsym.name),
            Self::Constant(c) => Some(c.name),
            Self::ManagedConstant(c) => Some(c.name),
            _ => None,
        }
    }
}
