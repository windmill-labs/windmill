<script lang="ts">
	import type { Snippet } from 'svelte'
	import { fade } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		/** Re-render and crossfade whenever this changes — usually the selected tab. */
		key: unknown
		class?: string
		children: Snippet
	}

	let { key, class: clazz = '', children }: Props = $props()
</script>

<!-- Both panels share one grid cell while the crossfade runs: stacked in normal flow
	 the outgoing one would push the incoming one down for the length of the fade. -->
<div class={twMerge('grid [&>*]:col-start-1 [&>*]:row-start-1', clazz)}>
	{#key key}
		<!-- min-w-0: a grid item defaults to min-width:auto, which lets a wide table push
			 past the container instead of scrolling inside its own overflow-x-auto. -->
		<div class="min-w-0" in:fade={{ duration: 90, delay: 50 }} out:fade={{ duration: 50 }}>
			{@render children()}
		</div>
	{/key}
</div>
