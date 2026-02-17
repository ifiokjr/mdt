use std::path::PathBuf;
use std::process;

use clap::Parser;
use mdt::check_project;
use mdt::compute_updates;
use mdt::project::ProjectContext;
use mdt::project::scan_project_with_config;
use mdt::write_updates;
use mdt_cli::Commands;
use mdt_cli::MdtCli;

fn main() {
	let args = MdtCli::parse();

	let result = match args.command {
		Some(Commands::Init) => run_init(&args),
		Some(Commands::Check) => run_check(&args),
		Some(Commands::Update { dry_run }) => run_update(&args, dry_run),
		Some(Commands::Lsp) => run_lsp(),
		None => {
			eprintln!("No subcommand specified. Run `mdt --help` for usage.");
			process::exit(1);
		}
	};

	if let Err(e) = result {
		eprintln!("error: {e}");
		process::exit(1);
	}
}

fn resolve_root(args: &MdtCli) -> PathBuf {
	args.path
		.clone()
		.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn run_init(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let template_path = root.join("template.t.md");

	if template_path.exists() {
		println!("Template file already exists: {}", template_path.display());
		return Ok(());
	}

	let sample_content = "<!-- {@greeting} -->\n\nHello from mdt! This is a provider \
	                      block.\n\n<!-- {/greeting} -->\n";

	std::fs::write(&template_path, sample_content)?;
	println!("Created template file: {}", template_path.display());
	println!();
	println!("Next steps:");
	println!(
		"  1. Edit {} to define your template blocks",
		template_path.display()
	);
	println!("  2. Add consumer tags in your markdown files:");
	println!("     <!-- {{=greeting}} -->");
	println!("     <!-- {{/greeting}} -->");
	println!("  3. Run `mdt update` to sync content");

	Ok(())
}

fn scan_and_warn(args: &MdtCli) -> Result<ProjectContext, Box<dyn std::error::Error>> {
	let root = resolve_root(args);
	let ctx = scan_project_with_config(&root)?;

	if args.verbose {
		println!(
			"Scanned project: {} provider(s), {} consumer(s)",
			ctx.project.providers.len(),
			ctx.project.consumers.len()
		);

		if !ctx.project.providers.is_empty() {
			println!("  Providers:");
			for (name, entry) in &ctx.project.providers {
				println!("    @{name} ({})", entry.file.display());
			}
		}
	}

	// Warn about consumers referencing non-existent providers
	for name in &ctx.find_missing_providers() {
		eprintln!("warning: consumer block `{name}` has no matching provider");
	}

	Ok(ctx)
}

fn run_check(args: &MdtCli) -> Result<(), Box<dyn std::error::Error>> {
	let ctx = scan_and_warn(args)?;
	let result = check_project(&ctx)?;

	if result.is_ok() {
		println!("All consumer blocks are up to date.");
		Ok(())
	} else {
		for entry in &result.stale {
			eprintln!(
				"Stale: block `{}` in {}",
				entry.block_name,
				entry.file.display()
			);
		}
		eprintln!(
			"\n{} consumer block(s) are out of date. Run `mdt update` to fix.",
			result.stale.len()
		);
		process::exit(1);
	}
}

fn run_update(args: &MdtCli, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
	let ctx = scan_and_warn(args)?;
	let updates = compute_updates(&ctx)?;

	if updates.updated_count == 0 {
		println!("All consumer blocks are already up to date.");
		return Ok(());
	}

	if dry_run {
		println!(
			"Dry run: would update {} block(s) in {} file(s):",
			updates.updated_count,
			updates.updated_files.len()
		);
		for path in updates.updated_files.keys() {
			println!("  {}", path.display());
		}
	} else {
		write_updates(&updates)?;
		println!(
			"Updated {} block(s) in {} file(s).",
			updates.updated_count,
			updates.updated_files.len()
		);

		if args.verbose {
			for path in updates.updated_files.keys() {
				println!("  {}", path.display());
			}
		}
	}

	Ok(())
}

fn run_lsp() -> Result<(), Box<dyn std::error::Error>> {
	let rt = tokio::runtime::Runtime::new()?;
	rt.block_on(mdt_lsp::run_server());
	Ok(())
}
