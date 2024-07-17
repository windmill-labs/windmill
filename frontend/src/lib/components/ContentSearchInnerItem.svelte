<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let title: string
	export let href: string

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col gap-2">
	<div class="flex flex-row items-center justify-between">
		<a
			class="text-sm text-blue-500 truncate hover:underline"
			{href}
			on:click={() => {
				dispatch('close')
			}}
		>
			{title}
		</a>

		<div class="flex flex-row gap-2 items-center">
			<slot name="actions" />
		</div>
	</div>
	<slot />
</div>
