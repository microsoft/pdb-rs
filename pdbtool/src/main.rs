#![forbid(unused_must_use)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::manual_map)]
#![allow(clippy::single_match)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_late_init)]

use clap::Parser;

mod addsrc;
mod copy;
mod counts;
mod dump;
mod dump_utils;
mod find;
mod glob_pdbs;
mod hexdump;
mod pdz;
mod save;
mod util;

#[derive(clap::Parser)]
struct CommandWithFlags {
    /// Reduce logging to just warnings and errors in `mspdb` and `pdbtool` modules.
    #[arg(long)]
    quiet: bool,

    /// Turn on debug output in all `mspdb` and `pdbtool` modules. Noisy!
    #[arg(long)]
    verbose: bool,

    /// Show timestamps in log messages
    #[arg(long)]
    timestamps: bool,

    /// Connect to Tracy (diagnostics tool). Requires that the `tracy` Cargo feature be enabled.
    #[arg(long)]
    tracy: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
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
    Counts(counts::CountsOptions),
    /// Dumps part of a file (any file, not just a PDB) as a hex dump. If you want to dump a
    /// specific stream, then use the `dump <filename> hex` command instead.
    Hexdump(hexdump::HexdumpOptions),
    PdzEncode(pdz::encode::PdzEncodeOptions),
}

fn main() -> anyhow::Result<()> {
    let command_with_flags = CommandWithFlags::parse();
    configure_tracing(&command_with_flags);

    match command_with_flags.command {
        Command::AddSrc(args) => addsrc::command(args)?,
        Command::Dump(args) => dump::dump_main(args)?,
        Command::Test => {}
        Command::Copy(args) => copy::copy_command(&args)?,
        Command::Save(args) => save::save_stream(&args)?,
        Command::Find(args) => find::find_command(&args)?,
        Command::FindName(args) => find::find_name_command(&args)?,
        Command::Counts(args) => counts::counts_command(args)?,
        Command::Hexdump(args) => hexdump::command(args)?,
        Command::PdzEncode(args) => pdz::encode::pdz_encode(args)?,
    }

    Ok(())
}

fn configure_tracing(args: &CommandWithFlags) {
    use tracing_subscriber::filter::LevelFilter;

    if args.tracy {
        #[cfg(feature = "tracy")]
        {
            use tracing_subscriber::layer::SubscriberExt;

            let layer = tracing_tracy::TracyLayer::default();
            tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer))
                .expect("setup tracy layer");

            return;
        }

        #[cfg(not(feature = "tracy"))]
        {
            eprintln!(
                "Tracing is not enabled in the build configuration.\n\
                 You can enable it by using 'cargo run --features \"tracy\"'."
            );
        }
    }

    let builder = tracing_subscriber::fmt();

    let max_level = if args.quiet {
        LevelFilter::WARN
    } else if args.verbose {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    builder.with_max_level(max_level).finish();
}
