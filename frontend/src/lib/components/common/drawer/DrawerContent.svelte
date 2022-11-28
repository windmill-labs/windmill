<script lang="ts">
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import Button from '../button/Button.svelte'

	export let title: string | undefined = undefined
	export let overflow_y = true
	export let noPadding = false
	export let forceOverflowVisible = false
	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col divide-y h-screen max-h-screen">
	<div class="flex justify-between items-center gap-x-2">
		<Button
			btnClasses="m-1"
			variant="border"
			size="lg"
			color="dark"
			on:click={() => {
				dispatch('close')
			}}
		>
			<Icon data={faClose} />
		</Button>
		<span class="font-bold truncate">{title}</span>
		<div class="flex flex-row m-1">
			{#if $$slots.submission}
				<slot name="submission" class="sticky" />
			{/if}
		</div>
	</div>

	<div
		class="{noPadding ? '' : 'py-2 px-6'} grow h-full max-h-full {forceOverflowVisible
			? '!overflow-visible'
			: ''}"
		class:overflow-y-auto={overflow_y}
	>
		<slot />
	</div>
</div>
