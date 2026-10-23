/**
 * Builds the static website for GitHub Pages: the landing page plus every
 * page listed in `docs/src/SUMMARY.md`, rendered with `marked` and
 * highlighted with `shiki`.
 *
 * `SUMMARY.md` stays the single source of truth for navigation, so the same
 * docs still build with mdBook. Old `/book/...` URLs are kept alive with
 * redirect stubs.
 *
 * Usage: `pnpm site:build [outDir]` (defaults to `site/`).
 */

import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { Marked, type Tokens } from "marked";
import { createHighlighter, type Highlighter } from "shiki";

const root = path.resolve(
	path.dirname(fileURLToPath(import.meta.url)),
	"../..",
);
const docsSource = path.join(root, "docs/src");
const staticDir = path.join(root, "scripts/site/static");
const outDir = path.resolve(root, process.argv[2] ?? "site");

const repoUrl = "https://github.com/ifiokjr/mdt";
const siteUrl = "https://ifiokjr.github.io/mdt";
const languages = [
	"markdown",
	"rust",
	"shellscript",
	"toml",
	"json",
	"yaml",
	"typescript",
	"python",
	"go",
	"dart",
	"html",
];
const languageAliases: Record<string, string> = {
	sh: "shellscript",
	bash: "shellscript",
	md: "markdown",
	ts: "typescript",
};

/** A page from `SUMMARY.md`. `source` is relative to `docs/src`. */
interface DocPage {
	title: string;
	section: string;
	source: string;
	/** Output path relative to the site root, e.g. `docs/guide/transformers.html`. */
	output: string;
}

interface Heading {
	depth: number;
	text: string;
	id: string;
}

interface SearchEntry {
	title: string;
	section: string;
	url: string;
	headings: string[];
	text: string;
}

/** Maps a markdown source path to its output path under `docs/`. */
function outputFor(source: string): string {
	const html = source === "readme.md"
		? "index.html"
		: source.replace(/\.md$/, ".html");
	return path.posix.join("docs", html);
}

function parseSummary(summary: string): DocPage[] {
	const pages: DocPage[] = [];
	let section = "Introduction";

	for (const line of summary.split("\n")) {
		const heading = /^#\s+(.+)$/.exec(line);
		if (heading && heading[1] !== "Summary") {
			section = heading[1].trim();
			continue;
		}

		const link = /^\s*(?:-\s+)?\[([^\]]+)\]\(\.\/([^)]+\.md)\)/.exec(line);
		if (!link) continue;

		const source = link[2];
		pages.push({ title: link[1], section, source, output: outputFor(source) });
	}

	return pages;
}

function slugify(text: string): string {
	return text
		.toLowerCase()
		.replace(/<[^>]+>/g, "")
		.replace(/[`*_]/g, "")
		.replace(/&[a-z]+;/g, "")
		.replace(/[^\p{L}\p{N}\s-]/gu, "")
		.trim()
		.replace(/\s+/g, "-");
}

function escapeHtml(text: string): string {
	return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
		.replace(/"/g, "&quot;");
}

/** Strips markdown and HTML so a page can be indexed for search. */
function plainText(markdown: string): string {
	return markdown
		.replace(/<!--[\s\S]*?-->/g, " ")
		.replace(/```[\s\S]*?```/g, " ")
		.replace(/<[^>]+>/g, " ")
		.replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
		.replace(/[#>*_`|-]+/g, " ")
		.replace(/\s+/g, " ")
		.trim();
}

