use clap::Parser;
use mdt_cli::Commands;
use mdt_cli::MdtCli;

fn main() {
  let args = MdtCli::parse();

  match args.command {
    Some(Commands::Init) => {
      println!("initializing project!");
    }
    Some(Commands::Check) => {
      // Check the mdt code blocks
    }
    Some(Commands::Update) => {
      // Update the mdt code blocks
    }
    None => {
      println!("No subcommand specified");
    }
  }
}
