use anyhow::Result;
use mspdb::msf::offset_within_page;
use mspdb::syms::{SymIter, SymKind};
use mspdb::tpi::TypeStreamHeader;
use mspdb::types::{Leaf, TypesIter};
use mspdb::{Pdb, Stream};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::mem::size_of;
use zerocopy::{AsBytes, FromZeroes};

/// Counts the number of records and record sizes for a given set of PDBs.
#[derive(clap::Parser)]
pub struct CountsOptions {
    /// The set of PDBs to read.
    #[command(flatten)]
    pdbs: crate::glob_pdbs::PdbList,

    /// Count type records in the Global Symbol Stream.
    #[arg(long)]
    global_symbols: bool,

    /// Count symbol records in the TPI Stream.
    #[arg(long)]
    tpi: bool,

    /// Count symbol records in the IPI Stream.
    #[arg(long)]
    ipi: bool,

    /// Count symbol records in each module symbol stream.
    #[arg(long)]
    module_symbols: bool,
}

#[derive(Default)]
struct Counts {
    ipi: TypeStreamCounts,
    tpi: TypeStreamCounts,
    module_sym_counts: HashMap<SymKind, PerRecord>,

    module_sym_sizes: Vec<(u64, u32)>, // (byte_size, module_index)
    global_syms_counts: HashMap<SymKind, PerRecord>,

    num_pdbs_failed: u32,

    sc: StreamCounts,
}

#[derive(Default)]
struct StreamCounts {
    total_file_size: u64,

    tpi: u64,
    tpi_hash: u64,
    ipi: u64,
    ipi_hash: u64,
    gsi: u64,
    psi: u64,

    named: BTreeMap<String, u64>,

    dbi: u64,
    dbi_contribs: u64,
    dbi_modules: u64,
    dbi_sources: u64,

    pdbi: u64,

    /// Size in bytes of all module streams combined
    modules: u64,

    modules_c13_lines: u64,
    modules_syms: u64,

    /// Size in bytes of GSS
    gss: u64,

    old_stream_dir: u64,

    /// Fragmentation in the last page of streams.
    stream_frag: u64,

    /// Number of bytes in pages that are free.
    free_pages_bytes: u64,
}

#[derive(Default)]
struct TypeStreamCounts {
    records: HashMap<Leaf, PerRecord>,
}

pub fn counts_command(options: CountsOptions) -> Result<()> {
    let mut counts = Counts::default();

    for file_name in options.pdbs.get_paths()? {
        match mspdb::Pdb::open(&file_name) {
            Ok(pdb) => match count_one_pdb(&options, &pdb, &mut counts) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!(
                        "Error occurred while processing PDB: {}\n  {}",
                        file_name.display(),
                        e
                    );
                    counts.num_pdbs_failed += 1;
                }
            },
            Err(e) => {
                eprintln!("Failed to open {} : {}", file_name.display(), e);
                counts.num_pdbs_failed += 1;
            }
        }
    }

    show_counts(&mut counts);

    Ok(())
}

// Count records in global symbol stream
fn count_global_symbols(pdb: &Pdb, counts: &mut Counts) -> anyhow::Result<()> {
    let global_syms_stream = pdb.dbi_header().sym_record_stream()?;
    let global_syms_stream_data = pdb.read_stream_to_vec(global_syms_stream)?;
    count_sym_records(&global_syms_stream_data, &mut counts.global_syms_counts);
    Ok(())
}

