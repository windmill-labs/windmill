<script lang="ts">
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Button from '../button/Button.svelte'

	export let title: string | undefined = undefined
	export let overflow_y = true
	export let noPadding = false
	export let forceOverflowVisible = false

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col divide-y h-screen max-h-screen">
	<div class="flex justify-between w-wull items-center px-4 py-2">
		<div class="flex items-center gap-2">
			<Button
				size="lg"
				color="light"
				on:click={() => dispatch('close')}
				startIcon={{ icon: faClose }}
				iconOnly
			/>

			<span class="font-semibold truncate text-gray-800">{title}</span>
		</div>

		<div>
			<div class="flex gap-2 items-center">
				{#if $$slots.submission}
					<slot name="submission" class="sticky" />
				{/if}
			</div>
		</div>
	</div>

	<div
		class="{noPadding ? '' : 'p-4'} grow h-full max-h-full {forceOverflowVisible
			? '!overflow-visible'
			: ''}"
		class:overflow-y-auto={overflow_y}
	>
		<slot />
	</div>
</div>
