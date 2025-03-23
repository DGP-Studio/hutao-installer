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
        let command = self.command();
        match command {
            Command::Install => "install".to_string(),
            Command::Update(args) => {
                if let Some(token) = args.token {
                    format!("update {}", token)
                } else {
                    "update".to_string()
                }
            }
        }
    }
}
