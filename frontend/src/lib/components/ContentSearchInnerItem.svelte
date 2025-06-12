<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	interface Props {
		title: string
		href: string
		actions?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let { title, href, actions, children }: Props = $props()

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col gap-2">
	<div class="flex flex-row items-center justify-between">
		<a
			class="text-sm text-blue-500 truncate hover:underline"
			{href}
			onclick={() => {
				dispatch('close')
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