fn count_one_pdb(options: &CountsOptions, pdb: &Pdb, counts: &mut Counts) -> Result<()> {
    if options.tpi {
        read_and_count_type_records(pdb, &mut counts.tpi, Stream::TPI)?;
    }
    if options.ipi {
        read_and_count_type_records(pdb, &mut counts.ipi, Stream::IPI)?;
    }

    if let Some(msf) = pdb.msf() {
        // TODO: Do something smarter for PDZ.
        counts.sc.total_file_size += msf.nominal_size();
    }

    let modules_substream = pdb.read_modules()?;
    for (module_index, module) in modules_substream.iter().enumerate() {
        if let Some(module_stream) = module.stream() {
            if options.module_symbols {
                let module_sym_size: u64;
                if let Some(modi) = pdb.read_module_stream(&module)? {
                    count_sym_records(modi.sym_data(), &mut counts.module_sym_counts);
                    module_sym_size = modi.sym_byte_size as u64;
                } else {
                    module_sym_size = 0;
                }
                counts
                    .module_sym_sizes
                    .push((module_sym_size, module_index as u32));
            }

            // counts.module_infos.push(module);
            counts.sc.modules += pdb.stream_len(module_stream);
            counts.sc.modules_c13_lines += module.header().c13_byte_size.get() as u64;
            counts.sc.modules_syms += module.header().sym_byte_size.get() as u64;
        }
    }

    counts.sc.dbi += pdb.stream_len(Stream::DBI.into());
    counts.sc.dbi_contribs += pdb.dbi_header().section_contribution_size.get() as u64;
    counts.sc.dbi_modules += pdb.dbi_header().mod_info_size.get() as u64;
    counts.sc.dbi_sources += pdb.dbi_header().source_info_size.get() as u64;

    counts.sc.pdbi += pdb.stream_len(Stream::PDB.into());

    // TPI
    {
        let tpi_len = pdb.stream_len(Stream::TPI.into());
        counts.sc.tpi += tpi_len;
        if tpi_len as usize >= size_of::<TypeStreamHeader>() {
            let mut header: TypeStreamHeader = TypeStreamHeader::new_zeroed();
            let mut reader = pdb.get_stream_reader(Stream::TPI.into())?;
            reader.read_exact(header.as_bytes_mut())?;

            if let Some(s) = header.hash_aux_stream_index.get() {
                // TODO: yes, yes, I know, it's the wrong stream count
                counts.sc.tpi_hash += pdb.stream_len(s);
            }

            if let Some(s) = header.hash_stream_index.get() {
                counts.sc.tpi_hash += pdb.stream_len(s);
            }
        }
    }

    // IPI
    {
        let ipi_len = pdb.stream_len(Stream::IPI.into());
        counts.sc.ipi += ipi_len;
        if ipi_len as usize >= size_of::<TypeStreamHeader>() {
            let mut header: TypeStreamHeader = TypeStreamHeader::new_zeroed();
            let mut reader = pdb.get_stream_reader(Stream::IPI.into())?;
            reader.read_exact(header.as_bytes_mut())?;

            if let Some(s) = header.hash_aux_stream_index.get() {
                // TODO: yes, yes, I know, it's the wrong stream count
                counts.sc.ipi_hash += pdb.stream_len(s);
            }

            if let Some(s) = header.hash_stream_index.get() {
                counts.sc.ipi_hash += pdb.stream_len(s);
            }
        }
    }

    if let Ok(gss) = pdb.dbi_header().sym_record_stream() {
        counts.sc.gss += pdb.stream_len(gss);
    }

    if let Ok(gsi) = pdb.dbi_header().global_stream_index() {
        counts.sc.gsi += pdb.stream_len(gsi);
    }

    if let Ok(psi) = pdb.dbi_header().public_stream_index() {
        counts.sc.psi += pdb.stream_len(psi);
    }

    if options.global_symbols {
        count_global_symbols(pdb, counts)?;
    }

    for (name, stream) in pdb.named_streams().iter() {
        let stream_len = pdb.stream_len(*stream);

        let name = name.to_ascii_lowercase();

        let chopped_name: &str = if name.ends_with(".cs") {
            "*.cs"
        } else if name.ends_with(".cpp") || name.ends_with(".CPP") {
            "*.cpp"
        } else if name.ends_with(".natvis") {
            "*.natvis"
        } else if name.ends_with(".xaml") {
            "*.xaml"
        } else {
            &name
        };

        if let Some(slot) = counts.sc.named.get_mut(chopped_name) {
            *slot += stream_len;
        } else {
            counts.sc.named.insert(chopped_name.to_string(), stream_len);
        }
    }

    counts.sc.old_stream_dir += pdb.stream_len(Stream::OLD_STREAM_DIR.into());

    // Count the space wasted due to fragmentation in the final page of streams.
    if let Some(msf) = pdb.msf() {
        let page_size = msf.page_size();
        for i in 1..msf.num_streams() {
            let stream_size_bytes = msf.stream_size(i);
            let stream_size_phase = offset_within_page(stream_size_bytes, page_size);
            if stream_size_phase != 0 {
                counts.sc.stream_frag += (u32::from(page_size) - stream_size_phase) as u64;
            }
        }

        let num_free_pages = msf.num_free_pages();
        counts.sc.free_pages_bytes += (num_free_pages as u64) << page_size.exponent();
    }

    Ok(())
}

