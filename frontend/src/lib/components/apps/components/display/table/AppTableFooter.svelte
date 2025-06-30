<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { Table } from '@tanstack/svelte-table'
	import { ChevronLeft, ChevronRight, Columns, Download } from 'lucide-svelte'
	import type { Readable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'

	type T = Record<string, any>

	interface Props {
		result: Array<T>
		manualPagination: boolean
		pageSize: number
		table: Readable<Table<T>>
		download?: boolean
		loading?: boolean
		class?: string
		style?: string
	}

	let {
		result,
		manualPagination,
		pageSize,
		table,
		download = true,
		loading = false,
		class: c = '',
		style = ''
	}: Props = $props()

	function convertJSONToCSV(objArray: Record<string, any>[]) {
		let str = ''

		const headers = Object.keys(objArray[0])
		str += headers.join(',') + '\r\n'

		for (let i = 0; i < objArray.length; i++) {
			let line = ''
			for (let j = 0; j < headers.length; j++) {
				let value = objArray[i][headers[j]]
				line += j ? ',' : ''
				if (typeof value != 'string') {
					value = JSON.stringify(value)
				}
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

	let isPreviousLoading = $state(false)
	let isNextLoading = $state(false)

	$effect(() => {
		if (!loading) {
			isPreviousLoading = false
			isNextLoading = false
		}
	})
</script>

{#if result.length > pageSize || manualPagination || download}
	<div
		class={twMerge('px-2 py-1 text-xs gap-2 items-center justify-between', c, 'flex flex-row')}
		{style}
	>
		<div class="flex items-center gap-1 flex-row">
			{#if download}
				<Popover>
					{#snippet text()}
						Download as CSV
					{/snippet}

					<Button
						size="xs2"
						color="light"
						on:click={downloadResultAsCSV}
						startIcon={{ icon: Download }}
						wrapperClasses="app-table-footer-btn"
						iconOnly
					/>
				</Popover>
			{/if}
			{#if !$table.getIsAllColumnsVisible()}
				<Popover>
					{#snippet text()}
						Display hidden columns
					{/snippet}
					<Button
						size="xs2"
						color="light"
						on:click={() => {
							$table.getAllColumns().forEach((column) => column.toggleVisibility(true))
						}}
						startIcon={{ icon: Columns }}
						wrapperClasses="app-table-footer-btn"
						iconOnly
					/>
				</Popover>
			{/if}
		</div>
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
					loading={isPreviousLoading && loading}
					startIcon={{ icon: ChevronLeft }}
					wrapperClasses="app-table-footer-btn"
				>
					Previous
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
					loading={isNextLoading && loading}
					endIcon={{ icon: ChevronRight }}
					wrapperClasses="app-table-footer-btn"
				>
					Next
				</Button>
				<div>
					Page:
					{$table.getState().pagination.pageIndex + 1}
					{$table.getPageCount() > 0 ? ` of ${$table.getPageCount()}` : ''}
				</div>
			</div>
		{/if}
	</div>
{/if}
