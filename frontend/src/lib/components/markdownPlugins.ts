import type { Plugin } from 'svelte-exmarkdown'
import { gfmPlugin } from 'svelte-exmarkdown/gfm'
import rehypeRaw from 'rehype-raw'
import rehypeSanitize from 'rehype-sanitize'
import { rehypeGithubAlerts } from 'rehype-github-alerts'
import MarkdownCodeBlock from './MarkdownCodeBlock.svelte'

/**
 * Shared plugin chain for rendering user-supplied Markdown (script/flow/resource
 * descriptions, flow-graph notes, the App "Markdown" component, ...).
 *
 * Order matters and is security-sensitive:
 *  1. `gfmPlugin`          — GitHub-flavored Markdown.
 *  2. `rehypeRaw`          — re-parses embedded raw HTML into live hast nodes.
 *  3. `rehypeSanitize`     — strips dangerous nodes (`<script>`, `<iframe>`,
 *                            `<svg><script>`, `on*` handlers, `javascript:` URLs)
 *                            from the raw-parsed tree. Without this stage the raw
 *                            HTML reaches the DOM and executes (stored XSS).
 *  4. `rehypeGithubAlerts` — runs AFTER sanitize so the trusted alert markup it
 *                            injects (including inline SVG icons) is preserved
 *                            without having to allowlist SVG for user input.
 *
 * The `pre` renderer gives fenced code blocks syntax highlighting, a copy
 * button, and a subtle scrollbar everywhere this chain is used. It reads the
 * `language-*` class rehypeSanitize preserves on `<code>`; it highlights code as
 * escaped text and never renders mermaid on untrusted input (chat opts in).
 *
 * Any Markdown sink that renders user input MUST use this chain rather than
 * assembling its own `rehypeRaw` pipeline.
 */
export const markdownPlugins: Plugin[] = [
	gfmPlugin(),
	{ rehypePlugin: [rehypeRaw] },
	{ rehypePlugin: [rehypeSanitize] },
	{ rehypePlugin: [rehypeGithubAlerts] },
	{ renderer: { pre: MarkdownCodeBlock } }
]
