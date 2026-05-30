import type { Plugin } from 'svelte-exmarkdown'
import { gfmPlugin } from 'svelte-exmarkdown/gfm'
import rehypeRaw from 'rehype-raw'
import rehypeSanitize from 'rehype-sanitize'
import { rehypeGithubAlerts } from 'rehype-github-alerts'

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
 * Any Markdown sink that renders user input MUST use this chain rather than
 * assembling its own `rehypeRaw` pipeline.
 */
export const markdownPlugins: Plugin[] = [
	gfmPlugin(),
	{ rehypePlugin: [rehypeRaw] },
	{ rehypePlugin: [rehypeSanitize] },
	{ rehypePlugin: [rehypeGithubAlerts] }
]
