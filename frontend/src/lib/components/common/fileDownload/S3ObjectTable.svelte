<script lang="ts">
	import { ChevronDown, Loader2, PanelTop } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { Button } from '$lib/components/common'
	import type { IDbTableOps } from '$lib/components/dbOps'
	import { s3TableOps, serverTableFormat } from '$lib/components/s3TableOps'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	interface Props {
		s3object: { s3: string; storage?: string; presigned?: string; disable_download?: boolean }
		workspaceId?: string | undefined
		/** Set in a deployed app: the file is read through the app-scoped `apps_u/*` routes. */
		appPath?: string | undefined
	}

	let { s3object, workspaceId = undefined, appPath = undefined }: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()

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

	let csvSeparator: string | undefined = $state(undefined)
	let csvHasHeader = $state(true)
	/** The separator the server guessed while none is picked. */
	let detectedSeparator: string | undefined = $state(undefined)
	let isCsv = $derived(serverTableFormat(s3object.s3) === 'csv')
	let width = $state(0)
	let separator = $derived(
		CSV_SEPARATORS.find((s) => s.value === (csvSeparator ?? detectedSeparator))
	)
	let separatorItems = $derived([
		...CSV_SEPARATORS.map((s) => ({
			displayName: s.short === s.name ? s.name : `${s.name} ${s.short}`,
			action: () => (csvSeparator = s.value)
		})),
		...(csvSeparator !== undefined
			? [{ displayName: 'Detect automatically', action: () => (csvSeparator = undefined) }]
			: [])
	])

	/** Rows in the whole file, kept while a search narrows them so the table keeps its height. */
	let totalRows: number | undefined = $state(undefined)
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

	// Lazy: DBTable reaches Monaco, and every page that shows a job result imports this card.
	const dbTable = import('$lib/components/DBTable.svelte')

	let table = $derived(
		s3TableOps({
			workspace: workspaceId ?? $operatingWorkspace!,
			fileKey: s3object.s3,
			storage: s3object.storage,
			csvSeparator,
			csvHasHeader,
			app: appPath ? { path: appPath, presigned: s3object.presigned } : undefined
		}).then((t) => {
			detectedSeparator = t.csvSeparator
			return dbTable.then((m) => ({ ...t, ops: recordingTotal(t.ops), DBTable: m.default }))
		})
	)
</script>

{#snippet csvControls()}
	<DropdownV2
		items={separatorItems}
		placement="bottom-end"
		fixedHeight={false}
		triggerLabel="Separator: {separator?.name ?? 'detecting'}{csvSeparator === undefined
			? ' (detected)'
			: ''}"
		class="h-7 px-2 gap-1 rounded-md text-xs text-primary hover:bg-surface-hover"
	>
		{#snippet buttonReplacement()}
			<span class="font-mono">{separator?.short ?? '…'}</span>
			<ChevronDown size={14} />
		{/snippet}
	</DropdownV2>
	<!-- The app-scoped routes always read the first row as the header. -->
	{#if !appPath}
		<Button
			variant="subtle"
			unifiedSize="sm"
			iconOnly
			startIcon={{ icon: PanelTop }}
			selected={csvHasHeader}
			title={csvHasHeader ? 'First row is the header' : 'First row is data'}
			onClick={() => (csvHasHeader = !csvHasHeader)}
		/>
	{/if}
	<div class="w-px h-5 border-l"></div>
{/snippet}

<div class="overflow-hidden" style="height: {height}px" bind:clientWidth={width} data-interactive>
	{#await table}
		<div class="flex h-full items-center justify-center gap-1 text-2xs text-secondary">
			<Loader2 size={12} class="animate-spin" /> Loading table
		</div>
	{:then t}
		<t.DBTable
			dbTableOps={t.ops}
			disableDownload={s3object.disable_download}
			toolbar={isCsv ? csvControls : undefined}
			compact={width < COMPACT_WIDTH}
		/>
	{:catch e}
		{#if isCsv}
			<div class="flex items-center justify-end gap-2 bg-surface-input px-2 py-1.5">
				{@render csvControls()}
			</div>
		{/if}
		<div class="p-3 text-xs text-red-600 dark:text-red-400 break-words">
			Could not read this file as a table: {e?.body ?? e?.message ?? e}
		</div>
	{/await}
</div>
