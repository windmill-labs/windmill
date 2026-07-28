<script lang="ts">
	import type { Snippet } from 'svelte'

	// Subtle horizontal scroll: native overflow (so wheel/trackpad/drag always
	// work) with the shared `.scrollbar-subtle` thumb (thin, hover-revealed). Bar
	// thickness is tunable via the `--wm-scrollbar-size` CSS var (pass through
	// `style`), so denser callers (e.g. the tab strip) can shrink it.
	let {
		class: c = '',
		style = '',
		children
	}: { class?: string; style?: string; children: Snippet } = $props()
</script>

<div class="overflow-x-auto overflow-y-hidden scrollbar-subtle {c}" {style}>
	{@render children()}
</div>
