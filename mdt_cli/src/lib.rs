use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

#[derive(Parser)]
#[command(
	name = "mdt",
	// Without this, usage and errors say `mdt.exe` on Windows.
	bin_name = "mdt",
	author,
	version,
	arg_required_else_help = true,
	subcommand_required = true,
	about = "Keep documentation synchronized across your project using template tags.",
	long_about = "mdt (manage markdown templates) is a data-driven template engine for keeping \
	              documentation synchronized across your project.\n\nIt uses comment-based \
	              template tags to define content once and distribute it to multiple locations — \
	              markdown files, code comments, READMEs, and more.\n\nQuick start:\n  mdt init    \
	              Create a template file\n  mdt update  Sync all consumer blocks\n  mdt check   \
	              Verify everything is up to date\n  mdt info    Inspect project diagnostics\n  \
	              mdt doctor  Run project health checks\n  mdt assist  Print assistant setup \
	              snippets\n  mdt skill   Print the agent skill for AI coding assistants"
)]
#[allow(clippy::struct_excessive_bools)]
pub struct MdtCli {
	#[command(subcommand)]
	pub command: Commands,

	/// Path to the project root directory. Must exist, except for `mdt
	/// init`, which creates it. Defaults to the nearest directory, from the
	/// current one upward, that contains an mdt config file (or the current
	/// directory when there is none; `mdt init` always uses the current
	/// directory).
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

	/// Silence warnings about provider blocks that have no consumers.
	#[arg(long, global = true, default_value_t = false)]
	pub ignore_unused_blocks: bool,

	/// Ignore markdown comments that look like mdt tags but cannot be parsed
	/// (for example `{ @name }` or `{=my.block}`).
	#[arg(long, global = true, default_value_t = false)]
	pub ignore_invalid_names: bool,

	/// Ignore unknown transformer names and invalid transformer argument
	/// errors.
	#[arg(long, global = true, default_value_t = false)]
	pub ignore_invalid_transformers: bool,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Initialize mdt in a project, adding only what is missing.
	///
	/// Writes an annotated `mdt.toml` (unless a config exists) and a sample
	/// `greeting` provider in `.templates/template.t.md` (unless the project
	/// already has providers). When the project has no README, also writes
	/// `readme.md` with the sample consumer already in sync; an existing
	/// README is never modified. In git repositories, adds the `.mdt/` cache
	/// directory to `.gitignore`. The project passes `mdt check` afterwards.
	Init,
	/// Check that all consumer blocks are up to date.
	///
	/// Scans all files in the project for consumer blocks and compares their
	/// current content against what the matching provider would produce.
	///
	/// Exit status: 0 when every consumer is linked and current; 1 when a
	/// consumer is stale, names no provider (an orphan), or its provider fails
	/// to render; 2 when validation errors (for example an unclosed block) or
	/// config errors stop the check.
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
	/// Consumers whose provider fails to render are left untouched and
	/// reported, and the command exits with status 1; validation errors stop
	/// the update with status 2.
	///
	/// Use `--dry-run` to preview changes without writing to disk, or
	/// `--watch` to automatically re-run whenever source files change.
	Update {
		/// Print how many blocks and which files would change, without
		/// writing anything. Use `mdt check --diff` to see the content.
		#[arg(long, default_value_t = false, conflicts_with = "watch")]
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
	/// data namespaces from config, template file overview, diagnostic
	/// totals (errors, warnings, and missing providers), and cache
	/// observability metrics.
	Info {
		/// Output format for info results. Use `text` for human-readable
		/// output or `json` for programmatic consumption.
		#[arg(long, value_enum, default_value_t = InfoOutputFormat::Text)]
		format: InfoOutputFormat,
	},
	/// Run project health checks with actionable remediation hints.
	///
	/// Evaluates config discovery, data loading, provider/consumer linkage,
	/// template directory conventions, parser diagnostics, and cache
	/// effectiveness trends. Exits with a non-zero code when failing checks
	/// are present.
	Doctor {
		/// Output format for doctor results. Use `text` for human-readable
		/// output or `json` for programmatic consumption.
		#[arg(long, value_enum, default_value_t = DoctorOutputFormat::Text)]
		format: DoctorOutputFormat,
	},
	/// Print an official assistant setup profile.
	///
	/// This command prints a ready-to-copy MCP configuration snippet plus
	/// repo-local guidance for an assistant workflow. The first slice is
	/// intentionally lightweight: mdt ships presets and guidance, not a
	/// marketplace or plugin system.
	Assist {
		/// Which assistant profile to print.
		#[arg(value_enum)]
		assistant: Assistant,

		/// Output format for the setup profile.
		#[arg(long, value_enum, default_value_t = AssistOutputFormat::Text)]
		format: AssistOutputFormat,
	},
	/// Print the mdt agent skill so AI coding agents can learn mdt.
	///
	/// Prints `SKILL.md`, the same skill published as `@m-d-t/skills`,
	/// embedded in this binary so it always matches the installed version.
	/// An agent without the skill installed can run `mdt skill` to load it,
	/// then `mdt skill --reference` for the full reference.
	///
	/// Use `--install <DIR>` to write the skill into an agent skills
	/// directory, for example `.claude/skills` or `.agents/skills`.
	Skill {
		/// Print the detailed reference (`REFERENCE.md`) instead of
		/// `SKILL.md`.
		#[arg(long, default_value_t = false, conflicts_with = "install")]
		reference: bool,

		/// Write `SKILL.md` and `REFERENCE.md` to `<DIR>/mdt/` instead of
		/// printing, replacing any previous copy. Relative paths resolve
		/// against the current directory.
		#[arg(long, value_name = "DIR")]
		install: Option<PathBuf>,
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
	/// to give it structured access to mdt's template system. The server
	/// manages the current directory, or `--path <DIR>`; tool `path`
	/// arguments must stay inside it.
	///
	/// Exposes tools for checking, updating, and listing template blocks,
	/// allowing AI assistants to manage documentation synchronization.
	Mcp,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
	/// Human-readable text output with colors and formatting.
	Text,
	/// JSON output for programmatic consumption, with `ok`, `stale`,
	/// `stale_files`, `orphans`, `errors` (render errors), and `diagnostics`.
	/// Entries carry the file, block name, line, and column.
	Json,
	/// GitHub Actions annotation format. Emits `::error` annotations for
	/// every failure (and `::warning` for warnings) that appear inline on pull
	/// request diffs.
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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Assistant {
	/// Generic MCP client configuration.
	Generic,
	/// Anthropic Claude / Claude Code style setup.
	Claude,
	/// Cursor editor MCP setup.
	Cursor,
	/// GitHub Copilot / VS Code MCP-style setup.
	Copilot,
	/// Pi coding agent setup.
	Pi,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AssistOutputFormat {
	/// Human-readable setup instructions.
	Text,
	/// JSON output for programmatic consumption.
	Json,
}
