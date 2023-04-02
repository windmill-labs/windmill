<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faDownload } from '@fortawesome/free-solid-svg-icons'
	import type { Table } from '@tanstack/svelte-table'
	import { ChevronLeft, ChevronRight } from 'lucide-svelte'
	import type { Readable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'

	type T = Record<string, any>

	export let result: Array<T>
	export let manualPagination: boolean
	export let pageSize: number
	export let table: Readable<Table<T>>
	let c = ''
	export { c as class }
	export let style = ''

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

<div
	class={twMerge('px-2 py-1 text-xs gap-2 items-center justify-between', c, 'flex flex-row')}
	{style}
>
	{#if result.length > pageSize || manualPagination}
		<div class="flex items-center gap-2 flex-row">
			<Button
				size="xs"
				variant="border"
				color="light"
				btnClasses="!py-1 !pl-1"
				on:click={() => $table.previousPage()}
				disabled={!$table.getCanPreviousPage()}
			>
				<ChevronLeft size={14} />
				Previous
			</Button>
			<Button
				size="xs"
				variant="border"
				color="light"
				btnClasses="!py-1 !pr-1"
				on:click={() => $table.nextPage()}
				disabled={!$table.getCanNextPage()}
			>
				Next
				<ChevronRight size={14} />
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
			btnClasses="!py-1"
			on:click={downloadResultAsJSON}
			startIcon={{ icon: faDownload }}
		>
			Download
		</Button>
	</div>
</div>
