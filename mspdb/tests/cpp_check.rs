//! This integration test runs the MSVC compiler and linker to generate complete executables and
//! and PDBs, and then reads the PDBs and verifies that they contain the expected information.

#![cfg(windows)]
#![allow(clippy::single_match)]
#![allow(clippy::useless_vec)]

use bstr::BStr;
use mspdb::syms::{Data, SymData, SymKind, Udt};
use mspdb::types::fields::Field;
use mspdb::types::{TypeData, TypeIndex};
use mspdb::Pdb;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use tracing::{error, info, trace};

const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
const CARGO_TARGET_TMPDIR: &str = env!("CARGO_TARGET_TMPDIR");

#[static_init::dynamic]
static INIT_LOGGER: () = {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_test_writer()
        .with_file(true)
        .with_line_number(true)
        .with_max_level(tracing::Level::DEBUG)
        .compact()
        .without_time()
        .finish();
};

// id should be the base name of a C++ source file in the "cpp_check" directory.
// e.g. id == `types`
fn run_test(id: &str) -> Box<Pdb> {
    info!("case: {id}");

    let cargo_manifest_dir = Path::new(CARGO_MANIFEST_DIR);
    let cargo_target_tmpdir = Path::new(CARGO_TARGET_TMPDIR);
    let cases_tmpdir = cargo_target_tmpdir.join("cases");

    let cases_dir = Path::new(cargo_manifest_dir)
        .join("tests")
        .join("cpp_check");
    let source_file_name = format!("{id}.cpp");
    let source_file_path = cases_dir.join(&source_file_name);

    let this_output_dir = cases_tmpdir.join(id);

    info!("source file: {}", source_file_name);
    info!("output dir: {}", this_output_dir.display());

    let dll_file_name = format!("{id}.dll");
    let pdb_file_name = format!("{id}.pdb");

    let dll_path = this_output_dir.join(dll_file_name);
    let pdb_path = this_output_dir.join(pdb_file_name);

    info!("target: {}", dll_path.display());
    info!("pdb:    {}", pdb_path.display());

    std::fs::create_dir_all(&this_output_dir).unwrap();

    let obj_file_path = this_output_dir.join(format!("{id}.obj"));

    {
        let mut cmd = Command::new("cl.exe");
        cmd.current_dir(&this_output_dir);
        cmd.arg("/nologo");
        cmd.arg("/Z7");
        cmd.arg("/c");
        cmd.arg("/O2");
        cmd.arg(source_file_path);
        cmd.arg(format!(
            "/Fo{}",
            obj_file_path.as_os_str().to_string_lossy()
        ));

        let mut child = cmd.spawn().unwrap();
        let child_exit = child.wait().unwrap();
        assert!(child_exit.success(), "cl.exe failed");
    }

    {
        let mut cmd = Command::new("link.exe");
        cmd.current_dir(&this_output_dir);
        cmd.arg("/nologo");
        cmd.arg("/dll");
        cmd.arg("/debug:full");
        cmd.arg(format!("/out:{}", dll_path.display()));
        cmd.arg(format!("/pdb:{}", pdb_path.display()));
        cmd.arg(format!("{}", obj_file_path.as_os_str().to_string_lossy()));

        let mut child = cmd.spawn().unwrap();
        let child_exit = child.wait().unwrap();
        assert!(child_exit.success(), "link.exe failed");
    }

    mspdb::Pdb::open(&pdb_path).unwrap()
}

