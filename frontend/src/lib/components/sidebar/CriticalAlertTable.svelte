<script lang="ts">
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AlertCircle, CheckCircle2, CheckSquare2 } from 'lucide-svelte'
	import { devopsRole } from '$lib/stores'

	export let alerts: any[]
	export let hideAcknowledged = false
	export let goToNextPage: () => void
	export let goToPreviousPage: () => void
	export let acknowledgeAlert: (id: number) => void
	export let page = 1
	export let hasMore = true

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
</script>

<DataTable
	size="xs"
	paginated
	on:next={goToNextPage}
	on:previous={goToPreviousPage}
	bind:currentPage={page}
	{hasMore}
>
	<Head>
		<tr>
			<Cell head class="min-w-10">Type</Cell>
			<Cell head class="min-w-24">Message</Cell>
			<Cell head class="w-26">Created At</Cell>
			{#if $devopsRole}
				<Cell head class="min-w-24">Context</Cell>
			{/if}
			<Cell head class="min-w-24">Acknowledged</Cell>
		</tr>
	</Head>

	<tbody class="divide-y">
		{#each alerts as { id, alert_type, message, created_at, acknowledged, workspace_id }}
			{#if !hideAcknowledged || !acknowledged}
				<Row>
					<Cell>
						<div class="flex items-center justify-center">
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
						<div class="flex-shrink min-w-0 break-words">{message}</div>
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
									color="green"
									startIcon={{ icon: CheckSquare2 }}
									size="xs2"
									on:click={() => {
										if (id) acknowledgeAlert(id)
									}}>Acknowledge</Button
								>
							{:else}
								<CheckCircle2 size="20" color="green" />
							{/if}
						</div>
					</Cell>
				</Row>
			{/if}
		{/each}
	</tbody>
</DataTable>
