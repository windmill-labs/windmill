<script lang="ts">
	import type { Snippet } from 'svelte'

	// Subtle horizontal scroll: native overflow (so wheel/trackpad/drag always
	// work) with a thin, rounded scrollbar that stays invisible until the region
	// is hovered. Bar thickness is tunable via the `--wm-scrollx-size` CSS var
	// (pass through `style`), so denser callers (e.g. the tab strip) can shrink it.
	let {
		class: c = '',
		style = '',
		children
	}: { class?: string; style?: string; children: Snippet } = $props()
</script>

<div class="wm-scrollx {c}" {style}>
	{@render children()}
</div>

<style>
	.wm-scrollx {
		overflow-x: auto;
		overflow-y: hidden;
		/* Firefox: thin bar, transparent until hover. */
		scrollbar-width: thin;
		scrollbar-color: transparent transparent;
	}
	.wm-scrollx:hover {
		scrollbar-color: rgb(var(--color-text-hint) / 0.4) transparent;
	}

	/* WebKit: override the app-wide `*::-webkit-scrollbar` bar with a subtle,
	   hover-revealed thumb (higher specificity than the global `*` rule). */
	.wm-scrollx::-webkit-scrollbar {
		height: var(--wm-scrollx-size, 6px);
	}
	.wm-scrollx::-webkit-scrollbar-track {
		background: transparent;
	}
	.wm-scrollx::-webkit-scrollbar-thumb {
		background: transparent;
		border-radius: 9999px;
		transition: background-color 0.15s;
	}
	.wm-scrollx:hover::-webkit-scrollbar-thumb {
		background: rgb(var(--color-text-hint) / 0.35);
	}
	.wm-scrollx:hover::-webkit-scrollbar-thumb:hover {
		background: rgb(var(--color-text-secondary) / 0.55);
	}
</style>
