//! Algorithm for traversing ("visiting") the dependency graph within a type stream or between a
//! symbol stream and a type stream.

use super::{ItemId, ItemIdLe};
use crate::parser::{Parser, ParserError, ParserMut};
use crate::types::{introduces_virtual, PointerFlags};
use crate::types::{Leaf, TypeIndex, TypeIndexLe};
use anyhow::Context;
use std::mem::replace;
use tracing::error;
use zerocopy::{LE, U32};

/// Defines the functions needed for generically visiting type indexes within a type record or a
/// symbol record.
///
/// This trait exists in order to allow a single generic function to handle visiting the `TypeIndex`
/// values within a buffer, generic over the mutability of the access.
pub trait RecordVisitor {
    /// True if the parser is empty
    fn is_empty(&self) -> bool;

    /// Provides access to the rest of the record
    fn peek_rest(&self) -> &[u8];

    /// Parses a `u16` value
    fn u16(&mut self) -> Result<u16, ParserError>;

    /// Parses a `u32` value
    fn u32(&mut self) -> Result<u32, ParserError>;

    /// Skips `n` bytes of input
    fn skip(&mut self, n: usize) -> Result<(), ParserError>;

    /// Parses the next `ItemId` value and visits the location or value.
    fn item(&mut self) -> Result<(), ParserError>;

    /// Parses the next `TypeIndex` value and visits the location or value.
    fn ty(&mut self) -> Result<(), ParserError>;

    /// Parses a `NameIndex` value and visits the location or value.
    fn name_index(&mut self) -> Result<(), ParserError>;

    /// Parses a `Number`.
    fn number(&mut self) -> Result<(), ParserError>;

    /// Parses a NUL-terminated string.
    fn strz(&mut self) -> Result<(), ParserError>;
}

/// Defines a visitor that visits every ItemId and TypeIndexLe in a record. Allows modification.
#[allow(missing_docs)]
pub trait IndexVisitorMut {
    #[allow(unused_variables)]
    fn type_index(&mut self, offset: usize, value: &mut TypeIndexLe) -> Result<(), ParserError> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn item_id(&mut self, offset: usize, value: &mut ItemIdLe) -> Result<(), ParserError> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn name_index(&mut self, offset: usize, value: &mut U32<LE>) -> Result<(), ParserError> {
        Ok(())
    }
}

/// Defines a visitor that visits every ItemId and TypeIndexLe in a record.
#[allow(missing_docs)]
pub trait IndexVisitor {
    #[allow(unused_variables)]
    fn type_index(&mut self, offset: usize, value: TypeIndex) -> Result<(), ParserError> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn item_id(&mut self, offset: usize, value: ItemId) -> Result<(), ParserError> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn name_index(&mut self, offset: usize, value: u32) -> Result<(), ParserError> {
        Ok(())
    }
}

struct RefVisitor<'a, IV: IndexVisitor> {
    parser: Parser<'a>,
    original_len: usize,
    index_visitor: IV,
}

impl<'a, IV: IndexVisitor> RecordVisitor for RefVisitor<'a, IV> {
    fn is_empty(&self) -> bool {
        self.parser.is_empty()
    }

    fn peek_rest(&self) -> &[u8] {
        self.parser.peek_rest()
    }

    fn u16(&mut self) -> Result<u16, ParserError> {
        self.parser.u16()
    }

    fn u32(&mut self) -> Result<u32, ParserError> {
        self.parser.u32()
    }

    fn skip(&mut self, n: usize) -> Result<(), ParserError> {
        self.parser.skip(n)
    }

    fn ty(&mut self) -> Result<(), ParserError> {
        let offset = self.original_len - self.parser.len();
        let ti = self.parser.type_index()?;
        self.index_visitor.type_index(offset, ti)?;
        Ok(())
    }

    fn item(&mut self) -> Result<(), ParserError> {
        let offset = self.original_len - self.parser.len();
        let ii = self.parser.u32()?;
        self.index_visitor.item_id(offset, ii)?;
        Ok(())
    }