#[derive(Default, Clone)]
struct PerRecord {
    count: u32,
    bytes: u32,
}

fn read_and_count_type_records(
    pdb: &Pdb,
    counts: &mut TypeStreamCounts,
    stream: Stream,
) -> Result<()> {
    let tpi_type_stream = pdb.read_tpi_or_ipi_stream(stream)?;
    count_type_records(tpi_type_stream.type_records_bytes(), counts);
    Ok(())
}

fn count_type_records(type_records: &[u8], counts: &mut TypeStreamCounts) {
    for type_record in TypesIter::new(type_records) {
        let per_record = counts.records.entry(type_record.kind).or_default();
        per_record.count += 1;
        per_record.bytes += type_record.data.len() as u32 + 4; // 4 for the header
    }
}

fn count_sym_records(sym_records: &[u8], counts: &mut HashMap<SymKind, PerRecord>) {
    for sym_record in SymIter::new(sym_records) {
        let per_record = counts.entry(sym_record.kind).or_default();
        per_record.count += 1;
        per_record.bytes += sym_record.data.len() as u32 + 4; // 4 for the header
    }
}

fn dump_type_counts_map(record_counts: &TypeStreamCounts) {
    let mut record_counts_vec: Vec<(Leaf, PerRecord)> = record_counts
        .records
        .iter()
        .map(|(&kind, per_record)| (kind, per_record.clone()))
        .collect();
    record_counts_vec.sort_unstable_by_key(|&(key, _)| key);

    println!("    {:>8}  {:>12}", "records", "bytes");
    println!("    {:>8}  {:>12}", "-------", "-----");
    let mut total_count = 0;
    let mut total_bytes = 0;
    for &(kind, ref per_record) in record_counts_vec.iter() {
        let raw_kind = kind.0;
        let count = per_record.count;
        let bytes = per_record.bytes;
        println!("    {count:8}  {bytes:12} : [{raw_kind:04x}] {kind:?}");
        total_count += count;
        total_bytes += bytes;
    }

    println!("    {total_count:8}  {total_bytes:12} : (total)");
}

fn dump_sym_counts_map(record_counts: &HashMap<SymKind, PerRecord>) {
    let mut record_counts_vec: Vec<(SymKind, PerRecord)> = record_counts
        .iter()
        .map(|(&kind, per_record)| (kind, per_record.clone()))
        .collect();
    record_counts_vec.sort_unstable_by_key(|&(key, _)| key);

    println!("    {:>8}  {:>12}", "records", "bytes");
    println!("    {:>8}  {:>12}", "-------", "-----");
    let mut total_count = 0;
    let mut total_bytes = 0;
    for &(kind, ref per_record) in record_counts_vec.iter() {
        let raw_kind = kind.0;
        let count = per_record.count;
        let bytes = per_record.bytes;
        println!("    {count:8}  {bytes:12} : [{raw_kind:04x}] {kind:?}");
        total_count += count;
        total_bytes += bytes;
    }
    println!("    {total_count:8}  {total_bytes:12} : (total)");
}

