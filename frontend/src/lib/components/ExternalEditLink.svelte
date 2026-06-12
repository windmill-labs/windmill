<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import type { Snippet } from 'svelte'

	// Shared "open in a new tab" link used by the compare/diff row titles
	// (CompareDrafts, CompareWorkspaces, WorkspaceDiffDrawer). Wraps the common
	// boilerplate — target/rel, the click-through stopPropagation (so following
	// the link doesn't toggle row selection), and the hover-revealed external
	// icon. Each caller supplies its own title text, extra classes, and inner
	// label content via the `children` snippet.
	let {
		href,
		title,
		class: klass = '',
		children
	}: {
		href: string
		title?: string
		class?: string
		children: Snippet
	} = $props()
</script>

<a
	{href}
	{title}
	target="_blank"
	rel="noopener noreferrer"
	onclick={(e) => e.stopPropagation()}
	class="group inline-flex items-center gap-1 max-w-full hover:underline {klass}"
>
	{@render children()}
	<ExternalLink class="w-3 h-3 shrink-0 opacity-0 group-hover:opacity-60 transition-opacity" />
</a>
