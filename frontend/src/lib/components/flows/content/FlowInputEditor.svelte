<script lang="ts">
	import { createEventDispatcher, onDestroy } from 'svelte'

	const dispatch = createEventDispatcher()

	interface Props {
		title?: string
		action?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let { title = '', action, children }: Props = $props()

	onDestroy(() => {
		dispatch('destroy')
	})
</script>

<div class="flex flex-col h-full px-3 pb-3">
	<div
		class="items-center grow-0 flex flex-row justify-between gap-2 data-schema-picker min-h-[40px]"
	>
		<h3 class="font-semibold text-emphasis text-xs flex flex-row items-center gap-1 leading-6">
			{title}
		</h3>
		{@render action?.()}
	</div>

	<div class="w-full min-h-0 grow">
		{@render children?.()}
	</div>
</div>
