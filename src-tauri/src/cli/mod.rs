pub mod arg;

use arg::Command;
use clap::Parser;

#[derive(Parser)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}
impl Cli {
    pub fn command(&self) -> Command {
        self.command.clone().unwrap_or(Command::Install)
    }

    pub fn command_as_str(&self) -> String {
        self.command().command_as_str().to_string()
    }
}