// TODO: Re-enable this in OneBranch pipelines. It needs VS build tools for this to work.
// #[ignore]
#[test]
fn types() -> anyhow::Result<()> {
    let pdb = run_test("types");

    let gss = pdb.read_gss()?;
    let gsi = pdb.read_gsi()?;

    let get_global = |kind: SymKind, name: &str| -> SymData {
        let sym = gsi
            .find_symbol(&gss, name.into())
            .expect("expected find_symbol to succeed")
            .unwrap_or_else(|| panic!("expected find_symbol to succeed: {name}"));
        assert_eq!(sym.kind, kind, "expected symbol kind {kind:?} : {name}");
        let sym_data = sym
            .parse()
            .unwrap_or_else(|e| panic!("expected symbol parse to succeed: {name} : {e:?}"));
        sym_data
    };

    let get_global_udt = |name: &str| -> Udt {
        match get_global(SymKind::S_UDT, name) {
            SymData::Udt(udt) => udt,
            unknown => panic!("expected UDT for {name}, got: {unknown:?}"),
        }
    };

    let get_global_data = |kind: SymKind, name: &str| -> Data {
        match get_global(kind, name) {
            SymData::Data(d) => d,
            unknown => panic!("expected SymData::Data for {name}, got: {unknown:?}"),
        }
    };

    let names = vec![
        "__acrt_initial_locale_pointers",
        "__xi_a",
        "get_initial_environment",
        "FEOFLAG",
        "StructWithManyEnums",
        "enums_export",
    ];

    // Dump some stuff for fun.
    for name in names.iter() {
        let s = gsi.find_symbol(&gss, BStr::new(name)).unwrap();
        info!("{:?} --> {:?}", name, s);
    }

    let enums_export = gsi.find_symbol(&gss, BStr::new("enums_export"))?.unwrap();
    assert_eq!(enums_export.kind, SymKind::S_PROCREF);

    let type_stream = pdb.read_type_stream()?;

    // Check that primitive types match the values we're expecting.
    {
        let primitives_data = get_global_data(SymKind::S_GDATA32, "g_structWithPrimitiveTypes");
        let primitives_ty_record = type_stream.record(primitives_data.header.type_.get())?;
        let primitives_ty_struct = match primitives_ty_record.parse()? {
            TypeData::Struct(s) => s,
            unknown => panic!("Expected StructWithPrimitiveTypes to be a struct: {unknown:?}"),
        };

        // Index the member (data) fields by name
        let mut fields: HashMap<&BStr, TypeIndex> = HashMap::new();
        for f in type_stream.iter_fields(primitives_ty_struct.fixed.field_list.get()) {
            match f {
                Field::Member(m) => {
                    // Turn this on when adding new fields in types.cpp
                    if false {
                        if m.ty.0 < 0x1000 {
                            println!("  (TypeIndex::{:?}, \"{}\"),", m.ty, m.name);
                        } else {
                            println!("  // non-primitive field: {}", m.name);
                        }
                    }
                    fields.insert(m.name, m.ty);
                }
                _ => {}
            }
        }

        // Validate our expectations
        //
        // TODO: This will fail if the C++ code is compiled for a 32-bit architecture because the
        // pointer types encode the size of the pointer.
        let expectations: &[(TypeIndex, &str)] = &[
            (TypeIndex::T_RCHAR, "f_char"),
            (TypeIndex::T_RCHAR, "f_const_char"),
            (TypeIndex::T_CHAR, "f_signed_char"),
            (TypeIndex::T_UCHAR, "f_unsigned_char"),
            (TypeIndex::T_64PRCHAR, "f_char_ptr"),
            // non-primitive field: f_const_char_ptr
            (TypeIndex::T_64PRCHAR, "f_char_const_ptr"),
            // non-primitive field: f_const_char_const_ptr
            (TypeIndex::T_INT4, "f_int"),
            (TypeIndex::T_INT4, "f_const_int"),
            (TypeIndex::T_INT4, "f_signed_int"),
            (TypeIndex::T_UINT4, "f_unsigned_int"),
            (TypeIndex::T_64PINT4, "f_int_ptr"),
            // non-primitive field: f_const_int_ptr
            (TypeIndex::T_64PINT4, "f_int_const_ptr"),
            // non-primitive field: f_const_int_const_ptr
            (TypeIndex::T_LONG, "f_long"),
            (TypeIndex::T_LONG, "f_const_long"),
            (TypeIndex::T_LONG, "f_signed_long"),
            (TypeIndex::T_ULONG, "f_unsigned_long"),
            (TypeIndex::T_64PLONG, "f_long_ptr"),
            // non-primitive field: f_const_long_ptr
            (TypeIndex::T_64PLONG, "f_long_const_ptr"),
            // non-primitive field: f_const_long_const_ptr
            (TypeIndex::T_SHORT, "f_short"),
            (TypeIndex::T_SHORT, "f_const_short"),
            (TypeIndex::T_SHORT, "f_signed_short"),
            (TypeIndex::T_USHORT, "f_unsigned_short"),
            (TypeIndex::T_64PSHORT, "f_short_ptr"),
            // non-primitive field: f_const_short_ptr
            (TypeIndex::T_64PSHORT, "f_short_const_ptr"),
            // non-primitive field: f_const_short_const_ptr
            (TypeIndex::T_QUAD, "f__long_long"),
            (TypeIndex::T_QUAD, "f_const__long_long"),
            (TypeIndex::T_QUAD, "f_signed__long_long"),
            (TypeIndex::T_UQUAD, "f_unsigned__long_long"),
            (TypeIndex::T_64PQUAD, "f__long_long_ptr"),
            // non-primitive field: f_const__long_long_ptr
            (TypeIndex::T_64PQUAD, "f__long_long_const_ptr"),
            // non-primitive field: f_const__long_long_const_ptr
            (TypeIndex::T_RCHAR, "f_int8"),
            (TypeIndex::T_RCHAR, "f_const_int8"),
            (TypeIndex::T_CHAR, "f_signed_int8"),
            (TypeIndex::T_UCHAR, "f_unsigned_int8"),
            (TypeIndex::T_64PRCHAR, "f_int8_ptr"),
            // non-primitive field: f_const_int8_ptr
            (TypeIndex::T_64PRCHAR, "f_int8_const_ptr"),
            // non-primitive field: f_const_int8_const_ptr
            (TypeIndex::T_SHORT, "f_int16"),
            (TypeIndex::T_SHORT, "f_const_int16"),
            (TypeIndex::T_SHORT, "f_signed_int16"),
            (TypeIndex::T_USHORT, "f_unsigned_int16"),
            (TypeIndex::T_64PSHORT, "f_int16_ptr"),
            // non-primitive field: f_const_int16_ptr
            (TypeIndex::T_64PSHORT, "f_int16_const_ptr"),
            // non-primitive field: f_const_int16_const_ptr
            (TypeIndex::T_INT4, "f_int32"),
            (TypeIndex::T_INT4, "f_const_int32"),
            (TypeIndex::T_INT4, "f_signed_int32"),
            (TypeIndex::T_UINT4, "f_unsigned_int32"),
            (TypeIndex::T_64PINT4, "f_int32_ptr"),
            // non-primitive field: f_const_int32_ptr
            (TypeIndex::T_64PINT4, "f_int32_const_ptr"),
            // non-primitive field: f_const_int32_const_ptr
            (TypeIndex::T_QUAD, "f_int64"),
            (TypeIndex::T_QUAD, "f_const_int64"),
            (TypeIndex::T_QUAD, "f_signed_int64"),
            (TypeIndex::T_UQUAD, "f_unsigned_int64"),
            (TypeIndex::T_64PQUAD, "f_int64_ptr"),
            // non-primitive field: f_const_int64_ptr
            (TypeIndex::T_64PQUAD, "f_int64_const_ptr"),
            // non-primitive field: f_const_int64_const_ptr
            (TypeIndex::T_BOOL8, "f_bool"),
            (TypeIndex::T_64PBOOL08, "f_bool_ptr"),
            (TypeIndex::T_64PVOID, "f_void_ptr"),
            (TypeIndex::T_BOOL8, "f_bool"),
            (TypeIndex::T_64PBOOL08, "f_bool_ptr"),
            (TypeIndex::T_REAL32, "f_float"),
            (TypeIndex::T_64PREAL32, "f_float_ptr"),
            (TypeIndex::T_REAL64, "f_double"),
            (TypeIndex::T_64PREAL64, "f_double_ptr"),
        ];

        let mut error = false;
        for &(expected_type, name) in expectations.iter() {
            if let Some(&actual_type) = fields.get(BStr::new(name)) {
                if expected_type == actual_type {
                    trace!("field has correct type: {expected_type:?} - {name}");
                } else {
                    error!("expected field {name} to have type {expected_type:?}, but it had type {actual_type:?}");
                    error = true;
                }
            } else {
                error!("did not find field: {name}");
                error = true;
            }
        }

        assert!(!error, "Found one or more fields with the wrong type");
    }

    // Find an S_LPROCREF symbol.
    {
        let _gf = match get_global(SymKind::S_PROCREF, "global_function") {
            SymData::RefSym2(r) => r,
            unknown => panic!("wrong symbol data for global_function: {unknown:?}"),
        };
        // TODO: look up the actual S_GDATA32 symbol and check things about it
    }

    // Find an S_CONSTANT symbol at global scope.
    {
        let c = match get_global(SymKind::S_CONSTANT, "WHAT_IS_SIX_TIMES_SEVEN") {
            SymData::Constant(c) => c,
            unknown => panic!("expected S_CONSTANT, got: {unknown:?}"),
        };
        let value: i32 = c.value.try_into().unwrap();
        assert_eq!(value, 42);
    }

    // Find an S_CONSTANT symbol that is nested within a class.
    {
        let c = match get_global(SymKind::S_CONSTANT, "Zebra::NUMBER_OF_STRIPES") {
            SymData::Constant(c) => c,
            unknown => panic!("expected S_CONSTANT, got: {unknown:?}"),
        };
        let value: i32 = c.value.try_into().unwrap();
        assert_eq!(value, 80);
    }

    // Find an S_CONSTANT symbol that is within nested C++ namespaces.
    {
        let c = match get_global(SymKind::S_CONSTANT, "foo::bar::CONSTANT_INSIDE_NAMESPACE") {
            SymData::Constant(c) => c,
            unknown => panic!("expected S_CONSTANT, got: {unknown:?}"),
        };
        let value: i32 = c.value.try_into().unwrap();
        assert_eq!(value, -333);
    }

    // TODO: It appears MSVC does not emit an S_UDT for enums, even when they are used. Why?
    // The LF_ENUM record exists, but there's no S_UDT for it.
    if false {
        let udt = get_global_udt("EnumSimple");
        let e = match type_stream.record(udt.type_)?.parse()? {
            TypeData::Enum(e) => e,
            unknown => panic!("expected LF_ENUM, got: {unknown:?}"),
        };

        let mut values = HashMap::new();
        for f in type_stream.iter_fields(e.fixed.fields.get()) {
            match f {
                Field::Enumerate(f) => {
                    values.insert(f.name, f.value);
                }
                _ => {}
            }
        }
        assert_eq!(
            u32::try_from(*values.get(BStr::new("Simple_A")).unwrap()).unwrap(),
            100u32
        );
    }

    Ok(())
}
