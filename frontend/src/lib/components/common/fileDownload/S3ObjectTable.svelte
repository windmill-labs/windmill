<script lang="ts">
	import { ChevronDown, Download, Loader2 } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import type { IDbTableOps } from '$lib/components/dbOps'
	import { s3TableOps, serverTableFormat } from '$lib/components/s3TableOps'
	import {
		setOperatingWorkspace,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import { downloadViaClient, shouldDownloadViaClient } from '$lib/utils/downloadFile'

	interface Props {
		s3object: { s3: string; storage?: string; presigned?: string; disable_download?: boolean }
		workspaceId?: string | undefined
		/** Set in a deployed app: the file is read through the app-scoped `apps_u/*` routes. */
		appPath?: string | undefined
	}

	let { s3object, workspaceId = undefined, appPath = undefined }: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()
	// DBTable loads no rows without an operating workspace, and a public app sets none.
	setOperatingWorkspace(() => workspaceId)

	const CSV_SEPARATORS = [
		{ value: ',', label: ',' },
		{ value: ';', label: ';' },
		{ value: '\t', label: 'Tab' },
		{ value: '|', label: '|' }
	]
	// DBTable's search bar, header row and body rows, in px.
	const TOOLBAR_HEIGHT = 40
	const HEADER_HEIGHT = 30
	const ROW_HEIGHT = 34
	const SCROLLBAR = 12

	let separator: string | undefined = $state(undefined)
	let hasHeader = $state(true)
	/** The separator the server guessed while none is picked. */
	let detectedSeparator: string | undefined = $state(undefined)
	/** Rows in the whole file, kept while a search narrows them so the table keeps its height. */
	let totalRows: number | undefined = $state(undefined)

	// JSON lines are read through the CSV route too, but have no separator or header row.
	let isCsv = $derived(/\.(csv|tsv)$/i.test(s3object.s3))
	let isParquet = $derived(serverTableFormat(s3object.s3) === 'parquet')
	let workspace = $derived(workspaceId ?? $operatingWorkspace!)
	// The server converts the whole file; the table's own export only has the rows it loaded.
	let parquetCsvPath = $derived(
		appPath
			? `/w/${workspace}/apps_u/download_s3_parquet_file_as_csv/${appPath}?file_key=${encodeURIComponent(s3object.s3)}${s3object.storage ? `&storage=${s3object.storage}` : ''}${s3object.presigned ? `&${s3object.presigned}` : ''}`
			: `/w/${workspace}/job_helpers/download_s3_parquet_file_as_csv?file_key=${encodeURIComponent(s3object.s3)}${s3object.storage ? `&storage=${s3object.storage}` : ''}`
	)
	let parquetCsvName = $derived((s3object.s3.split('/').pop() ?? 'download') + '.csv')

	let height = $derived(
		totalRows === undefined
			? 240
			: Math.min(
					600,
					Math.max(180, TOOLBAR_HEIGHT + HEADER_HEIGHT + totalRows * ROW_HEIGHT + SCROLLBAR)
				)
	)

	function recordingTotal(ops: IDbTableOps): IDbTableOps {
		return {
			...ops,
			getCount: async (params) => {
				const n = await ops.getCount(params)
				const filtered = !!params.quicksearch || Object.keys(params.columnFilters ?? {}).length > 0
				if (!filtered) totalRows = n
				return n
			}
		}
	}

	// Lazy: DBTable reaches Monaco, and every page that shows a job result imports this table.
	const dbTable = import('$lib/components/DBTable.svelte')

	let table = $derived.by(() => {
		const pending = s3TableOps({
			workspace,
			fileKey: s3object.s3,
			storage: s3object.storage,
			csvSeparator: separator,
			csvHasHeader: hasHeader,
			app: appPath ? { path: appPath, presigned: s3object.presigned } : undefined
		}).then((t) => {
			// A table superseded while loading must not label the current one.
			if (table !== pending) return t as never
			detectedSeparator = t.csvSeparator
			return dbTable.then((m) => ({ ...t, ops: recordingTotal(t.ops), DBTable: m.default }))
		})
		return pending
	})
</script>

<div class="flex flex-col gap-1 mt-2">
	{#if isCsv || (isParquet && !s3object.disable_download)}
		<div class="flex items-center justify-end gap-4 text-xs text-secondary">
			{#if isCsv}
				<div class="flex items-center gap-2">
					<span>Separator</span>
					<Select
						size="sm"
						class="w-16"
						RightIcon={ChevronDown}
						items={CSV_SEPARATORS}
						bind:value={separator}
						placeholder={CSV_SEPARATORS.find((s) => s.value === detectedSeparator)?.label ??
							'Auto'}
					/>
				</div>
				<Toggle size="xs" bind:checked={hasHeader} options={{ right: 'Header row' }} />
			{:else if shouldDownloadViaClient()}
				<Button
					variant="subtle"
					unifiedSize="xs"
					startIcon={{ icon: Download }}
					onClick={() => downloadViaClient(parquetCsvPath, parquetCsvName)}
				>
					Download as CSV
				</Button>
			{:else}
				<Button
					variant="subtle"
					unifiedSize="xs"
					startIcon={{ icon: Download }}
					href="/api{parquetCsvPath}"
					download={parquetCsvName}
				>
					Download as CSV
				</Button>
			{/if}
		</div>
	{/if}

	<div class="overflow-hidden border rounded-md" style="height: {height}px" data-interactive>
		{#await table}
			<div class="flex h-full items-center justify-center gap-1 text-2xs text-secondary">
				<Loader2 size={12} class="animate-spin" /> Loading table
			</div>
		{:then t}
			<t.DBTable dbTableOps={t.ops} disableDownload={s3object.disable_download || isParquet} />
		{:catch e}
			{@const message = String(e?.body ?? e?.message ?? e)}
			<!-- The storage reports an empty file as a failed range read. -->
			{#if /Range started at 0 and ended at 0/.test(message)}
				<div class="p-3 text-xs text-secondary">This file is empty.</div>
			{:else}
				<div class="p-3 text-xs text-red-600 dark:text-red-400 break-words">
					Could not read this file as a table: {message}
				</div>
			{/if}
		{/await}
	</div>
</div>
