<script lang="ts">
	import { untrack } from 'svelte'
	import { ChevronDown, FileDown, Loader2, PanelTop } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { Button } from '$lib/components/common'
	import type { IDbTableOps } from '$lib/components/dbOps'
	import { s3TableOps, serverTableFormat } from '$lib/components/s3TableOps'
	import {
		setOperatingWorkspace,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import { downloadViaClient, shouldDownloadViaClient } from '$lib/utils/downloadFile'
	import { S3TableView } from './s3TableView.svelte'

	interface Props {
		s3object: { s3: string; storage?: string; presigned?: string; disable_download?: boolean }
		workspaceId?: string | undefined
		/** Set in a deployed app: the file is read through the app-scoped `apps_u/*` routes. */
		appPath?: string | undefined
		/** The reader's settings, kept by the caller across remounts; local when not passed. */
		view?: S3TableView
	}

	let {
		s3object,
		workspaceId = undefined,
		appPath = undefined,
		view: viewProp = undefined
	}: Props = $props()
	let view = $derived(viewProp ?? new S3TableView())

	const operatingWorkspace = useOperatingWorkspace()
	// DBTable loads no rows without an operating workspace, and a public app sets none.
	setOperatingWorkspace(() => workspaceId)

	const CSV_SEPARATORS = [
		{ value: ',', short: ',', name: 'Comma' },
		{ value: ';', short: ';', name: 'Semicolon' },
		{ value: '\t', short: 'Tab', name: 'Tab' },
		{ value: '|', short: '|', name: 'Pipe' }
	]
	// Below this width the table drops the Columns label and shortens its search placeholder.
	const COMPACT_WIDTH = 600
	// DBTable's search bar, header row and body rows, in px.
	const TOOLBAR_HEIGHT = 40
	const HEADER_HEIGHT = 30
	const ROW_HEIGHT = 34
	const SCROLLBAR = 12

	/** The control that changed the table: the reloaded table remounts it, and it takes the
	 * focus back. */
	let refocus: 'separator' | 'header' | undefined = $state(undefined)
	function takesFocusBack(control: 'separator' | 'header') {
		return (el: HTMLElement) => {
			if (untrack(() => refocus) !== control) return
			refocus = undefined
			el.querySelector('button')?.focus()
		}
	}
	/** The separator the server guessed while none is picked. */
	let detectedSeparator: string | undefined = $state(undefined)
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
	let width = $state(0)
	let separator = $derived(
		CSV_SEPARATORS.find((s) => s.value === (view.separator ?? detectedSeparator))
	)
	let separatorItems = $derived([
		...CSV_SEPARATORS.map((s) => ({
			displayName: s.short === s.name ? s.name : `${s.name} ${s.short}`,
			action: () => {
				refocus = 'separator'
				view.separator = s.value
			}
		})),
		...(view.separator !== undefined
			? [
					{
						displayName: 'Detect automatically',
						action: () => {
							refocus = 'separator'
							view.separator = undefined
						}
					}
				]
			: [])
	])

	/** Rows in the whole file, kept while a search narrows them so the table keeps its height. */
	let height = $derived(
		view.totalRows === undefined
			? 240
			: Math.min(
					600,
					Math.max(180, TOOLBAR_HEIGHT + HEADER_HEIGHT + view.totalRows * ROW_HEIGHT + SCROLLBAR)
				)
	)

	function recordingTotal(ops: IDbTableOps): IDbTableOps {
		return {
			...ops,
			getCount: async (params) => {
				const n = await ops.getCount(params)
				const filtered = !!params.quicksearch || Object.keys(params.columnFilters ?? {}).length > 0
				if (!filtered) view.totalRows = n
				return n
			}
		}
	}

	// Lazy: DBTable reaches Monaco, and every page that shows a job result imports this card.
	const dbTable = import('$lib/components/DBTable.svelte')

	let table = $derived.by(() => {
		const pending = s3TableOps({
			workspace,
			fileKey: s3object.s3,
			storage: s3object.storage,
			csvSeparator: view.separator,
			csvHasHeader: view.hasHeader,
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

{#snippet csvControls()}
	<span class="contents" {@attach takesFocusBack('separator')}>
		<DropdownV2
			items={separatorItems}
			placement="bottom-end"
			fixedHeight={false}
			triggerLabel={separator
				? `Separator: ${separator.name}${view.separator === undefined ? ' (detected)' : ''}`
				: 'Separator: detect automatically'}
			class="h-7 px-2 gap-1 rounded-md text-xs text-primary hover:bg-surface-hover"
		>
			{#snippet buttonReplacement()}
				<span class={separator ? 'font-mono' : ''}>{separator?.short ?? 'Auto'}</span>
				<ChevronDown size={14} />
			{/snippet}
		</DropdownV2>
	</span>
	<span class="contents" {@attach takesFocusBack('header')}>
		<Button
			variant="subtle"
			unifiedSize="sm"
			iconOnly
			startIcon={{ icon: PanelTop }}
			selected={view.hasHeader}
			title={view.hasHeader ? 'First row is the header' : 'First row is data'}
			onClick={() => {
				refocus = 'header'
				view.hasHeader = !view.hasHeader
			}}
		/>
	</span>
	<div class="w-px h-5 border-l"></div>
{/snippet}

{#snippet parquetControls()}
	{#if shouldDownloadViaClient()}
		<Button
			variant="subtle"
			unifiedSize="sm"
			iconOnly
			startIcon={{ icon: FileDown }}
			title="Download the whole file as CSV"
			onClick={() => downloadViaClient(parquetCsvPath, parquetCsvName)}
		/>
	{:else}
		<Button
			variant="subtle"
			unifiedSize="sm"
			iconOnly
			startIcon={{ icon: FileDown }}
			title="Download the whole file as CSV"
			href="/api{parquetCsvPath}"
			download={parquetCsvName}
		/>
	{/if}
	<div class="w-px h-5 border-l"></div>
{/snippet}

<!-- The storage reports an empty file as a failed range read. -->
{#snippet loadError(e: any)}
	{@const message = String(e?.body ?? e?.message ?? e)}
	{#if /Range started at 0 and ended at 0/.test(message)}
		<div class="p-3 text-xs text-secondary">This file is empty.</div>
	{:else}
		<div class="p-3 text-xs text-red-600 dark:text-red-400 break-words">
			Could not read this file as a table: {message}
		</div>
	{/if}
{/snippet}

<div class="overflow-hidden" style="height: {height}px" bind:clientWidth={width} data-interactive>
	{#await table}
		<div class="flex h-full items-center justify-center gap-1 text-2xs text-secondary">
			<Loader2 size={12} class="animate-spin" /> Loading table
		</div>
	{:then t}
		<t.DBTable
			dbTableOps={t.ops}
			initialFilters={view.filters}
			onFiltersChange={(f) => (view.filters = f)}
			disableDownload={s3object.disable_download || isParquet}
			toolbar={isCsv
				? csvControls
				: isParquet && !s3object.disable_download
					? parquetControls
					: undefined}
			compact={width < COMPACT_WIDTH}
		/>
	{:catch e}
		{#if isCsv}
			<div class="flex items-center justify-end gap-2 bg-surface-input px-2 py-1.5">
				{@render csvControls()}
			</div>
		{/if}
		{@render loadError(e)}
	{/await}
</div>
