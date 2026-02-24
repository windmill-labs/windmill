<script lang="ts">
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import type { Runnable } from '../apps/inputType'
	import RawAppInlineScriptPanelList from './RawAppInlineScriptPanelList.svelte'
	import FileExplorer from '../FileExplorer.svelte'
	import { Plus, File, Folder, Camera } from 'lucide-svelte'
	import type { Modules } from './RawAppModules.svelte'
	import RawAppModules from './RawAppModules.svelte'
	import RawAppHistoryList from './RawAppHistoryList.svelte'
	import type { RawAppHistoryManager } from './RawAppHistoryManager.svelte'
	import Button from '../common/button/Button.svelte'
	import RawAppDataTableList from './RawAppDataTableList.svelte'
	import type { DataTableRef } from './dataTableRefUtils'
	import RawAppDataTableDrawer from './RawAppDataTableDrawer.svelte'

	interface Props {
		runnables: Record<string, Runnable>
		selectedRunnable: string | undefined
		files: Record<string, string>
		modules?: Modules
		onSelectFile?: (path: string) => void
		selectedDocument: string | undefined
		historyManager?: RawAppHistoryManager
		historySelectedId?: number | undefined
		onHistorySelect?: (id: number) => void
		onHistorySelectCurrent?: () => void
		onManualSnapshot?: () => void
		dataTableRefs?: DataTableRef[]
		onDataTableRefsChange?: (refs: DataTableRef[]) => void
		/** Default datatable for new tables */
		defaultDatatable?: string | undefined
		/** Default schema for new tables */
		defaultSchema?: string | undefined
		onDefaultChange?: (datatable: string | undefined, schema: string | undefined) => void
	}

	let {
		runnables,
		selectedRunnable = $bindable(),
		files = $bindable({}),
		modules,
		onSelectFile,
		selectedDocument = $bindable(),
		historyManager,
		historySelectedId,
		onHistorySelect,
		onHistorySelectCurrent,
		onManualSnapshot,
		dataTableRefs = [],
		onDataTableRefsChange,
		defaultDatatable = undefined,
		defaultSchema = undefined,
		onDefaultChange
	}: Props = $props()

	let dataTableDrawer: RawAppDataTableDrawer | undefined = $state()
	let selectedDataTableIndex: number | undefined = $state(undefined)

	function handleAddDataTable(ref: DataTableRef) {
		onDataTableRefsChange?.([...dataTableRefs, ref])
	}

	function handleRemoveDataTable(index: number) {
		onDataTableRefsChange?.(dataTableRefs.filter((_, i) => i !== index))
		if (selectedDataTableIndex === index) {
			selectedDataTableIndex = undefined
		}
	}

	function handleSelectDataTable(ref: DataTableRef, index: number) {
		selectedDataTableIndex = selectedDataTableIndex === index ? undefined : index
		if (selectedDataTableIndex === index) {
			dataTableDrawer?.openDrawerWithRef(ref)
		}
	}

	let fileExplorer: FileExplorer | undefined = $state()

	function handleSelectPath(path: string) {
		selectedDocument = path
		if (!path.endsWith('/')) {
			onSelectFile?.(path)
		}
	}
</script>

<PanelSection size="sm" fullHeight={false} title="frontend" id="app-editor-frontend-panel">
	{#snippet action()}
		<div class="flex gap-1">
			<Button
				onClick={() => fileExplorer?.handleAddRootFile()}
				title="Add file to root"
				unifiedSize="xs"
				variant="subtle"
				btnClasses="px-1 gap-0.5"
			>
				<Plus size={12} />
				<File size={12} />
			</Button>
			<Button
				onClick={() => fileExplorer?.handleAddRootFolder()}
				title="Add folder to root"
				unifiedSize="xs"
				variant="subtle"
				btnClasses="px-1 gap-0.5"
			>
				<Plus size={12} />
				<Folder size={12} />
			</Button>
		</div>
	{/snippet}
	<FileExplorer
		bind:this={fileExplorer}
		bind:files
		selectedPath={selectedDocument}
		onSelectPath={handleSelectPath}
		extraNodes={[{ name: 'wmill.ts', path: '/wmill.ts', isFolder: false }]}
		hideHeader
	/>
</PanelSection>

<RawAppModules {modules} />

<div class="py-4"></div>
<RawAppInlineScriptPanelList
	bind:selectedRunnable
	{runnables}
	onSelect={() => {
		selectedDocument = undefined
	}}
/>

<div class="py-4"></div>
<RawAppDataTableList
	{dataTableRefs}
	{defaultDatatable}
	{defaultSchema}
	onAdd={() => dataTableDrawer?.openDrawer()}
	onRemove={handleRemoveDataTable}
	onSelect={handleSelectDataTable}
	{onDefaultChange}
	selectedIndex={selectedDataTableIndex}
/>
<RawAppDataTableDrawer
	bind:this={dataTableDrawer}
	onAdd={handleAddDataTable}
	existingRefs={dataTableRefs}
/>

{#if historyManager && onHistorySelect && onManualSnapshot}
	<div class="py-4"></div>
	<PanelSection fullHeight={false} size="sm" title="history" id="app-editor-history-panel">
		{#snippet action()}
			<div class="flex items-center gap-2">
				<span class="text-2xs text-tertiary">{historyManager.allEntries.length}/50</span>
				<Button
					unifiedSize="xs"
					variant="subtle"
					on:click={onManualSnapshot}
					btnClasses="px-1 gap-0.5"
					title="Create a new snapshot"
				>
					<Plus size={12} />
					<Camera size={12} />
				</Button>
			</div>
		{/snippet}
		<RawAppHistoryList
			entries={historyManager.allEntries}
			branches={historyManager.allBranches}
			selectedId={historySelectedId}
			onSelect={onHistorySelect}
			onSelectCurrent={onHistorySelectCurrent}
		/>
	</PanelSection>
{/if}
