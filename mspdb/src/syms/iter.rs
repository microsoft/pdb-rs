use super::*;
use crate::utils::iter::HasRestLen;
use std::mem::take;

/// Parses [`Sym`] records from a symbol stream.
#[derive(Clone)]
pub struct SymIter<'a> {
    data: &'a [u8],
}

impl<'a> HasRestLen for SymIter<'a> {
    fn rest_len(&self) -> usize {
        self.data.len()
    }
}

/// Parses [`SymMut`] records from a symbol stream.
///
/// This iterator allows you to modify the payload of a symbol record but not to change its length
/// or its kind.
pub struct SymIterMut<'a> {
    data: &'a mut [u8],
}

impl<'a> SymIterMut<'a> {
    /// Parses the 4-byte CodeView signature that is at the start of a module symbol stream.
    pub fn get_signature(&mut self) -> Result<[u8; 4], ParserError> {
        let mut p = ParserMut::new(take(&mut self.data));
        let sig = p.copy()?;
        self.data = p.into_rest();
        Ok(sig)
    }
}

impl<'a> HasRestLen for SymIterMut<'a> {
    fn rest_len(&self) -> usize {
        self.data.len()
    }
}

impl<'a> SymIter<'a> {
    /// Creates a new symbol iterator.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Parses the 4-byte CodeView signature that is at the start of a module symbol stream.
    pub fn get_signature(&mut self) -> Result<[u8; 4], ParserError> {
        let mut p = Parser::new(self.data);
        let sig = p.copy()?;
        self.data = p.into_rest();
        Ok(sig)
    }

    /// The remaining unparsed bytes in the symbol stream.
    pub fn rest(&self) -> &'a [u8] {
        self.data
    }

    /// Parses a single record from `data`.
    pub fn one(data: &'a [u8]) -> Option<Sym<'a>> {
        Self::new(data).next()
    }
}

impl<'a> Iterator for SymIter<'a> {
    type Item = Sym<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }

        let mut p = Parser::new(self.data);
        let record_len = p.u16().ok()?;
        if record_len < 2 {
            error!(
                "type record has invalid len {record_len}, at iter len {}",
                self.data.len()
            );
            return None;
        }

        let kind = SymKind(p.u16().ok()?);
        let record_data = p.bytes(record_len as usize - 2).ok()?;

        self.data = p.into_rest();

        Some(Sym {
            kind,
            data: record_data,
        })
    }
}

#[test]
fn test_sym_iter() {
    #[rustfmt::skip]
    let data: &[u8] = &[
        // record 0, total size = 8
        /* 0x0000 */ 6, 0,                              // size
        /* 0x0002 */ 0x4c, 0x11,                        // S_BUILDINFO
        /* 0x0004 */ 1, 2, 3, 4,                        // payload (ItemId)

        // record 1, total size = 12
        /* 0x0008 */ 10, 0,                              // size
        /* 0x000a */ 0x24, 0x11,                        // S_UNAMESPACE
        /* 0x000c */ b'b', b'o', b'o', b's',            // payload (6 bytes)
        /* 0x0010 */ b't', 0,
        /* 0x0012 */ 0xf1, 0xf2,                        // alignment padding (inside payload)

        // record 2, total size = 12
        /* 0x0014 */ 10, 0,                             // size
        /* 0x0016 */ 0x24, 0x11,                        // S_UNAMESPACE
        /* 0x0018 */ b'a', b'b', b'c', b'd',            // payload
        /* 0x001c */ b'e', b'f', b'g', 0,               // no alignment padding

        /* 0x0020 : end */
    ];

    let mut i = SymIter::new(data);

    // parse record 0
    assert_eq!(i.rest_len(), 0x20);
    let s0 = i.next().unwrap();
    assert_eq!(s0.kind, SymKind::S_BUILDINFO);
    let s0_data = s0.parse().unwrap();
    assert!(matches!(s0_data, SymData::BuildInfo(_)));

    // parse record 1
    assert_eq!(i.rest_len(), 0x18);
    let s1 = i.next().unwrap();
    assert_eq!(s1.kind, SymKind::S_UNAMESPACE);
    match s1.parse() {
        Ok(SymData::UsingNamespace(ns)) => assert_eq!(ns.namespace, "boost"),
        sd => panic!("wrong: {sd:?}"),
    }

    // parse record 2
    assert_eq!(i.rest_len(), 0xc);
    let s1 = i.next().unwrap();
    assert_eq!(s1.kind, SymKind::S_UNAMESPACE);
    match s1.parse() {
        Ok(SymData::UsingNamespace(ns)) => assert_eq!(ns.namespace, "abcdefg"),
        sd => panic!("wrong: {sd:?}"),
    }

    // end
    assert_eq!(i.rest_len(), 0);
    assert!(i.next().is_none());
}

