<script lang="ts">
	import Cell from './table/Cell.svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'

	export let headers: string[] | undefined
	export let data: any[] | undefined // Object containing the data
	export let keys: string[]
	export let size: 'sm' | 'md' | 'lg' = 'md'
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
				{/if}
			</tr>
		</Head>
		<tbody class="divide-y">
			{#if data && keys && data.length > 0}
				{#each data as row}
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
					</tr>
				{/each}
			{:else}
				<tr>Loading...</tr>
			{/if}
		</tbody>
	</DataTable>
</div>
