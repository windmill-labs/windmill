<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import type { AuditLog } from '$lib/gen'
	import { displayDate } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Button from '../common/button/Button.svelte'
	import { ListFilter } from 'lucide-svelte'

	export let logs: AuditLog[] = []
	export let pageIndex: number | undefined = 1
	export let perPage: number | undefined = 100
	export let hasMore: boolean = true
	export let actionKind: string | undefined = undefined
	export let operation: string | undefined = undefined
	export let selectedId: number | undefined = undefined
	export let usernameFilter: string | undefined = undefined
	export let resourceFilter: string | undefined = undefined

	function groupLogsByDay(logs: AuditLog[]): Record<string, AuditLog[]> {
		const groupedLogs = {}

		if (!logs) return groupedLogs

		for (const log of logs) {
			const date = new Date(log.timestamp)
			const key = date.toLocaleString('en-US', {
				day: 'numeric',
				month: 'long',
				year: 'numeric'
			})

			if (!groupedLogs[key]) {
				groupedLogs[key] = []
			}

			groupedLogs[key].push(log)
		}
		return groupedLogs
	}

	const dispatch = createEventDispatcher()
	$: groupedLogs = groupLogsByDay(logs)

	function kindToBadgeColor(kind: string) {
		if (kind == 'Execute') {
			return 'blue'
		} else if (kind == 'Delete') {
			return 'red'
		} else if (kind == 'Update') {
			return 'yellow'
		} else if (kind == 'Create') {
			return 'green'
		}
		return 'gray'
	}
</script>

<!-- <VirtualList -->
<!-- 	width="100%" -->
<!-- 	height={tableHeight - headerHeight} -->
<!-- 	itemCount={flatJobs?.length ?? 3} -->
<!-- 	itemSize={42} -->
<!-- 	overscanCount={20} -->
<!-- 	{stickyIndices} -->
<!-- 	{scrollToIndex} -->
<!-- 	scrollToAlignment="center" -->
<!-- 	scrollToBehaviour="smooth" -->
<!-- ></VirtualList> -->

<DataTable
	on:next={() => {
		pageIndex = (pageIndex ?? 1) + 1
	}}
	on:previous={() => {
		pageIndex = (pageIndex ?? 1) - 1
	}}
	currentPage={pageIndex}
	paginated
	rounded={false}
	size="sm"
	{hasMore}
	bind:perPage
>
	<Head>
		<Cell first head>ID</Cell>
		<Cell head>Timestamp</Cell>
		<Cell head>Username</Cell>
		<Cell head>Operation</Cell>
		<Cell head last>Resource</Cell>
	</Head>
	{#if logs?.length > 0}
		<tbody class="divide-y">
			{#each Object.entries(groupedLogs) as [date, logsByDay]}
				<tr class="border-t">
					<Cell
						first
						colspan="6"
						scope="colgroup"
						class="bg-surface-secondary/30 py-2 border-b font-semibold"
					>
						{date}
					</Cell>
				</tr>
				{#each logsByDay as { id, timestamp, username, operation: op, action_kind, resource, parameters }}
					<Row
						hoverable
						selected={id === selectedId}
						on:click={() => {
							dispatch('select', id)
						}}
					>
						<Cell first>
							{id}
						</Cell>
						<Cell>
							{displayDate(timestamp)}
						</Cell>
						<Cell>
							<div class="flex flex-row gap-2 items-center">
								<div class="whitespace-nowrap overflow-x-auto no-scrollbar max-w-52">
									{username}
									{#if parameters && 'end_user' in parameters}
										<span> ({parameters.end_user})</span>
									{/if}
								</div>
								<Button
									color="light"
									size="xs2"
									iconOnly
									startIcon={{ icon: ListFilter }}
									on:click={() => {
										usernameFilter = username
									}}
								/>
							</div>
						</Cell>
						<Cell>
							<div class="flex flex-row gap-1">
								<Badge
									on:click={() => {
										actionKind = action_kind.toLocaleLowerCase()
									}}
									color={kindToBadgeColor(action_kind)}>{action_kind}</Badge
								>
								<Badge
									on:click={() => {
										operation = op
									}}
								>
									{op}
								</Badge>
							</div>
						</Cell>
						<Cell last>
							<div class="flex flex-row gap-2 items-center">
								<div class="whitespace-nowrap overflow-x-auto no-scrollbar w-48">
									{resource}
								</div>
								<Button
									color="light"
									size="xs2"
									iconOnly
									startIcon={{ icon: ListFilter }}
									on:click={() => {
										resourceFilter = resource
									}}
								/>
							</div>
						</Cell>
					</Row>
				{/each}
			{/each}
		</tbody>
	{:else}
		<tr>
			<td colspan="4" class="text-center py-8">
				<div class="text-xs text-secondary"> No logs found for the selected filters. </div>
			</td>
		</tr>
	{/if}
</DataTable>

<style lang="postcss">
	/* Hide scrollbar for Chrome, Safari and Opera */
	.no-scrollbar::-webkit-scrollbar {
		display: none;
	}

	/* Hide scrollbar for IE, Edge and Firefox */
	.no-scrollbar {
		-ms-overflow-style: none; /* IE and Edge */
		scrollbar-width: none; /* Firefox */
	}
</style>