/** Resolves a relative markdown link from `fromSource` into a site-relative href. */
function rewriteHref(
	href: string,
	fromSource: string,
	fromOutput: string,
): string {
	if (/^[a-z]+:|^#|^\//i.test(href)) return href;

	const [target, hash] = href.split("#");
	if (!target.endsWith(".md")) return href;

	const targetSource = path.posix.normalize(
		path.posix.join(path.posix.dirname(fromSource), target),
	);
	const targetOutput = outputFor(targetSource);
	const relative = path.posix.relative(
		path.posix.dirname(fromOutput),
		targetOutput,
	);
	return hash ? `${relative}#${hash}` : relative;
}

function renderMarkdown(
	markdown: string,
	page: DocPage,
	highlighter: Highlighter,
): { html: string; headings: Heading[] } {
	const headings: Heading[] = [];
	const seen = new Map<string, number>();
	const marked = new Marked({ gfm: true });

	marked.use({
		renderer: {
			heading(token: Tokens.Heading) {
				const inner = this.parser.parseInline(token.tokens);
				const base = slugify(token.text) || "section";
				const count = seen.get(base) ?? 0;
				seen.set(base, count + 1);
				const id = count === 0 ? base : `${base}-${count}`;
				if (token.depth === 2 || token.depth === 3) {
					headings.push({
						depth: token.depth,
						text: inner.replace(/<[^>]+>/g, ""),
						id,
					});
				}
				if (token.depth === 1) return `<h1>${inner}</h1>\n`;
				return `<h${token.depth} id="${id}"><a class="anchor" href="#${id}" aria-hidden="true">#</a>${inner}</h${token.depth}>\n`;
			},
			code(token: Tokens.Code) {
				const requested = (token.lang ?? "").trim().split(/\s+/)[0];
				const lang = languageAliases[requested] ?? requested;
				const known = languages.includes(lang);
				const highlighted = highlighter.codeToHtml(token.text, {
					lang: known ? lang : "text",
					themes: { light: "github-light", dark: "vitesse-dark" },
					defaultColor: false,
				});
				const label = requested && requested !== "text"
					? `<span class="code-lang">${escapeHtml(requested)}</span>`
					: "";
				return `<div class="code">${label}<button class="copy" type="button" aria-label="Copy code">Copy</button>${highlighted}</div>\n`;
			},
			link(token: Tokens.Link) {
				const href = rewriteHref(token.href, page.source, page.output);
				const inner = this.parser.parseInline(token.tokens);
				const external = /^https?:/.test(href) ? ' rel="noopener"' : "";
				const title = token.title ? ` title="${escapeHtml(token.title)}"` : "";
				return `<a href="${escapeHtml(href)}"${title}${external}>${inner}</a>`;
			},
			table(token: Tokens.Table) {
				const head = token.header.map((cell) =>
					`<th>${this.parser.parseInline(cell.tokens)}</th>`
				).join("");
				const body = token.rows
					.map((row) =>
						`<tr>${
							row.map((cell) =>
								`<td>${this.parser.parseInline(cell.tokens)}</td>`
							).join("")
						}</tr>`
					)
					.join("\n");
				return `<div class="table"><table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>\n`;
			},
		},
	});

	const html = marked.parse(markdown, { async: false });
	return { html, headings };
}

/** The shared top bar. `base` is the relative path back to the site root. */
function topBar(base: string, active: "home" | "docs"): string {
	return `<header class="topbar">
	<div class="topbar-inner">
		<a class="brand" href="${base}index.html" aria-label="mdt home">
			<img src="${base}assets/mdt-mark.svg" alt="" width="30" height="30">
			<span>mdt</span>
		</a>
		<nav class="topnav" aria-label="Primary">
			<a href="${base}docs/index.html"${
		active === "docs" ? ' aria-current="page"' : ""
	}>Docs</a>
			<a href="${base}docs/getting-started/quick-start.html">Quick start</a>
			<a href="${base}docs/reference/cli.html">CLI</a>
			<a href="${repoUrl}" rel="noopener">GitHub</a>
		</nav>
		<button class="search-trigger" type="button" data-search-open aria-label="Search the docs">
			<svg viewBox="0 0 20 20" width="15" height="15" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.5" fill="none" stroke="currentColor" stroke-width="2"/><path d="M13 13l4.5 4.5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
			<span>Search</span><kbd>/</kbd>
		</button>
		<button class="theme-toggle" type="button" data-theme-toggle aria-label="Toggle dark mode">
			<svg viewBox="0 0 20 20" width="16" height="16" aria-hidden="true"><path d="M10 2a8 8 0 1 0 0 16z" fill="currentColor"/><circle cx="10" cy="10" r="7.25" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>
		</button>
	</div>
</header>`;
}

function searchDialog(base: string): string {
	return `<dialog class="search" data-search data-base="${base}" aria-label="Search the docs">
	<form method="dialog" class="search-box">
		<input type="search" placeholder="Search the docs…" aria-label="Search the docs" autocomplete="off" spellcheck="false" data-search-input>
		<button type="submit" class="search-close" aria-label="Close search">esc</button>
	</form>
	<ol class="search-results" data-search-results></ol>
</dialog>`;
}

function head(
	title: string,
	description: string,
	base: string,
	canonical: string,
): string {
	return `<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<meta name="description" content="${escapeHtml(description)}">
<meta property="og:title" content="${escapeHtml(title)}">
<meta property="og:description" content="${escapeHtml(description)}">
<meta property="og:image" content="${siteUrl}/assets/og.png">
<meta property="og:url" content="${siteUrl}/${canonical}">
<meta name="twitter:card" content="summary_large_image">
<link rel="canonical" href="${siteUrl}/${canonical}">
<link rel="icon" href="${base}assets/mdt-icon.svg" type="image/svg+xml">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=DM+Mono:wght@400;500&family=Instrument+Serif:ital@0;1&family=Schibsted+Grotesk:wght@400;500;600;700&display=swap">
<link rel="stylesheet" href="${base}assets/style.css">
<script>try{const t=localStorage.getItem("mdt-theme");if(t)document.documentElement.dataset.theme=t}catch{}</script>
<script src="${base}assets/app.js" defer></script>`;
}

function renderDocPage(
	page: DocPage,
	pages: DocPage[],
	body: string,
	headings: Heading[],
	description: string,
): string {
	const depth = page.output.split("/").length - 1;
	const base = "../".repeat(depth);
	const index = pages.indexOf(page);
	const previous = pages[index - 1];
	const next = pages[index + 1];
	const href = (target: DocPage) =>
		path.posix.relative(path.posix.dirname(page.output), target.output);

	const sections = new Map<string, DocPage[]>();
	for (const entry of pages) {
		sections.set(entry.section, [
			...(sections.get(entry.section) ?? []),
			entry,
		]);
	}

	const sidebar = [...sections]
		.map(([section, entries]) => {
			const links = entries
				.map((entry) =>
					`<li><a href="${href(entry)}"${
						entry === page ? ' aria-current="page"' : ""
					}>${escapeHtml(entry.title)}</a></li>`
				)
				.join("");
			return `<div class="side-group"><p class="side-title">${
				escapeHtml(section)
			}</p><ul>${links}</ul></div>`;
		})
		.join("\n");

	const toc = headings.length > 1
		? `<aside class="toc" aria-label="On this page"><p class="side-title">On this page</p><ul>${
			headings.map((h) =>
				`<li class="toc-${h.depth}"><a href="#${h.id}">${
					escapeHtml(h.text)
				}</a></li>`
			).join("")
		}</ul></aside>`
		: `<aside class="toc" aria-hidden="true"></aside>`;

	const pager = `<nav class="pager" aria-label="Pagination">${
		previous
			? `<a class="pager-prev" href="${href(previous)}"><span>Previous</span>${
				escapeHtml(previous.title)
			}</a>`
			: "<span></span>"
	}${
		next
			? `<a class="pager-next" href="${href(next)}"><span>Next</span>${
				escapeHtml(next.title)
			}</a>`
			: "<span></span>"
	}</nav>`;

	return `<!doctype html>
<html lang="en">
<head>
${head(`${page.title} · mdt`, description, base, page.output)}
</head>
<body class="docs-body">
<a class="skip" href="#content">Skip to content</a>
${topBar(base, "docs")}
<div class="docs">
	<button class="side-toggle" type="button" data-side-toggle aria-expanded="false" aria-controls="sidebar">Menu</button>
	<nav class="sidebar" id="sidebar" aria-label="Documentation">${sidebar}</nav>
	<main class="article" id="content">
		<p class="crumb">${escapeHtml(page.section)}</p>
		<article class="prose">${body}</article>
		<p class="edit"><a href="${repoUrl}/edit/main/docs/src/${page.source}" rel="noopener">Edit this page on GitHub</a></p>
		${pager}
	</main>
	${toc}
</div>
${searchDialog(base)}
</body>
</html>
`;
}

function redirectStub(target: string): string {
	return `<!doctype html><meta charset="utf-8"><title>Moved</title><meta http-equiv="refresh" content="0; url=${target}"><link rel="canonical" href="${target}"><a href="${target}">This page moved.</a>\n`;
}

async function readVersion(): Promise<string> {
	const cargo = await readFile(path.join(root, "Cargo.toml"), "utf8");
	const match = /^version\s*=\s*"([^"]+)"/m.exec(cargo);
	if (!match) {
		throw new Error("could not find the workspace version in Cargo.toml");
	}
	return match[1];
}

