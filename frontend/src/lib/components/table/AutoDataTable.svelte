<script lang="ts">
	import { Download, MoreVertical } from 'lucide-svelte'
	import Dropdown from '../DropdownV2.svelte'
	import Cell from './Cell.svelte'
	import DataTable from './DataTable.svelte'
	import Head from './Head.svelte'
	import Row from './Row.svelte'

	export let objects: Array<Record<string, any>> = []

	let currentPage = 1
	let perPage = 5

	let search: string = ''

	$: data = objects
		.filter((row) => {
			return Object.values(row).some((value) => {
				return value.toString().toLowerCase().includes(search.toLowerCase())
			})
		})
		.slice((currentPage - 1) * perPage, currentPage * perPage)
</script>

<div class="flex flex-col divide-y gap-2 pt-4 mt-4">
	<div class="flex flex-row justify-between items-center">
		<input bind:value={search} placeholder="Search..." class="h-8 !text-xs" />
		<Dropdown
			items={() => {
				return [
					{
						displayName: 'Download CSV',
						icon: Download,
						action: () => {
							const csv = objects
								.map((row) => {
									return Object.values(row).join(',')
								})
								.join('\n')

							const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
							const url = URL.createObjectURL(blob)
							const link = document.createElement('a')
							link.setAttribute('href', url)
							link.setAttribute('download', 'data.csv')
							link.style.visibility = 'hidden'
							document.body.appendChild(link)
							link.click()

							document.body.removeChild(link)
						}
					}
				]
			}}
		>
			<svelte:fragment slot="buttonReplacement">
				<MoreVertical
					size={8}
					class="w-8 h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md"
				/>
			</svelte:fragment>
		</Dropdown>
	</div>
	<DataTable
		size="xs"
		shouldHidePagination={false}
		paginated={true}
		{perPage}
		{currentPage}
		on:next={() => {
			currentPage += 1
		}}
		on:previous={() => {
			currentPage -= 1
		}}
		on:change={(event) => {
			currentPage = event.detail
		}}
		showNext={currentPage * perPage < objects.length}
	>
		<Head>
			<tr>
				{#each Object.keys(data[0] ?? {}) ?? [] as key, index}
					<Cell head first={index == 0} last={index == Object.keys(objects[0] ?? {}).length - 1}
						>{key}</Cell
					>
				{/each}
			</tr>
		</Head>
		<tbody class="divide-y">
			{#each data as row}
				<Row>
					{#each Object.values(row ?? {}) ?? [] as value, index}
						<Cell first={index == 0} last={index == Object.values(row ?? {}).length - 1}>
							{value}
						</Cell>
					{/each}
				</Row>
			{/each}
		</tbody>
	</DataTable>
</div>
