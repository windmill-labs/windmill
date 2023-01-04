<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faDownload } from '@fortawesome/free-solid-svg-icons'
	import type { Table } from '@tanstack/svelte-table'
	import type { Readable } from 'svelte/store'
	import { tableOptions } from './tableOptions'

	type T = Record<string, any>

	export let result: Array<T>
	export let paginationEnabled: boolean = false
	export let table: Readable<Table<T>>

	function downloadResultAsJSON() {
		const dataStr = 'data:text/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(result))
		const downloadAnchorNode = document.createElement('a')
		downloadAnchorNode.setAttribute('href', dataStr)
		downloadAnchorNode.setAttribute('download', 'data.json')
		document.body.appendChild(downloadAnchorNode)
		downloadAnchorNode.click()
		downloadAnchorNode.remove()
	}
</script>

<div class="px-4 py-2 text-xs flex flex-row gap-2 items-center justify-between">
	{#if paginationEnabled && result.length > (tableOptions.initialState?.pagination?.pageSize ?? 25)}
		<div class="flex items-center gap-2 flex-row">
			<Button
				size="xs"
				variant="border"
				color="light"
				on:click={() => $table.previousPage()}
				disabled={!$table.getCanPreviousPage()}
			>
				Previous
			</Button>
			<Button
				size="xs"
				variant="border"
				color="light"
				on:click={() => $table.nextPage()}
				disabled={!$table.getCanNextPage()}
			>
				Next
			</Button>
			{$table.getState().pagination.pageIndex + 1} of {$table.getPageCount()}
		</div>
	{:else}
		<div />
	{/if}
	<div class="flex items-center gap-2 flex-row">
		<Button
			size="xs"
			variant="border"
			color="light"
			on:click={downloadResultAsJSON}
			startIcon={{ icon: faDownload }}
		>
			Download
		</Button>
	</div>
</div>
