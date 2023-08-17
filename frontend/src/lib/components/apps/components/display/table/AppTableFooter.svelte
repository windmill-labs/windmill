<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faDownload } from '@fortawesome/free-solid-svg-icons'
	import type { Table } from '@tanstack/svelte-table'
	import { ChevronLeft, ChevronRight, Loader2 } from 'lucide-svelte'
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
	export let download: boolean = true
	export let loading: boolean = false

	function convertJSONToCSV(objArray: Record<string, any>[]) {
		let str = ''

		const headers = Object.keys(objArray[0])
		str += headers.join(',') + '\r\n'

		for (let i = 0; i < objArray.length; i++) {
			let line = ''
			for (let j = 0; j < headers.length; j++) {
				const value = objArray[i][headers[j]]
				line += j ? ',' : ''
				line += /[\",\n]/.test(value) ? '"' + value.replace(/"/g, '""') + '"' : value
			}
			str += line + '\r\n'
		}

		return str
	}

	function downloadResultAsCSV() {
		const csvStr = 'data:text/csv;charset=utf-8,' + encodeURIComponent(convertJSONToCSV(result))
		const downloadAnchorNode = document.createElement('a')
		downloadAnchorNode.setAttribute('href', csvStr)
		downloadAnchorNode.setAttribute('download', 'data.csv')
		document.body.appendChild(downloadAnchorNode)
		downloadAnchorNode.click()
		downloadAnchorNode.remove()
	}

	let isPreviousLoading = false
	let isNextLoading = false

	$: if (!loading) {
		isPreviousLoading = false
		isNextLoading = false
	}
</script>

{#if result.length > pageSize || manualPagination || download}
	<div
		class={twMerge('px-2 py-1 text-xs gap-2 items-center justify-between', c, 'flex flex-row')}
		{style}
	>
		{#if result.length > pageSize || manualPagination}
			<div class="flex items-center gap-2 flex-row">
				<Button
					size="xs2"
					variant="border"
					color="light"
					on:click={() => {
						isPreviousLoading = true
						$table.previousPage()
					}}
					disabled={!$table.getCanPreviousPage()}
				>
					<div class="flex flex-row gap-1 items-center">
						{#if isPreviousLoading && loading}
							<Loader2 size={14} class="animate-spin" />
						{:else}
							<ChevronLeft size={14} />
						{/if}
						Previous
					</div>
				</Button>
				<Button
					size="xs2"
					variant="border"
					color="light"
					on:click={() => {
						isNextLoading = true
						$table.nextPage()
					}}
					disabled={!$table.getCanNextPage()}
				>
					<div class="flex flex-row gap-1 items-center">
						Next

						{#if isNextLoading && loading}
							<Loader2 size={14} class="animate-spin" />
						{:else}
							<ChevronRight size={14} />
						{/if}
					</div>
				</Button>
				{$table.getState().pagination.pageIndex + 1} of {$table.getPageCount()}
			</div>
		{:else}
			<div />
		{/if}
		<div class="flex items-center gap-2 flex-row">
			{#if download}
				<Button
					size="xs"
					variant="border"
					color="light"
					btnClasses="!py-1"
					on:click={downloadResultAsCSV}
					startIcon={{ icon: faDownload }}
				>
					Download
				</Button>
			{/if}
		</div>
	</div>
{/if}
