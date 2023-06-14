use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct MdtCli {
  #[command(subcommand)]
  pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
  Init,
  Check,
  Update,
}