async function write(file: string, contents: string): Promise<void> {
	await mkdir(path.dirname(file), { recursive: true });
	await writeFile(file, contents);
}

async function main(): Promise<void> {
	const highlighter = await createHighlighter({
		themes: ["github-light", "vitesse-dark"],
		langs: languages,
	});
	const pages = parseSummary(
		await readFile(path.join(docsSource, "SUMMARY.md"), "utf8"),
	);
	const version = await readVersion();
	const search: SearchEntry[] = [];

	await rm(outDir, { recursive: true, force: true });
	await mkdir(path.join(outDir, "assets"), { recursive: true });

	for (const page of pages) {
		const raw = await readFile(path.join(docsSource, page.source), "utf8");
		// The introduction opens with a centred logo for GitHub; the site has its own.
		const markdown = raw.replace(/^<p align="center">.*<\/p>\s*/, "");
		const { html, headings } = renderMarkdown(markdown, page, highlighter);
		const text = plainText(markdown);
		const description = text.replace(new RegExp(`^${page.title}\\s*`), "")
			.slice(0, 155).replace(/\s\S*$/, "…");

		await write(
			path.join(outDir, page.output),
			renderDocPage(page, pages, html, headings, description),
		);
		await write(
			path.join(outDir, page.output.replace(/^docs\//, "book/")),
			redirectStub(`${siteUrl}/${page.output}`),
		);
		search.push({
			title: page.title,
			section: page.section,
			url: page.output,
			headings: headings.map((h) => h.text),
			text: text.slice(0, 4000),
		});
	}

	const landing = await readFile(path.join(staticDir, "index.html"), "utf8");
	await write(
		path.join(outDir, "index.html"),
		landing.replaceAll("{{version}}", version).replace(
			"{{search}}",
			searchDialog(""),
		),
	);
	await write(
		path.join(outDir, "404.html"),
		await readFile(path.join(staticDir, "404.html"), "utf8"),
	);
	await write(path.join(outDir, "assets/search.json"), JSON.stringify(search));

	for (const asset of ["style.css", "app.js", "og.png"]) {
		await cp(path.join(staticDir, asset), path.join(outDir, "assets", asset));
	}
	for (
		const asset of [
			"mdt-mark.svg",
			"mdt-mark-reversed.svg",
			"mdt-icon.svg",
			"mdt-demo.gif",
		]
	) {
		await cp(
			path.join(root, "assets", asset),
			path.join(outDir, "assets", asset),
		);
	}

	console.log(
		`built ${pages.length} docs pages (mdt ${version}) into ${
			path.relative(root, outDir) || "."
		}`,
	);
}

await main();
