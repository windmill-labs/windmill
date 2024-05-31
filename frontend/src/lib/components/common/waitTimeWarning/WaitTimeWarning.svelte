<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { msToSec } from '$lib/utils'
	import { Hourglass } from 'lucide-svelte'
	import Badge from '../badge/Badge.svelte'

	export let waiting_time_ms: number
	export let variant: 'badge' | 'icon' = 'icon'

	function waitColorToClass(color: string): string | undefined {
		const colors: Record<string, string> = {
			gray: 'text-gray-400 dark:text-gray-300',
			red: 'text-red-400 dark:text-red-500',
			yellow: 'text-yellow-500 dark:text-yellow-500',
			orange: 'text-orange-500 dark:text-orange-500'
		}

		return colors[color]
	}

	function waitTimeWarnColor(waiting_time_ms: number): any {
		if (waiting_time_ms > 300_000) {
			return 'red'
		}
		if (waiting_time_ms > 60_000) {
			return 'orange'
		}
		if (waiting_time_ms > 5_000) {
			return 'yellow'
		}
		return 'gray'
	}
</script>

<Popover notClickable>
	<svete:frament slot="text">
		This job had an aggregate queue time of {msToSec(waiting_time_ms)}s
		<Tooltip>
			<div>
				This means the job and its tasks (e.g., flow steps) cumulatively spent a significant amount
				of time waiting for an executor.
			</div>
		</Tooltip>
	</svete:frament>
	{#if variant === 'icon'}
		<Hourglass class={waitColorToClass(waitTimeWarnColor(waiting_time_ms))} size={14} />
	{:else}
		<Badge
			large
			icon={{ icon: Hourglass, position: 'left' }}
			color={waitTimeWarnColor(waiting_time_ms)}
		>{msToSec(waiting_time_ms)}s</Badge>
	{/if}
</Popover>
