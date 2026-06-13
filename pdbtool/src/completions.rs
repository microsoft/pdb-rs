use clap::CommandFactory;
use clap_complete::{generate, Generator, Shell};

use crate::CommandWithFlags;

/// Provides shell completions
#[derive(clap::Parser)]
pub(crate) struct CompletionsOptions {
    /// The shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
    generate(generator, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}

pub(crate) fn command(options: CompletionsOptions) -> anyhow::Result<()> {
    let shell = options.shell;
    let mut cmd = CommandWithFlags::command();
    print_completions(shell, &mut cmd);
    Ok(())
}
