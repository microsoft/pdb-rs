#![forbid(unused_must_use)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::manual_map)]
#![allow(clippy::single_match)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_late_init)]

use structopt::StructOpt;

mod addsrc;
mod check;
mod copy;
mod counts;
mod dump;
mod find;
mod glob_pdbs;
mod hash;
mod hexdump;
mod oscheck;
mod pdz;
mod save;
mod util;

#[derive(StructOpt)]
struct CommandWithFlags {
    /// Reduce logging to just warnings and errors in `mspdb` and `pdbtool` modules.
    #[structopt(long, short)]
    quiet: bool,

    /// Turn on debug output in all `mspdb` and `pdbtool` modules. Noisy!
    #[structopt(long, short)]
    verbose: bool,

    /// Custom log directives, using same format as `RUST_LOG`.
    #[structopt(long, short)]
    log: Option<String>,

    /// Show timestamps in log messages
    #[structopt(long, short)]
    timestamps: bool,

    /// Show source lines of logging messages.
    #[structopt(long, short)]
    source: bool,

    /// Connect to Tracy (diagnostics tool)
    #[structopt(long)]
    tracy: bool,

    #[structopt(flatten)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    /// Adds source file contents to the PDB. The contents are embedded directly within the PDB.
    /// WinDbg and Visual Studio can both extract the source files.
    AddSrc(addsrc::AddSrcOptions),
    /// Copies a PDB from one file to another. All stream contents are preserved exactly, byte-for-byte.
    /// The blocks within streams are laid out sequentially.
    Copy(copy::Options),
    Test,
    Dump(dump::DumpOptions),
    Save(save::SaveStreamOptions),
    Find(find::FindOptions),
    FindName(find::FindNameOptions),
    Check(check::CheckOptions),
    Counts(counts::CountsOptions),
    Oscheck(oscheck::OSCheckOptions),
    /// Dumps part of a file (any file, not just a PDB) as a hex dump. If you want to dump a
    /// specific stream, then use the `dump <filename> hex` command instead.
    Hexdump(hexdump::HexdumpOptions),
    PdzEncode(pdz::encode::PdzEncodeOptions),
    // PdzDecode(pdz::decode::PdzDecodeOptions),
}

fn main() -> anyhow::Result<()> {
    let command_with_flags = CommandWithFlags::from_args();

    if command_with_flags.tracy {
        enable_tracy();
    } else {
        let mut builder = tracing_subscriber::fmt();
        if !command_with_flags.timestamps {
            // builder = builder.without_time();
        }

        if command_with_flags.quiet {
            builder = builder.with_max_level(tracing_subscriber::filter::LevelFilter::WARN);
        } else if command_with_flags.verbose {
            builder = builder.with_max_level(tracing_subscriber::filter::LevelFilter::DEBUG);
        } else {
            builder = builder.with_max_level(tracing_subscriber::filter::LevelFilter::INFO);
        }

        if let Some(log) = &command_with_flags.log {
            // builder.parse_filters(log);
        }

        /*
        if command_with_flags.source {
            builder.format(|buf, record| {
                use std::io::Write;
                // ...
                writeln!(
                    buf,
                    "{:6} - {}:{:<5}] {}",
                    record.level(),
                    record.file().unwrap_or("??"),
                    record.line().unwrap_or(1),
                    record.args()
                )
            });
        }

        builder.parse_env("PDBTOOL_LOG");
        builder.init();
        */

        builder.finish();
    }

    match command_with_flags.command {
        Command::AddSrc(args) => addsrc::command(args)?,
        Command::Dump(args) => dump::dump_main(args)?,
        Command::Test => {}
        Command::Copy(args) => copy::copy_command(&args)?,
        Command::Save(args) => save::save_stream(&args)?,
        Command::Find(args) => find::find_command(&args)?,
        Command::Check(args) => check::check_command(args)?,
        Command::FindName(args) => find::find_name_command(&args)?,
        Command::Counts(args) => counts::counts_command(args)?,
        Command::Oscheck(args) => oscheck::oscheck_command(args)?,
        Command::Hexdump(args) => hexdump::command(args)?,
        Command::PdzEncode(args) => pdz::encode::pdz_encode(args)?,
    }

    // std::thread::sleep_ms(5000);

    Ok(())
}

fn enable_tracy() {
    use tracing_subscriber::layer::SubscriberExt;

    let layer = tracing_tracy::TracyLayer::default();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer))
        .expect("setup tracy layer");
}
