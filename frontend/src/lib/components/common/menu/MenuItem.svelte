<script lang="ts">
	import { createBubbler } from 'svelte/legacy';

	const bubble = createBubbler();
	interface Props {
		children?: import('svelte').Snippet;
	}

	let { children }: Props = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div onclick={bubble('click')} class="py-1">
	<span
		class="text-secondary block px-4 py-2 text-sm hover:bg-surface-hover hover:text-primary"
		role="menuitem"
		tabindex="-1"
	>
		{@render children?.()}
	</span>
</div>
