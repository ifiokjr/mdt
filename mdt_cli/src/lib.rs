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
	              Verify everything is up to date\n  mdt info    Inspect project diagnostics\n  \
	              mdt doctor  Run project health checks"
)]
#[allow(clippy::struct_excessive_bools)]
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

	/// Ignore unclosed block errors during validation.
	#[arg(long, global = true, default_value_t = false)]
	pub ignore_unclosed_blocks: bool,

	/// Ignore unused provider blocks (providers with no consumers).
	#[arg(long, global = true, default_value_t = false)]
	pub ignore_unused_blocks: bool,

	/// Ignore invalid block name errors.
	#[arg(long, global = true, default_value_t = false)]
	pub ignore_invalid_names: bool,

	/// Ignore unknown transformer names and invalid transformer argument
	/// errors.
	#[arg(long, global = true, default_value_t = false)]
	pub ignore_invalid_transformers: bool,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Initialize mdt in a project by creating a sample template file.
	///
	/// Creates a `template.t.md` file in the project root with example provider
	/// blocks and usage instructions. If the file already exists, this command
	/// is a no-op and exits successfully.
	Init,
	/// Check that all consumer blocks are up to date.
	///
	/// Scans all files in the project for consumer blocks and compares their
	/// current content against what the matching provider would produce. Exits
	/// with a non-zero status code if any consumer blocks are stale.
	///
	/// Ideal for CI pipelines to enforce documentation synchronization. Use
	/// `--diff` to see exactly what changed and `--format` to control the
	/// output style.
	Check {
		/// Show a unified diff for each stale consumer block, highlighting
		/// the differences between current and expected content.
		#[arg(long, default_value_t = false)]
		diff: bool,

		/// Output format for check results. Use `text` for human-readable
		/// output, `json` for programmatic consumption, or `github` for
		/// GitHub Actions annotations that appear inline on PRs.
		#[arg(long, value_enum, default_value_t = OutputFormat::Text)]
		format: OutputFormat,

		/// Watch for file changes and re-run checks automatically. Monitors
		/// template files and consumer files for modifications.
		#[arg(long, default_value_t = false)]
		watch: bool,
	},
	/// Update all consumer blocks with the latest provider content.
	///
	/// Reads provider blocks from `*.t.md` template files, renders any
	/// template variables using data from `mdt.toml`, applies transformers,
	/// and replaces matching consumer block content in all scanned files.
	///
	/// Use `--dry-run` to preview changes without writing to disk, or
	/// `--watch` to automatically re-run whenever source files change.
	Update {
		/// Preview changes without writing files. Prints which files would
		/// be modified and shows the updated content.
		#[arg(long, default_value_t = false)]
		dry_run: bool,

		/// Watch for file changes and re-run updates automatically. Monitors
		/// template files and consumer files for modifications.
		#[arg(long, default_value_t = false)]
		watch: bool,
	},
	/// List all provider and consumer blocks in the project.
	///
	/// Displays every provider block (from `*.t.md` files) and consumer block
	/// found across the project, along with file paths and block names. Useful
	/// for auditing template coverage and discovering orphaned consumers.
	List,
	/// Print a diagnostic summary of the current project.
	///
	/// Shows discovered providers/consumers, orphan and unused counts,
	/// data namespaces from config, template file overview, and diagnostic
	/// totals (errors, warnings, and missing providers).
	Info {
		/// Output format for info results. Use `text` for human-readable
		/// output or `json` for programmatic consumption.
		#[arg(long, value_enum, default_value_t = InfoOutputFormat::Text)]
		format: InfoOutputFormat,
	},
	/// Run project health checks with actionable remediation hints.
	///
	/// Evaluates config discovery, data loading, provider/consumer linkage,
	/// template directory conventions, and parser diagnostics. Exits with a
	/// non-zero code when failing checks are present.
	Doctor {
		/// Output format for doctor results. Use `text` for human-readable
		/// output or `json` for programmatic consumption.
		#[arg(long, value_enum, default_value_t = DoctorOutputFormat::Text)]
		format: DoctorOutputFormat,
	},
	/// Start the mdt language server (LSP).
	///
	/// Communicates over stdin/stdout using the Language Server Protocol.
	/// Configure your editor to run `mdt lsp` as the language server
	/// command for markdown and template files.
	///
	/// Provides diagnostics for stale consumers, auto-completion of block
	/// names, hover information, and go-to-definition from consumers to
	/// their providers.
	Lsp,
	/// Start the mdt MCP (Model Context Protocol) server.
	///
	/// Communicates over stdin/stdout using the Model Context Protocol.
	/// Configure your AI assistant to run `mdt mcp` as an MCP server
	/// to give it structured access to mdt's template system.
	///
	/// Exposes tools for checking, updating, and listing template blocks,
	/// allowing AI assistants to manage documentation synchronization.
	Mcp,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
	/// Human-readable text output with colors and formatting.
	Text,
	/// JSON output for programmatic consumption. Each stale entry includes
	/// the file path, block name, current content, and expected content.
	Json,
	/// GitHub Actions annotation format. Emits `::warning` or `::error`
	/// annotations that appear inline on pull request diffs.
	Github,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum InfoOutputFormat {
	/// Human-readable text output with colors and formatting.
	Text,
	/// JSON output for programmatic consumption.
	Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DoctorOutputFormat {
	/// Human-readable text output with colors and formatting.
	Text,
	/// JSON output for programmatic consumption.
	Json,
}
