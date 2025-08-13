use clap::Subcommand;

#[derive(Debug, Clone, clap::Args)]
pub struct UpdateArgs {
    pub token: Option<String>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    #[clap(hide = true)]
    Install,
    #[clap(hide = true)]
    Update(UpdateArgs),
}

impl Command {
    pub fn command_as_str(&self) -> String {
        match self {
            Command::Install => "install".to_string(),
            Command::Update(args) => {
                if let Some(token) = &args.token {
                    format!("update {token}")
                } else {
                    "update".to_string()
                }
            }
        }
    }
}
