<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	interface Props {
		title: string
		href: string
		actions?: import('svelte').Snippet
		children?: import('svelte').Snippet
		onclose?: (...args: any[]) => any
	}

	let { title, href, actions, children, onclose = undefined }: Props = $props()

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col gap-2">
	<div class="flex flex-row items-center justify-between">
		<a
			class="text-accent truncate text-xs"
			{href}
			onclick={() => {
				dispatch('close')
				onclose?.()
			}}
		>
			{title}
		</a>

		<div class="flex flex-row gap-2 items-center">
			{@render actions?.()}
		</div>
	</div>
	{@render children?.()}
</div>