fn show_counts(counts: &mut Counts) {
    println!("TPI Stream:");
    dump_type_counts_map(&counts.tpi);

    println!();

    println!("IPI Stream:");
    dump_type_counts_map(&counts.ipi);

    let module_sym_counts = &counts.module_sym_counts;

    println!("Record counts for module symbols (all modules):");
    dump_sym_counts_map(module_sym_counts);

    println!();

    counts
        .module_sym_sizes
        .sort_unstable_by_key(|(size, _)| *size);

    println!();

    println!("Record counts for Global Symbol Stream:");
    dump_sym_counts_map(&counts.global_syms_counts);

    println!();

    if !counts.module_sym_sizes.is_empty() {
        println!();

        let module_sym_size_percentile5 =
            counts.module_sym_sizes[counts.module_sym_sizes.len() * 5 / 100].0;
        let module_sym_size_median = counts.module_sym_sizes[counts.module_sym_sizes.len() / 2].0;
        let module_sym_size_percentile95 =
            counts.module_sym_sizes[counts.module_sym_sizes.len() * 95 / 100].0;
        println!("Number of modules: {}", counts.module_sym_sizes.len());
        println!("Module symbol stream sizes:");
        println!("    percentile  5%  : {:8}", module_sym_size_percentile5);
        println!("    percentile 50%  : {:8}", module_sym_size_median);
        println!("    percentile 95%  : {:8}", module_sym_size_percentile95);
        println!();
    }

    println!();
    println!(
        "Total size of all PDB files:       {}",
        friendly::bytes(counts.sc.total_file_size)
    );
    let pct = |size: u64| -> Percent { Percent(size as f64, counts.sc.total_file_size as f64) };

    let sc = &counts.sc;
    let total_named_stream_size: u64 = sc.named.values().copied().sum();

    let accounted_bytes = sc.tpi
        + sc.tpi_hash
        + sc.ipi
        + sc.ipi_hash
        + sc.gsi
        + sc.psi
        + total_named_stream_size
        + sc.pdbi
        + sc.modules
        + sc.gss
        + sc.old_stream_dir
        + sc.stream_frag
        + sc.free_pages_bytes;

    let unaccounted_bytes = sc.total_file_size - accounted_bytes;

    let show_one = |name: &str, size: u64| {
        println!(
            "    {:-20}     : {}, {}",
            name,
            friendly::bytes(size),
            pct(size)
        );
    };

    let show_level2 = |name: &str, size: u64| {
        println!(
            "        {:-20} : {}, {}",
            name,
            friendly::bytes(size),
            pct(size)
        );
    };

    show_one("PDBI streams", sc.pdbi);

    show_one("DBI streams", sc.dbi);
    show_level2("DBI Contribs", sc.dbi_contribs);
    show_level2("DBI Modules", sc.dbi_modules);
    show_level2("DBI Sources", sc.dbi_sources);

    show_one("TPI streams", sc.tpi);
    show_one("TPI hash streams", sc.tpi_hash);
    show_one("IPI streams", sc.ipi);
    show_one("IPI hash streams", sc.ipi_hash);

    show_one("Module streams", sc.modules);
    show_level2("Module symbols", sc.modules_syms);
    show_level2("Module line data", sc.modules_c13_lines);

    show_one("GSS streams", sc.gss);
    show_one("GSI streams", sc.gsi);
    show_one("PSI streams", sc.psi);

    show_one("Named streams", total_named_stream_size);
    for (name, count) in sc.named.iter() {
        show_level2(name, *count);
    }

    show_one("Old Stream Dir", sc.old_stream_dir);
    show_one("Page fragmentation", sc.stream_frag);
    show_one("Free pages", sc.free_pages_bytes);
    show_one("Unaccounted", unaccounted_bytes);
}

struct Percent(pub f64, pub f64);

impl std::fmt::Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.1 > 0.0 {
            let pct = self.0 / self.1 * 100.0;
            write!(f, "{pct:2.1} %")
        } else {
            write!(f, "n/a")
        }
    }
}
