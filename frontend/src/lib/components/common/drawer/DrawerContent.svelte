<script lang="ts">
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import Button from '../button/Button.svelte'

	export let title: string | undefined = undefined
	export let overflow_y = true
	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col divide-y h-screen max-h-screen">
	<div class="flex justify-between items-center py-2 px-6">
		<Button
			variant="border"
			size="lg"
			color="dark"
			on:click={() => {
				dispatch('close')
			}}
		>
			<Icon data={faClose} />
		</Button>
		<span class="font-bold">{title}</span>
		<div class="flex flex-row">
			{#if $$slots.submission}
				<slot name="submission" class="sticky" />
			{/if}
		</div>
	</div>

	<div class="py-2 px-6 grow h-full max-h-full" class:overflow-y-auto={overflow_y}>
		<slot />
	</div>
</div>
