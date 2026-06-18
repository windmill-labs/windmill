<script lang="ts">
	import { ConfigService, type AutoscalingEvent } from '$lib/gen'
	import { RefreshCw } from 'lucide-svelte'
	import { Button, Skeleton } from './common'
	import { twMerge } from 'tailwind-merge'
	import TimeAgo from './TimeAgo.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { untrack } from 'svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Cell from './table/Cell.svelte'

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

<div class="flex flex-col gap-2">
	<div class="flex flex-row items-center justify-between">
		<div class="flex flex-row items-baseline gap-2">
			<h3 class="text-xs font-semibold text-emphasis">Autoscaling events</h3>
			{#if $enterpriseLicense && events && events.length > 0}
				<span class="text-2xs text-secondary">Showing last {Math.min(limit, events.length)}</span>
			{/if}
		</div>
		{#if $enterpriseLicense}
			<Button
				startIcon={{
					icon: RefreshCw,
					classes: twMerge(loading ? 'animate-spin' : '')
				}}
				variant="subtle"
				unifiedSize="sm"
				on:click={() => loadEvents()}
				iconOnly
			/>
		{/if}
	</div>

	{#if !$enterpriseLicense}
		<div class="text-xs font-normal text-secondary">Autoscaling is an EE feature</div>
	{:else if loading}
		<Skeleton layout={[[12], 1]} />
	{:else if events}
		{#if events.length == 0}
			<div class="text-xs font-normal text-secondary">
				No events. Is autoscaling configured in the worker group config?
			</div>
		{:else}
			<DataTable size="sm" noBorder={false} rounded={true}>
				<Head>
					<tr>
						<Cell head first>Event type</Cell>
						<Cell head>Desired workers</Cell>
						<Cell head>Reason</Cell>
						<Cell head last>Time</Cell>
					</tr>
				</Head>
				<tbody>
					{#each events as event}
						<tr class="border-b last:border-b-0">
							<Cell first class="text-xs font-normal text-primary">{event.event_type ?? 'N/A'}</Cell
							>
							<Cell class="text-xs font-normal text-primary">{event.desired_workers}</Cell>
							<Cell class="text-xs font-normal text-secondary">{event.reason ?? 'N/A'}</Cell>
							<Cell last class="text-xs font-normal text-secondary">
								<TimeAgo date={event.applied_at ?? ''} />
							</Cell>
						</tr>
					{/each}
				</tbody>
			</DataTable>

			{#if events.length >= limit && limit < 100}
				<div class="flex">
					<Button variant="subtle" unifiedSize="sm" on:click={() => (limit = limit + 25)}>
						Show more
					</Button>
				</div>
			{/if}
		{/if}

		{#if limit > 50}
			<div class="text-2xs font-normal text-hint">
				Note: Autoscaling events are only stored for the last 30 days.
			</div>
		{/if}
	{/if}
</div>
