<script lang="ts">
	import { createPopover, createSync, melt } from '@melt-ui/svelte'
	import { fade } from 'svelte/transition'
	import { X } from 'lucide-svelte'

	export let open = false
	export let closeButton: boolean = true

	const {
		elements: { trigger, content, arrow, close },
		states
	} = createPopover({
		forceVisible: true
	})

	const sync = createSync(states)
	$: sync.open(open, (v) => (open = v))
</script>

<button type="button" use:melt={$trigger} aria-label="Popup button">
	<slot name="trigger" />
</button>

{#if open}
	<div use:melt={$content} transition:fade={{ duration: 100 }} class="content">
		<div use:melt={$arrow} />
		<slot name="content" />
		{#if closeButton}
			<button class="close" use:melt={$close}>
				<X class="size-3" />
			</button>
		{/if}
	</div>
{/if}

<style lang="postcss">
	.close {
		@apply absolute right-1.5 top-1.5 flex h-7 w-7 items-center justify-center rounded-full;
		@apply text-primary  transition-colors hover:bg-surface-hover;
		@apply focus-visible:ring focus-visible:ring-gray-400 focus-visible:ring-offset-2;
		@apply bg-surface p-0 text-sm font-medium;
	}

	.content {
		@apply z-10 w-fit rounded-[4px] bg-surface p-0 overflow-hidden shadow-md;
	}
</style>