    fn number(&mut self) -> Result<(), ParserError> {
        self.parser.number()?;
        Ok(())
    }

    fn strz(&mut self) -> Result<(), ParserError> {
        self.parser.skip_strz()
    }

    fn name_index(&mut self) -> Result<(), ParserError> {
        let offset = self.original_len - self.parser.len();
        let ni = self.parser.u32()?;
        self.index_visitor.name_index(offset, ni)?;
        Ok(())
    }
}

struct MutVisitor<'a, IV: IndexVisitorMut> {
    parser: ParserMut<'a>,
    original_len: usize,
    index_visitor: IV,
}

impl<'a, IV: IndexVisitorMut> RecordVisitor for MutVisitor<'a, IV> {
    fn is_empty(&self) -> bool {
        self.parser.is_empty()
    }

    fn peek_rest(&self) -> &[u8] {
        self.parser.peek_rest()
    }

    fn u16(&mut self) -> Result<u16, ParserError> {
        self.parser.u16()
    }

    fn u32(&mut self) -> Result<u32, ParserError> {
        self.parser.u32()
    }

    fn skip(&mut self, n: usize) -> Result<(), ParserError> {
        self.parser.skip(n)
    }

    fn ty(&mut self) -> Result<(), ParserError> {
        let offset = self.original_len - self.parser.len();
        let ti: &mut TypeIndexLe = self.parser.get_mut()?;
        self.index_visitor.type_index(offset, ti)?;
        Ok(())
    }

    fn item(&mut self) -> Result<(), ParserError> {
        let offset = self.original_len - self.parser.len();
        let ii: &mut ItemIdLe = self.parser.get_mut()?;
        self.index_visitor.item_id(offset, ii)?;
        Ok(())
    }

    fn number(&mut self) -> Result<(), ParserError> {
        self.parser.skip_number()
    }

    fn strz(&mut self) -> Result<(), ParserError> {
        self.parser.skip_strz()
    }

    fn name_index(&mut self) -> Result<(), ParserError> {
        let offset = self.original_len - self.parser.len();
        let ni: &mut U32<LE> = self.parser.get_mut()?;
        self.index_visitor.name_index(offset, ni)?;
        Ok(())
    }
}

/// Scans the type indexes within a type record and calls `f` for each type index. This function
/// can only read data.
#[inline(never)]
pub fn visit_type_indexes_in_record_slice<IV: IndexVisitor>(
    type_kind: Leaf,
    record_data: &[u8],
    index_visitor: IV,
) -> Result<(), anyhow::Error> {
    let record_data_len = record_data.len();

    let mut v = RefVisitor {
        original_len: record_data.len(),
        parser: Parser::new(record_data),
        index_visitor,
    };

    visit_type_indexes_in_record(type_kind, &mut v).with_context(|| {
        let offset = record_data_len - v.parser.len();
        format!(
            "at byte offset 0x{:x} {} within type record",
            offset, offset
        )
    })
}

/// Scans the type indexes within a type record and calls `f` for each type index. This function
/// can modify the type indexes within the record.
#[inline(never)]
pub fn visit_type_indexes_in_record_slice_mut<IV>(
    type_kind: Leaf,
    record_data: &mut [u8],
    index_visitor: IV,
) -> Result<(), anyhow::Error>
where
    IV: IndexVisitorMut,
{
    let record_data_len = record_data.len();

    let mut v = MutVisitor {
        original_len: record_data.len(),
        parser: ParserMut::new(record_data),
        index_visitor,
    };

    visit_type_indexes_in_record(type_kind, &mut v).with_context(|| {
        let offset = record_data_len - v.parser.len();
        format!(
            "at byte offset 0x{:x} {} within type record",
            offset, offset
        )
    })
}

