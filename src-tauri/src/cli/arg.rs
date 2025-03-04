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
