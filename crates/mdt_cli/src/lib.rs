use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

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

	/// Disable colored output.
	#[arg(long, global = true, default_value_t = false)]
	pub no_color: bool,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Initialize mdt in a project by creating a sample template file.
	Init,
	/// Check that all consumer blocks are up to date.
	///
	/// Exits with a non-zero status code if any consumer blocks are stale.
	/// Use this in CI to ensure documentation stays synchronized.
	Check {
		/// Show a diff for each stale consumer block.
		#[arg(long, default_value_t = false)]
		diff: bool,

		/// Output format for check results.
		#[arg(long, value_enum, default_value_t = OutputFormat::Text)]
		format: OutputFormat,
	},
	/// Update all consumer blocks with the latest provider content.
	///
	/// Reads provider blocks from *.t.md template files and replaces
	/// matching consumer blocks in all scanned files.
	Update {
		/// Show what would change without writing files.
		#[arg(long, default_value_t = false)]
		dry_run: bool,

		/// Watch for file changes and re-run updates automatically.
		#[arg(long, default_value_t = false)]
		watch: bool,
	},
	/// List all provider and consumer blocks in the project.
	List,
	/// Start the mdt language server (LSP).
	///
	/// Communicates over stdin/stdout using the Language Server Protocol.
	/// Configure your editor to run `mdt lsp` as the language server
	/// command for markdown and template files.
	Lsp,
	/// Start the mdt MCP (Model Context Protocol) server.
	///
	/// Communicates over stdin/stdout using the Model Context Protocol.
	/// Configure your AI assistant to run `mdt mcp` as an MCP server
	/// to give it structured access to mdt's template system.
	Mcp,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
	/// Human-readable text output.
	Text,
	/// JSON output for programmatic consumption.
	Json,
	/// GitHub Actions annotation format.
	Github,
}