/// This function examines a type record and traverses the type indexes within it.
///
/// The caller provides an implementation of the visitor trait.  The visitor trait provides the
/// ability to read fields from the type record, and receives notifications that the visitor is
/// positioned on a type index.
pub fn visit_type_indexes_in_record<V: RecordVisitor>(
    type_kind: Leaf,
    p: &mut V,
) -> Result<(), ParserError> {
    match type_kind {
        Leaf::LF_LABEL => {}

        Leaf::LF_MODIFIER => {
            p.ty()?;
        }

        Leaf::LF_POINTER => {
            p.ty()?; // underlying type
            let attr = PointerFlags(p.u32()?);
            match attr.mode() {
                2 => {
                    // 2 is pointer to data member
                    p.ty()?;
                }
                3 => {
                    // 3 is pointer to method
                    p.ty()?;
                }
                _ => {}
            }
        }

        Leaf::LF_ALIAS => {
            p.ty()?;
        }

        Leaf::LF_ARRAY => {
            p.ty()?; // element type
            p.ty()?; // index type
        }

        Leaf::LF_CLASS | Leaf::LF_STRUCTURE => {
            p.u16()?; // count
            p.skip(2)?; // property
            p.ty()?; // field list
            p.ty()?; // derivation list
            p.ty()?; // vtable shape
        }

        Leaf::LF_ENUM => {
            p.u16()?; // count
            p.skip(2)?; // property
            p.ty()?; // type
            p.ty()?; // field list
        }

        Leaf::LF_UNION => {
            p.u16()?; // count
            p.skip(2)?; // property
            p.ty()?; // field list
        }

        Leaf::LF_PROCEDURE => {
            p.ty()?; // return type
            p.skip(4)?; // calling convention, reserved, and num params
            p.ty()?; // arg list
        }

        Leaf::LF_MFUNCTION => {
            p.ty()?; // return type
            p.ty()?; // class definition
            p.ty()?; // this type
            p.skip(4)?; // calling convention, reserved, and num params
            p.ty()?; // arg list
        }

        Leaf::LF_ARGLIST => {
            let num_args = p.u32()?;
            for _ in 0..num_args {
                p.ty()?;
            }
        }

        Leaf::LF_FIELDLIST => {
            let mut prev_item_kind = None;
            loop {
                let rest = p.peek_rest();
                if rest.is_empty() {
                    break;
                }

                // Check for padding (alignment) bytes.
                let mut padding_len = 0;
                while padding_len < rest.len() && rest[padding_len] >= 0xf0 {
                    padding_len += 1;
                }
                if padding_len > 0 {
                    p.skip(padding_len)?;
                }

                if p.is_empty() {
                    break;
                }

                let item_kind = Leaf(p.u16()?);
                let after = replace(&mut prev_item_kind, Some(item_kind));

                match item_kind {
                    Leaf::LF_BCLASS => {
                        let _attr = p.u16()?;
                        p.ty()?; // class type
                        p.number()?; // offset
                    }

                    Leaf::LF_VBCLASS => {
                        let _attr = p.u16()?;
                        p.ty()?; // base class
                        p.ty()?; // vbtype
                        p.number()?; // vbpoff
                        p.number()?; // vbpff
                    }

                    Leaf::LF_IVBCLASS => {
                        let _attr = p.u16()?;
                        p.ty()?; // base class
                        p.ty()?; // virtual base type
                        p.number()?; // vbpoff
                        p.number()?; // vbpff
                    }

                    Leaf::LF_ENUMERATE => {
                        // nothing needed
                        let _attr = p.u16()?;
                        p.number()?; // value
                        p.strz()?; // name
                    }

                    Leaf::LF_FRIENDFCN => {
                        p.skip(2)?; // padding
                        p.ty()?; // type
                        p.strz()?; // name
                    }

                    Leaf::LF_INDEX => {
                        p.skip(2)?; // padding
                        p.ty()?; // index
                    }

                    Leaf::LF_MEMBER => {
                        let _attr = p.u16()?;
                        p.ty()?; // type
                        p.number()?; // offset
                        p.strz()?; // name
                    }

                    Leaf::LF_STMEMBER => {
                        let _attr = p.u16()?;
                        p.ty()?; // type
                        p.strz()?; // name
                    }

                    Leaf::LF_METHOD => {
                        let _count = p.u16()?;
                        p.ty()?; // method list
                        p.strz()?; // name
                    }

                    Leaf::LF_NESTEDTYPE => {
                        p.skip(2)?; // padding
                        p.ty()?; // index
                        p.strz()?; // name
                    }

                    Leaf::LF_VFUNCTAB => {
                        p.skip(2)?; // padding
                        p.ty()?; // vtable type
                    }

                    Leaf::LF_FRIENDCLS => {
                        p.skip(2)?; // padding
                        p.ty()?; // friend class type
                    }

                    Leaf::LF_ONEMETHOD => {
                        let attr = p.u16()?; // attribute
                        p.ty()?; // type of method
                        if introduces_virtual(attr) {
                            p.u32()?; // vbaseoff
                        }
                        p.strz()?; // name
                    }

                    Leaf::LF_VFUNCOFF => {
                        p.u16()?; // padding
                        p.ty()?; // vtable type
                        p.u32()?; // offset
                    }

                    Leaf::LF_NESTEDTYPEEX => {
                        p.u16()?; // attribute
                        p.ty()?; // nested type
                        p.strz()?; // name
                    }

                    unknown_item_kind => {
                        error!(
                            ?unknown_item_kind,
                            ?after,
                            "unrecognized item within LF_FIELDLIST"
                        );
                        break;
                    }
                }
            }
        }

        Leaf::LF_DERIVED => {
            let count = p.u32()?;
            for _ in 0..count {
                p.ty()?;
            }
        }

        Leaf::LF_BITFIELD => {
            p.ty()?;
        }

        Leaf::LF_METHODLIST => {
            while !p.is_empty() {
                let attr = p.u16()?;
                p.skip(2)?; // padding
                p.ty()?;
                if introduces_virtual(attr) {
                    p.skip(4)?; // vtable offset
                }
            }
        }

        Leaf::LF_DIMCONU => {
            p.ty()?; // index type
        }

        Leaf::LF_DIMCONLU => {
            p.ty()?; // index type
        }

        Leaf::LF_DIMVARU => {
            let rank = p.u32()?;
            p.ty()?; // index type
            for _ in 0..rank {
                p.ty()?; // upper bound for this dimension
            }
        }

        // These types do not contain any pointers to other types.
        Leaf::LF_VTSHAPE | Leaf::LF_PRECOMP | Leaf::LF_ENDPRECOMP | Leaf::LF_SKIP => {}

        Leaf::LF_VFTPATH => {
            let count = p.u32()?;
            for _ in 0..count {
                p.ty()?;
            }
        }

        Leaf::LF_VFTABLE => {
            p.ty()?; // type
            p.ty()?; // base_vftable
        }

        Leaf::LF_CLASS2 | Leaf::LF_STRUCTURE2 | Leaf::LF_UNION2 | Leaf::LF_INTERFACE2 => {
            p.skip(4)?; // property
            p.ty()?; // field
            p.ty()?; // derived
            p.ty()?; // vshape
        }

        Leaf::LF_FUNC_ID => {
            p.item()?; // parent scope of the ID, 0 if global
            p.ty()?; // function type
        }

        Leaf::LF_MFUNC_ID => {
            p.ty()?; // parent type
            p.ty()?; // function type
        }

        Leaf::LF_BUILDINFO => {
            let n = p.u16()?;
            for _ in 0..n {
                p.item()?;
            }
        }

        Leaf::LF_SUBSTR_LIST => {
            let count = p.u32()?;
            for _ in 0..count {
                p.item()?;
            }
        }

        Leaf::LF_STRING_ID => {
            p.item()?; // ID to list of sub string IDs
        }

        Leaf::LF_UDT_SRC_LINE => {
            p.ty()?;
            p.name_index()?; // NameIndex of source file name
        }

        Leaf::LF_UDT_MOD_SRC_LINE => {
            p.ty()?;
            p.name_index()?; // NameIndex of source file name
        }

        _ => {
            error!("unrecognized type kind: {:?}", type_kind);
            return Err(ParserError::new());
        }
    }

    Ok(())
}
