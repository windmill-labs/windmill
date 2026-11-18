<script lang="ts">
	import { ArrowLeft, ArrowRight } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import Alert from '../common/alert/Alert.svelte'

	interface Props {
		activeIndex: number
		totalSteps: number
		onPrevious: () => void
		onNext: () => void
	}

	let { activeIndex, totalSteps, onPrevious, onNext }: Props = $props()
</script>

<div class="flex flex-col gap-4 w-full pt-4">
	{#if activeIndex === 0}
		<Alert size="xs" title="Help">
			<li>UI is not interactive during the tour, press next at every step</li>
			<li>You can use the arrow keys to navigate</li>
		</Alert>
	{/if}
	<div class="flex flex-row gap-2 justify-between w-full items-center">
		<div class="text-xs">
			Step {activeIndex + 1} of {totalSteps}
		</div>
		<div class="flex flex-row gap-2">
			<Button
				unifiedSize="xs"
				variant="default"
				startIcon={{ icon: ArrowLeft }}
				onclick={onPrevious}
			>
				Previous
			</Button>
			<Button unifiedSize="xs" variant="accent" endIcon={{ icon: ArrowRight }} onclick={onNext}>
				Next
			</Button>
		</div>
	</div>
</div>
