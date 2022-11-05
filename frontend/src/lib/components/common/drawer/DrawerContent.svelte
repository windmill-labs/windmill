<script lang="ts">
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'

	export let title: string | undefined = undefined
	export let overflow_y = true
	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col divide-y h-screen max-h-screen">
	<div class="flex justify-between items-center py-2 px-4">
		<span class="text-sm font-bold">{title}</span>
		<button on:click={() => dispatch('close')}>
			<Icon data={faClose} class="w-4 h-4" />
		</button>
	</div>

	<div class="p-2 grow h-full max-h-full" class:overflow-y-auto={overflow_y}>
		<slot />
	</div>
	{#if $$slots.submission}
		<div class="flex flex-col bg-white border-gray-200 p-2">
			<div class="flex flex-row-reverse p-2 ">
				<slot name="submission" class="sticky" />
			</div>
		</div>
	{/if}
</div>
