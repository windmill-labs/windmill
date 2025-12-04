<script lang="ts">
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AlertCircle, CheckCircle2 } from 'lucide-svelte'
	import { devopsRole } from '$lib/stores'
	import List from '$lib/components/common/layout/List.svelte'
	import { Skeleton } from '../common'
	import Linkify from './Linkify.svelte'

	export let alerts: any[]
	export let hideAcknowledged = false
	export let goToNextPage: () => void
	export let goToPreviousPage: () => void
	export let acknowledgeAlert: (id: number) => void
	export let acknowledgeAll: () => void
	export let numUnacknowledgedCriticalAlerts: number
	export let page = 1
	export let hasMore = true
	export let pageSize = 0

	let headerHeight = 0
	let contentHeight = 0

	function formatDate(dateString: string | undefined): string {
		if (!dateString) return ''
		const date = new Date(dateString)
		return new Intl.DateTimeFormat('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		}).format(date)
	}

	$: availableHeight = (contentHeight - headerHeight - pageSize - 1) / pageSize
</script>

<div class="relative grow min-h-0 w-full">
	<DataTable
		size="xs"
		paginated
		on:next={goToNextPage}
		on:previous={goToPreviousPage}
		bind:currentPage={page}
		{hasMore}
		bind:contentHeight
	>
		<Head>
			<tr bind:clientHeight={headerHeight}>
				<Cell head first class="min-w-12">&nbsp;</Cell>
				<Cell head class="min-w-24 w-full">Message</Cell>
				<Cell head class="min-w-28">Created At</Cell>
				{#if $devopsRole}
					<Cell head class="min-w-24">Context</Cell>
				{/if}
				<Cell head last class="min-w-36">
					<List horizontal gap="sm">
						<span>Acked</span>

						<Button
							variant="accent"
							startIcon={{ icon: CheckCircle2 }}
							unifiedSize="sm"
							disabled={numUnacknowledgedCriticalAlerts === 0}
							on:click={acknowledgeAll}
							title="Acknowledge all"
						>
							All</Button
						>
					</List>
				</Cell>
			</tr>
		</Head>
		{#if alerts == undefined}
			<tbody>
				{#each new Array(3) as _}
					<Row>
						{#each new Array(5) as _}
							<Cell>
								<Skeleton layout={[[5]]} />
							</Cell>
						{/each}
					</Row>
				{/each}
			</tbody>
		{:else if alerts.length === 0}
			<div class="absolute top-0 left-0 w-full h-full center-center">
				<p class="text-center text-gray-500 mt-4">No critical alerts.</p>
			</div>
		{:else}
			<tbody class="divide-y border-b w-full overflow-y-auto">
				{#each alerts as { id, alert_type, message, created_at, acknowledged, workspace_id }}
					{#if !hideAcknowledged || !acknowledged}
						<Row disabled={acknowledged}>
							<Cell class="py-0">
								<div class="flex items-center justify-center" style="height: {availableHeight}px">
									{#if alert_type === 'recovered_critical_error'}
										<span title="Recovered Critical Alert">
											<CheckCircle2 size="20" color="green" />
										</span>
									{:else}
										<span title="Critical Alert">
											<AlertCircle size="20" color="red" />
										</span>
									{/if}
								</div>
							</Cell>

							<Cell wrap>
								<div class="flex-shrink min-w-0 break-words"><Linkify text={message} /></div>
							</Cell>
							<!-- Flexible width -->
							<Cell wrap>{formatDate(created_at)}</Cell>
							{#if $devopsRole}
								<Cell>{workspace_id ? workspace_id : 'global'}</Cell>
							{/if}
							<Cell>
								<div class="w-full flex justify-center items-center">
									{#if !acknowledged}
										<Button
											variant="accent"
											startIcon={{ icon: CheckCircle2 }}
											unifiedSize="sm"
											on:click={() => {
												if (id) acknowledgeAlert(id)
											}}
											title="Acknowledge"
										>
											Acknowledge
										</Button>
									{:else}
										<CheckCircle2 size="20" />
									{/if}
								</div>
							</Cell>
						</Row>
					{/if}
				{/each}
			</tbody>
		{/if}
	</DataTable>
</div>
