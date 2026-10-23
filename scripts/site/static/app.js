// mdt website behaviour: theme, copy buttons, search, docs navigation, and the
// landing-page sync playground. No dependencies.

const root = document.documentElement;

function copyText(button, text) {
	void navigator.clipboard.writeText(text).then(() => {
		button.dataset.copied = "";
		setTimeout(() => delete button.dataset.copied, 1600);
	});
}

// ─── theme ─────────────────────────────────────────────────

function currentTheme() {
	if (root.dataset.theme) return root.dataset.theme;
	return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

for (const button of document.querySelectorAll("[data-theme-toggle]")) {
	button.addEventListener("click", () => {
		const next = currentTheme() === "dark" ? "light" : "dark";
		root.dataset.theme = next;
		try {
			localStorage.setItem("mdt-theme", next);
		} catch {
			// Storage can be unavailable (private mode); the toggle still works for this page.
		}
	});
}

// ─── copy buttons ──────────────────────────────────────────

for (const button of document.querySelectorAll("[data-copy]")) {
	button.addEventListener("click", () => copyText(button, button.dataset.copy));
}

for (const button of document.querySelectorAll(".code .copy")) {
	button.addEventListener("click", () => {
		const code = button.parentElement.querySelector("pre code");
		copyText(button, code.innerText);
		button.textContent = "Copied";
		setTimeout(() => (button.textContent = "Copy"), 1600);
	});
}

// ─── docs sidebar and table of contents ────────────────────

const sideToggle = document.querySelector("[data-side-toggle]");
if (sideToggle) {
	const sidebar = document.getElementById("sidebar");
	sideToggle.addEventListener("click", () => {
		const open = sidebar.classList.toggle("open");
		sideToggle.setAttribute("aria-expanded", String(open));
	});
}

// Keep the current page visible in a long sidebar without scrolling the window.
const current = document.querySelector(".sidebar a[aria-current]");
if (current) {
	const sidebar = current.closest(".sidebar");
	sidebar.scrollTop = current.offsetTop - sidebar.clientHeight / 2;
}

const tocLinks = [...document.querySelectorAll(".toc a")];
if (tocLinks.length) {
	const byId = new Map(
		tocLinks.map((link) => [decodeURIComponent(link.hash.slice(1)), link]),
	);
	const observer = new IntersectionObserver(
		(entries) => {
			const visible = entries.filter((entry) => entry.isIntersecting);
			if (!visible.length) return;
			for (const link of tocLinks) link.classList.remove("active");
			byId.get(visible[0].target.id)?.classList.add("active");
		},
		{ rootMargin: "-70px 0px -70% 0px" },
	);
	for (const id of byId.keys()) {
		const heading = document.getElementById(id);
		if (heading) observer.observe(heading);
	}
}

// ─── search ────────────────────────────────────────────────

const dialog = document.querySelector("[data-search]");
if (dialog) {
	const input = dialog.querySelector("[data-search-input]");
	const results = dialog.querySelector("[data-search-results]");
	const base = dialog.dataset.base;
	let index = null;
	let selected = 0;

	const escape = (text) =>
		text.replace(
			/[&<>"]/g,
			(c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c],
		);
	const highlight = (text, terms) => {
		let html = escape(text);
		for (const term of terms) {
			html = html.replace(
				new RegExp(`(${term.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi"),
				"<mark>$1</mark>",
			);
		}
		return html;
	};

	async function open() {
		dialog.showModal();
		input.select();
		if (!index) {
			const response = await fetch(`${base}assets/search.json`);
			index = await response.json();
		}
		render();
	}

	function snippet(text, term) {
		const at = text.toLowerCase().indexOf(term);
		if (at < 0) return text.slice(0, 120);
		const start = Math.max(0, at - 40);
		return (start > 0 ? "…" : "") + text.slice(start, start + 140);
	}

	function render() {
		const query = input.value.trim().toLowerCase();
		const terms = query.split(/\s+/).filter(Boolean);
		let matches;

		if (!terms.length) {
			matches = index.slice(0, 8).map((page) => ({
				page,
				text: page.text.slice(0, 120),
			}));
		} else {
			matches = index
				.map((page) => {
					let score = 0;
					const title = page.title.toLowerCase();
					const headings = page.headings.join(" ").toLowerCase();
					const text = page.text.toLowerCase();
					for (const term of terms) {
						if (title.includes(term)) score += 10;
						if (headings.includes(term)) score += 4;
						if (text.includes(term)) {
							score += 1 + Math.min(text.split(term).length - 1, 6) * 0.3;
						} else if (!title.includes(term) && !headings.includes(term)) {
							return null;
						}
					}
					return { page, score, text: snippet(page.text, terms[0]) };
				})
				.filter(Boolean)
				.sort((a, b) => b.score - a.score)
				.slice(0, 12);
		}

		selected = 0;
		if (!matches.length) {
			results.innerHTML = `<li class="search-empty">Nothing matches “${
				escape(query)
			}”.</li>`;
			return;
		}
		results.innerHTML = matches
			.map(
				({ page, text }, i) =>
					`<li><a href="${base}${page.url}"${
						i === 0 ? ' aria-selected="true"' : ""
					}><small>${escape(page.section)}</small><strong>${
						highlight(page.title, terms)
					}</strong><p>${highlight(text, terms)}</p></a></li>`,
			)
			.join("");
	}

	function move(step) {
		const links = [...results.querySelectorAll("a")];
		if (!links.length) return;
		links[selected].removeAttribute("aria-selected");
		selected = (selected + step + links.length) % links.length;
		links[selected].setAttribute("aria-selected", "true");
		links[selected].scrollIntoView({ block: "nearest" });
	}

	input.addEventListener("input", render);
	input.addEventListener("keydown", (event) => {
		if (event.key === "ArrowDown") move(1);
		else if (event.key === "ArrowUp") move(-1);
		else if (event.key === "Enter") {
			event.preventDefault();
			results.querySelectorAll("a")[selected]?.click();
			return;
		} else return;
		event.preventDefault();
	});
	dialog.addEventListener("click", (event) => {
		if (event.target === dialog) dialog.close();
	});
	for (const trigger of document.querySelectorAll("[data-search-open]")) {
		trigger.addEventListener("click", open);
	}
	document.addEventListener("keydown", (event) => {
		const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(
			document.activeElement?.tagName ?? "",
		);
		if (
			(event.key === "/" && !typing) ||
			(event.key === "k" && (event.metaKey || event.ctrlKey))
		) {
			event.preventDefault();
			void open();
		}
	});
}

// ─── language tabs ─────────────────────────────────────────

const langs = document.querySelector("[data-langs]");
if (langs) {
	const tabs = [...langs.querySelectorAll("[data-lang]")];
	for (const tab of tabs) {
		tab.addEventListener("click", () => {
			for (const other of tabs) {
				other.setAttribute("aria-selected", String(other === tab));
			}
			for (const panel of langs.querySelectorAll("[data-lang-panel]")) {
				panel.hidden = panel.dataset.langPanel !== tab.dataset.lang;
			}
		});
	}
}

// ─── playground: a provider and three consumers ────────────

const playground = document.querySelector("[data-playground]");
if (playground) {
	const source = playground.querySelector("[data-source]");
	const status = playground.querySelector("[data-status]");
	const syncButton = playground.querySelector("[data-sync]");
	const copies = [...playground.querySelectorAll("[data-copy-kind]")];

	// The same transformers the consumer tags show, applied to the provider text.
	const transforms = {
		markdown: (text) => text,
		rust: (text) =>
			text.trim().split("\n").map((line) => (line ? `//! ${line}` : "//!"))
				.join("\n"),
		fence: (text) => "```text\n" + text.trim() + "\n```",
	};

	let synced = source.value;

	function paint(stamp) {
		const drift = source.value !== synced;
		playground.dataset.state = drift ? "drift" : "synced";
		status.textContent = drift
			? `Check failed. ${copies.length} consumer block(s) are out of date. Run \`mdt update\`.`
			: "Check passed: all consumer blocks are up to date.";

		copies.forEach((copy, i) => {
			copy.querySelector("[data-body]").textContent = transforms
				[copy.dataset.copyKind](synced);
			copy.classList.toggle("stale", drift);
			if (!stamp) return;
			copy.classList.remove("stamped");
			setTimeout(() => {
				copy.classList.add("stamped");
				setTimeout(() => copy.classList.remove("stamped"), 900);
			}, i * 110);
		});
	}

	source.addEventListener("input", () => paint(false));
	syncButton.addEventListener("click", () => {
		const changed = source.value !== synced;
		synced = source.value;
		paint(changed);
	});
	source.addEventListener("keydown", (event) => {
		if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
			syncButton.click();
		}
	});

	paint(false);
}