impl<'a> SymIterMut<'a> {
    /// Creates a new symbol iterator.
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data }
    }

    /// The remaining unparsed bytes in the symbol stream.
    pub fn rest(&self) -> &[u8] {
        self.data
    }

    /// The remaining unparsed bytes in the symbol stream, with mutable access.
    pub fn rest_mut(&mut self) -> &mut [u8] {
        self.data
    }

    /// Converts this iterator into a mutable reference to the unparsed bytes in the symbol stream.
    pub fn into_rest(self) -> &'a mut [u8] {
        self.data
    }
}

impl<'a> Iterator for SymIterMut<'a> {
    type Item = SymMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() < 4 {
            return None;
        }

        // We steal self.data because it is the only way that split_at_mut() can work.
        let d = core::mem::take(&mut self.data);

        let mut p = Parser::new(d);
        let record_len = p.u16().ok()?;
        if record_len < 2 {
            error!(
                "type record has invalid len {record_len}, at iter len {}",
                self.data.len()
            );
            self.data = d;
            return None;
        }

        let kind = SymKind(p.u16().ok()?);

        let (entire_record_data, hi) = d.split_at_mut(2 + record_len as usize);
        self.data = hi;

        let record_data = &mut entire_record_data[4..];

        Some(SymMut {
            kind,
            data: record_data,
        })
    }
}

/// Scans a buffer that contains symbol records and calls `f` on each `&mut TypeIndex` value.
pub fn iter_sym_type_refs_mut<F>(syms: &mut [u8], mut f: F) -> Result<(), ParserError>
where
    F: FnMut(&mut TypeIndexLe),
{
    for sym in SymIterMut::new(syms) {
        iter_one_sym_type_refs_mut(sym.kind, sym.data, |t| f(t))?;
    }
    Ok(())
}

