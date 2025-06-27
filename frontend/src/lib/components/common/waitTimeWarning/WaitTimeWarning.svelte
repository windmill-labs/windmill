<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import { msToSec } from '$lib/utils'
	import { AlertTriangle, Hourglass } from 'lucide-svelte'
	import Badge from '../badge/Badge.svelte'

	interface Props {
		self_wait_time_ms?: number | undefined
		aggregate_wait_time_ms?: number | undefined
		variant?: 'icon' | 'alert' | 'badge' | 'badge-self-wait'
	}

	let {
		self_wait_time_ms = undefined,
		aggregate_wait_time_ms = undefined,
		variant = 'icon'
	}: Props = $props()

	let total_wait = $derived((self_wait_time_ms ?? 0) + (aggregate_wait_time_ms ?? 0))

	function classFromColorName(color: string): string | undefined {
		const colors: Record<string, string> = {
			gray: 'text-gray-400 dark:text-gray-300',
			red: 'text-red-400 dark:text-red-500',
			yellow: 'text-yellow-500 dark:text-yellow-500',
			orange: 'text-orange-500 dark:text-orange-500'
		}

		return colors[color]
	}

	function waitColorTresholds(waiting_time_ms: number): any {
		if (waiting_time_ms > 300_000) {
			return 'red'
		}
		if (waiting_time_ms > 30_000) {
			return 'orange'
		}
		if (waiting_time_ms > 5_000) {
			return 'yellow'
		}
		return 'gray'
	}
</script>

<Popover notClickable>
	{#snippet text()}
		<div class="mb-5">
			{#if self_wait_time_ms != undefined}
				<div>
					Time spent waiting for an executor: <span class="font-bold"
						>{msToSec(self_wait_time_ms)}s</span
					>
				</div>
			{/if}
			{#if aggregate_wait_time_ms != undefined}
				<div>
					Child jobs' time spent waiting for an executor: <span class="font-bold"
						>{msToSec(aggregate_wait_time_ms)}s</span
					>
				</div>
			{/if}
		</div>
		{#if self_wait_time_ms != undefined && aggregate_wait_time_ms != undefined}
			The top level job and its children (e.g. flow steps) had to wait a for an unexpected amount of
			time before starting. The first value is the top level job's time spent waiting for a worker
			and the second is the cumulative wait time for its children.
		{:else if self_wait_time_ms}
			<div> This job spent an unexpected amount of time waiting for a worker before starting. </div>
		{:else if aggregate_wait_time_ms != undefined}
			<div>
				This job's children spent an unexpected amount of time waiting for a worker before starting.
				The value is an aggregate of their individual waiting times.
			</div>
		{/if}
		<div> In a healthy queue, jobs are expected to start in under 50ms. </div>
	{/snippet}
	{#if variant === 'icon'}
		<Hourglass class={classFromColorName(waitColorTresholds(total_wait))} size={14} />
	{:else if variant === 'badge'}
		<Badge large icon={{ icon: Hourglass, position: 'left' }} color={waitColorTresholds(total_wait)}
			>{msToSec(total_wait)}s</Badge
		>
	{:else if variant === 'badge-self-wait'}
		{#if self_wait_time_ms}
			<Badge color={waitColorTresholds(self_wait_time_ms)}>+{msToSec(self_wait_time_ms)}s</Badge>
		{/if}
	{:else if variant === 'alert'}
		<AlertTriangle class={classFromColorName(waitColorTresholds(total_wait))} size={14} />
	{/if}
</Popover>
