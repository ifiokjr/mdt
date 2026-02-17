use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(
	author,
	version,
	about = "Keep documentation synchronized across your project using template tags.",
	long_about = "mdt (manage markdown templates) is a data-driven template engine for keeping \
	              documentation synchronized across your project.\n\nIt uses comment-based \
	              template tags to define content once and distribute it to multiple locations — \
	              markdown files, code comments, READMEs, and more.\n\nQuick start:\n  mdt init    \
	              Create a template file\n  mdt update  Sync all consumer blocks\n  mdt check   \
	              Verify everything is up to date"
)]
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
	///
	/// Exits with a non-zero status code if any consumer blocks are stale.
	/// Use this in CI to ensure documentation stays synchronized.
	Check,
	/// Update all consumer blocks with the latest provider content.
	///
	/// Reads provider blocks from *.t.md template files and replaces
	/// matching consumer blocks in all scanned files.
	Update {
		/// Show what would change without writing files.
		#[arg(long, default_value_t = false)]
		dry_run: bool,
	},
	/// Start the mdt language server (LSP).
	///
	/// Communicates over stdin/stdout using the Language Server Protocol.
	/// Configure your editor to run `mdt lsp` as the language server
	/// command for markdown and template files.
	Lsp,
}
