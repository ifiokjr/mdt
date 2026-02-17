use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct MdtCli {
	#[command(subcommand)]
	pub command: Option<Commands>,

	/// Path to the project root directory.
	#[arg(long, short, global = true)]
	pub path: Option<PathBuf>,

	/// Enable verbose output.
	#[arg(long, short, global = true, default_value_t = false)]
	pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Initialize mdt in a project by creating a sample template file.
	Init,
	/// Check that all consumer blocks are up to date.
	Check,
	/// Update all consumer blocks with the latest provider content.
	Update {
		/// Show what would change without writing files.
		#[arg(long, default_value_t = false)]
		dry_run: bool,
	},
}
