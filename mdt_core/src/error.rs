use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[non_exhaustive]
pub enum MdtError {
	#[error(transparent)]
	#[diagnostic(code(mdt::io_error))]
	Io(#[from] std::io::Error),

	#[error("failure to load markdown: {0}")]
	#[diagnostic(code(mdt::markdown))]
	Markdown(String),

	#[error("missing closing tag for block: `{0}`")]
	#[diagnostic(
		code(mdt::missing_closing_tag),
		help("add `<!-- {{/{0}}} -->` to close this block")
	)]
	MissingClosingTag(String),

	#[error("invalid token sequence")]
	#[diagnostic(code(mdt::invalid_token_sequence))]
	InvalidTokenSequence(usize),

	#[error("no provider found for consumer block: `{0}`")]
	#[diagnostic(
		code(mdt::missing_provider),
		help("define a provider block `<!-- {{@{0}}} -->...<!-- {{/{0}}} -->` in a *.t.md file")
	)]
	MissingProvider(String),

	#[error("consumer block `{name}` in {file} is out of date")]
	#[diagnostic(
		code(mdt::stale_consumer),
		help("run `mdt update` to synchronize consumer blocks")
	)]
	StaleConsumer { name: String, file: String },

	#[error("failed to parse config file: {0}")]
	#[diagnostic(
		code(mdt::config_parse),
		help(
			"mdt.toml must be valid TOML using only supported keys (unknown keys are rejected to \
			 catch typos): max_file_size, disable_gitignore, [data], [padding], [check], \
			 [exclude], [include], [templates], and [[formatters]]"
		)
	)]
	ConfigParse(String),

	#[error("failed to load data file `{path}`: {reason}")]
	#[diagnostic(code(mdt::data_file))]
	DataFile { path: String, reason: String },

	#[error("failed to execute data script for `{namespace}`: {reason}")]
	#[diagnostic(code(mdt::data_script))]
	DataScript { namespace: String, reason: String },

	#[error("unsupported data file format: `{0}`")]
	#[diagnostic(
		code(mdt::unsupported_format),
		help("supported formats: text, json, toml, yaml, yml, kdl, ini")
	)]
	UnsupportedDataFormat(String),

	#[error("template rendering failed: {0}")]
	#[diagnostic(code(mdt::template_render))]
	TemplateRender(String),

	#[error("duplicate provider `{name}`: defined in `{first_file}` and `{second_file}`")]
	#[diagnostic(
		code(mdt::duplicate_provider),
		help("each provider block name must be unique across the project")
	)]
	DuplicateProvider {
		name: String,
		first_file: String,
		second_file: String,
	},

	#[error("unknown transformer: `{0}`")]
	#[diagnostic(
		code(mdt::unknown_transformer),
		help(
			"available transformers: trim, trimStart, trimEnd, indent, prefix, suffix, \
			 linePrefix, lineSuffix, wrap, codeBlock, code, replace, if"
		)
	)]
	UnknownTransformer(String),

	#[error("transformer `{name}` expects {expected} argument(s), got {got}")]
	#[diagnostic(code(mdt::invalid_transformer_args))]
	InvalidTransformerArgs {
		name: String,
		expected: String,
		got: usize,
	},

	#[error("`[templates] paths` entry `{path}` is not a directory")]
	#[diagnostic(
		code(mdt::templates_path),
		help("paths are relative to the project root; fix the path or remove the entry")
	)]
	TemplatesPath { path: String },

	#[error("failed to read `{path}`: {reason}")]
	#[diagnostic(
		code(mdt::read_file),
		help(
			"mdt reads scanned files as UTF-8 text; exclude binary or generated files with \
			 `[exclude] patterns`, or narrow `[include] patterns` to text file extensions"
		)
	)]
	ReadFile { path: String, reason: String },

	#[error("file too large: `{path}` is {size} bytes (limit: {limit} bytes)")]
	#[diagnostic(
		code(mdt::file_too_large),
		help("increase the file size limit in mdt.toml or exclude this file")
	)]
	FileTooLarge { path: String, size: u64, limit: u64 },

	/// No longer returned: scanning visits each directory once, so symlink
	/// aliases and cycles are skipped instead of failing the scan.
	#[deprecated(note = "scanning skips already-visited directories instead of failing")]
	#[error("symlink cycle detected at: `{path}`")]
	#[diagnostic(
		code(mdt::symlink_cycle),
		help("remove the circular symlink or exclude this path")
	)]
	SymlinkCycle { path: String },

	#[error("formatter failed for `{file}` while running `{command}`: {reason}")]
	#[diagnostic(
		code(mdt::formatter),
		help(
			"check the formatter command, its config, and whether the formatter preserves mdt \
			 block tags"
		)
	)]
	Formatter {
		file: String,
		command: String,
		reason: String,
	},

	#[error("unconvertible float value in data file `{path}`: {value}")]
	#[diagnostic(
		code(mdt::unconvertible_float),
		help("NaN and Infinity are not valid JSON numbers")
	)]
	UnconvertibleFloat { path: String, value: String },
}

pub type MdtResult<T> = Result<T, MdtError>;

/// Converts an `MdtError` into an `std::io::Error` for test compatibility.
/// This allows tests to use `?` with mixed error types.
impl From<MdtError> for std::io::Error {
	fn from(error: MdtError) -> Self {
		std::io::Error::other(error.to_string())
	}
}
