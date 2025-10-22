<script lang="ts">
	import { ArrowLeft, ArrowRight } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { createEventDispatcher } from 'svelte'
	import Alert from '../common/alert/Alert.svelte'

	export let activeIndex: number | undefined = undefined
	export let totalSteps: number | undefined = undefined

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col gap-4 w-full pt-4">
	{#if activeIndex === 0}
		<Alert size="xs" title="Help">
			<li> UI is not interactive during tutorial, press next at every step </li>
			<li> You can use the arrow keys to navigate </li>
		</Alert>
	{/if}
	<div class="flex flex-row gap-2 justify-between w-full items-center">
		{#if activeIndex !== undefined && totalSteps !== undefined}
			<div class="text-xs">
				Step {activeIndex + 1} of {totalSteps}
			</div>
		{/if}
		<div class="flex flex-row gap-2">
			<Button
				size="xs2"
				color="light"
				startIcon={{ icon: ArrowLeft }}
				on:click={() => {
					dispatch('previous')
				}}
			>
				Previous
			</Button>
			<Button
				size="xs2"
				variant="accent"
				endIcon={{ icon: ArrowRight }}
				on:click={() => {
					dispatch('next')
				}}
			>
				Next
			</Button>
		</div>
	</div>
</div>
