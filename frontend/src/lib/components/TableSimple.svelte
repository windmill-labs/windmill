<script lang="ts">
	import DropdownV2 from './DropdownV2.svelte'
	import Cell from './table/Cell.svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'

	export let headers: string[] | undefined
	export let data: any[] | undefined // Object containing the data
	export let keys: string[]
	export let size: 'sm' | 'md' | 'lg' = 'md'

	export let getRowActions: ((row: any) => any[]) | undefined = undefined
</script>

<div class="mt-2 w-full">
	<DataTable {size}>
		<Head>
			<tr>
				{#if headers}
					{#each headers as header, i}
						<Cell first={i == 0} last={i == headers.length - 1} head class="max-w-96">{header}</Cell
						>
					{/each}
					{#if getRowActions !== undefined}
						<Cell head last />
					{/if}
				{/if}
			</tr>
		</Head>
		<tbody class="divide-y">
			{#if data && keys && data.length > 0}
				{#each data as row}
					{@const rowActions = getRowActions?.(row)}
					<tr>
						{#each keys as key, i}
							<Cell
								first={i == 0}
								last={i == keys.length - 1}
								class="max-w-96 whitespace-pre-wrap overflow-hidden text-ellipsis"
							>
								{row[key] ?? ''}
							</Cell>
						{/each}
						{#if rowActions && rowActions.length > 0}
							<Cell last shouldStopPropagation>
								<DropdownV2 items={rowActions} />
							</Cell>
						{/if}
					</tr>
				{/each}
			{:else}
				<tr><td>Loading...</td></tr>
			{/if}
		</tbody>
	</DataTable>
</div>
