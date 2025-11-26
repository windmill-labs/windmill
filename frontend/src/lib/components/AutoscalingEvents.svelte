<script lang="ts">
	import { ConfigService, type AutoscalingEvent } from '$lib/gen'
	import { LoaderIcon, RefreshCw } from 'lucide-svelte'
	import { Button, Skeleton } from './common'
	import { twMerge } from 'tailwind-merge'
	import TimeAgo from './TimeAgo.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { untrack } from 'svelte'

	interface Props {
		worker_group: string
	}

	let { worker_group }: Props = $props()

	let loading = $state(true)
	let events: AutoscalingEvent[] | undefined = $state(undefined)
	let limit = $state(5)

	async function loadEvents() {
		loading = true
		try {
			events = await ConfigService.listAutoscalingEvents({
				workerGroup: worker_group,
				perPage: limit
			})
		} catch (e) {
			events = []
			console.error(e)
		} finally {
			loading = false
		}
	}
	$effect(() => {
		limit
		worker_group && untrack(() => loadEvents())
	})
</script>

<div>
	<h6
		class={!$enterpriseLicense || (events != undefined && events.length == 0)
			? 'text-xs text-emphasis font-semibold'
			: ''}
		>Autoscaling events {#if $enterpriseLicense}<span class="text-xs text-primary">(5 last)</span>
			<span class="inline-flex ml-6">
				<Button
					startIcon={{
						icon: loading ? LoaderIcon : RefreshCw,
						classes: twMerge(
							loading ? 'animate-spin text-blue-800' : '',
							'transition-all text-gray-500 dark:text-white'
						)
					}}
					color="light"
					size="xs2"
					btnClasses={twMerge(loading ? ' bg-blue-100 dark:bg-blue-400' : '', 'transition-all')}
					on:click={() => loadEvents()}
					iconOnly
				/>
			</span>{/if}
	</h6>
	{#if !$enterpriseLicense}
		<div class="text-xs pt-1 text-secondary">Autoscaling is an EE feature</div>
	{:else if loading}
		<Skeleton layout={[[12], 1]} />
	{:else if events}
		{#if events.length == 0}
			<div class="text-xs pt-2 text-primary"
				>No events, is autoscaling set in the worker group config?</div
			>
		{:else}
			<div class="flex flex-col gap-2 text-xs text-primary pt-4">
				{#each events as event}
					<div class="flex flex-row gap-4">
						<div class="text-primary">{event.event_type} to {event.desired_workers}</div>
						<div class="text-secondary">{event.reason}</div>
						<div class="text-primary"><TimeAgo date={event.applied_at ?? ''} /></div>
					</div>
				{/each}
			</div>
			<div class="mt-4 flex">
				<Button color="light" size="xs2" on:click={() => (limit = limit + 25)}>Show more</Button>
			</div>
		{/if}

		{#if limit > 50}
			<div class="mt-4 flex text-xs text-primary">
				Note that autoscaling events are only stored for the last 30 days.
			</div>
		{/if}
	{/if}
</div>