/// Scans the definition of a single symbol record and calls `f` on each `&mut TypeIndex` value.
pub fn iter_one_sym_type_refs_mut<F>(
    sym_kind: SymKind,
    sym_payload: &mut [u8],
    mut f: F,
) -> Result<(), ParserError>
where
    F: FnMut(&mut TypeIndexLe),
{
    let mut p = ParserMut::new(sym_payload);

    match sym_kind {
        // These symbol types do not contain TypeIndex.
        | SymKind::S_ANNOTATION
        | SymKind::S_ANNOTATIONREF
        | SymKind::S_ARMSWITCHTABLE
        | SymKind::S_BLOCK32
        | SymKind::S_BUILDINFO
        | SymKind::S_COMPILE2
        | SymKind::S_COMPILE3
        | SymKind::S_DEFRANGE_CONSTVAL_FULL_SCOPE
        | SymKind::S_DEFRANGE_FRAMEPOINTER_REL
        | SymKind::S_DEFRANGE_FRAMEPOINTER_REL_FULL_SCOPE
        | SymKind::S_DEFRANGE_GLOBALSYM_FULL_SCOPE
        | SymKind::S_DEFRANGE_REGISTER
        | SymKind::S_DEFRANGE_REGISTER_REL
        | SymKind::S_DEFRANGE_SUBFIELD_REGISTER
        | SymKind::S_END
        | SymKind::S_ENVBLOCK
        | SymKind::S_FRAMECOOKIE
        | SymKind::S_FRAMEPROC
        | SymKind::S_GMANPROC
        | SymKind::S_INLINESITE
        | SymKind::S_LABEL32
        | SymKind::S_LMANPROC
        | SymKind::S_LPROCREF
        | SymKind::S_OBJNAME
        | SymKind::S_PROCREF
        | SymKind::S_PUB32
        | SymKind::S_PUB32_ST
        | SymKind::S_SEPCODE
        | SymKind::S_THUNK32
        | SymKind::S_UNAMESPACE
        // keep
        => {}


        // TODO: These are undocumented. We don't know if they contain TypeIndex.
        | SymKind::S_COFFGROUP
        | SymKind::S_EXPORT
        | SymKind::S_INLINESITE_END
        | SymKind::S_SECTION
        | SymKind::S_POGODATA
        | SymKind::S_INLINESITE2
        // keep
        => {}

        // FUNCTIONLIST: used for S_CALLERS, S_CALLEES, S_INLINEES
        | SymKind::S_CALLEES
        | SymKind::S_CALLERS
        | SymKind::S_INLINEES => {
            // These symbols use FUNCTIONLIST, which contains ItemId, not TypeIndex.
        }

        SymKind::S_CALLSITEINFO => {
            p.u32()?; // offset of call site
            p.u16()?; // section index
            p.u16()?; // padding
            f(p.get_mut()?);
        }

        SymKind::S_HEAPALLOCSITE => {
            p.u32()?; // offset of call site
            p.u16()?; // section index of call site
            p.u16()?; // length of heap allocation call instruction
            f(p.get_mut()?); // type index describing function signature
        }

        SymKind::S_FILESTATIC => {
            f(p.get_mut()?);
        }

        // These symbols have two TypeIndex fields, at offsets 0 and 4.
        SymKind::S_VFTABLE32
        | SymKind::S_LTHREAD32
        | SymKind::S_GTHREAD32
        | SymKind::S_LTHREAD32_ST
        | SymKind::S_GTHREAD32_ST => {
            f(p.get_mut()?);
            f(p.get_mut()?);
        }

        // These symbols have a single type index, at offset 4.
        SymKind::S_BPREL32 | SymKind::S_REGREL32 => {
            p.u32()?;
            f(p.get_mut()?);
        }

        SymKind::S_LPROC32
        | SymKind::S_GPROC32
        | SymKind::S_LPROC32_ST
        | SymKind::S_GPROC32_ST => {
            let proc = p.get_mut::<ProcFixed>()?;
            f(&mut proc.proc_type);
        }

        // These symbols have a single type index, at offset 0.
        | SymKind::S_UDT
        | SymKind::S_LOCAL
        | SymKind::S_CONSTANT
        | SymKind::S_GDATA32
        | SymKind::S_LDATA32
        | SymKind::S_GDATA32_ST
        | SymKind::S_LDATA32_ST
        | SymKind::S_REGISTER => {
            f(p.get_mut()?);
        }

        SymKind::S_TRAMPOLINE => {
            // TODO
        }

        SymKind::S_SKIP => {
            // you got it, bud
        }

        SymKind::S_COMPILE => {}

        SymKind::S_FRAMEREG => {
            // TODO
        }

        // TODO
        SymKind::S_GMANDATA |
        SymKind::S_LMANDATA |
        SymKind::S_MANSLOT |
        SymKind::S_TOKENREF |
        SymKind::S_MANCONSTANT |
        SymKind::S_OEM
        => {}

        unknown => {
            error!("unknown symbol kind: {unknown:?}");
        }
    }

    Ok(())
}
