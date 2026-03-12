<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Select from './select/Select.svelte'
	import {
		ArrowLeft,
		Copy,
		Download,
		Expand,
		LoaderCircle,
		Minimize,
		RefreshCcw,
		Upload
	} from 'lucide-svelte'
	import DBManagerContent from './DBManagerContent.svelte'
	import { resource } from 'runed'
	import { untrack } from 'svelte'
	import type { DbManagerUriState } from './dbManagerDrawerModel.svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import ResourcePicker from './ResourcePicker.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		uriState: DbManagerUriState
		/** Z-index offset for the drawer, useful when opening from within modals */
		offset?: number
	}

	let { uriState, offset = 0 }: Props = $props()

	let open = $derived(uriState.open)

	// Load available datatables when drawer opens with datatable input
	const datatables = resource<string[]>([], async () => {
		if (!$workspaceStore) return []
		try {
			return await WorkspaceService.listDataTables({ workspace: $workspaceStore })
		} catch (e) {
			console.error('Failed to load datatables:', e)
			return []
		}
	})

	const datatableItems = $derived(
		datatables.current.map((dt) => ({
			value: dt,
			label: dt
		}))
	)

	// Refetch datatables when switching to a datatable input
	$effect(() => {
		if (uriState.isDatatableInput) {
			untrack(() => datatables.refetch())
		}
	})

	function handleClose() {
		uriState.closeDrawer()
		dbManagerContent?.clearReplResult()
	}

	let windowWidth = $state(window.innerWidth)
	let expand = $state(false)

	$effect(() => {
		if (!open) {
			expand = false
			uriState.closeDrawer()
		}
	})

	let dbManagerContent: DBManagerContent | undefined = $state()

	let hasReplResult = $state(false)

	// Export/Import state
	let exportDrawerOpen = $state(false)
	let exportLoading = $state(false)
	let exportResult = $state('')
	let importDrawerOpen = $state(false)
	let importLoading = $state(false)
	let importSource = $state<string | undefined>(undefined)
	let importBehavior = $state<'schema_only' | 'schema_and_data'>('schema_only')

	const isPostgresqlInput = $derived(
		uriState.isDatatableInput ||
			(uriState.input?.type === 'database' && uriState.input.resourceType === 'postgresql')
	)

	function currentSourceIdentifier(): string | undefined {
		const input = uriState.effectiveInput
		if (!input || input.type !== 'database') return undefined
		return input.resourcePath
	}

	async function handleExportSchema() {
		const source = currentSourceIdentifier()
		if (!source || !$workspaceStore) return
		exportLoading = true
		try {
			exportResult = await WorkspaceService.exportPgSchema({
				workspace: $workspaceStore,
				requestBody: { source }
			})
			exportDrawerOpen = true
		} catch (e) {
			sendUserToast(`Failed to export schema: ${e}`, true)
		} finally {
			exportLoading = false
		}
	}

	async function handleImportDatabase() {
		if (!importSource || !$workspaceStore) return
		const target = currentSourceIdentifier()
		if (!target) return
		importLoading = true
		try {
			await WorkspaceService.forkPgDatabase({
				workspace: $workspaceStore,
				requestBody: {
					source: importSource,
					target,
					fork_behavior: importBehavior
				}
			})
			sendUserToast('Database import completed successfully')
			importDrawerOpen = false
			importSource = undefined
			dbManagerContent?.refresh()
		} catch (e) {
			sendUserToast(`Failed to import database: ${e}`, true)
		} finally {
			importLoading = false
		}
	}
</script>

<svelte:window bind:innerWidth={windowWidth} />

<Drawer
	bind:open
	size={expand ? `${windowWidth}px` : '1200px'}
	preventEscape
	{offset}
	on:close={handleClose}
>
	<DrawerContent
		title={hasReplResult ? 'Query Result' : 'Database Manager'}
		on:close={() => {
			if (hasReplResult) {
				dbManagerContent?.clearReplResult()
			} else {
				handleClose()
			}
		}}
		CloseIcon={hasReplResult ? ArrowLeft : undefined}
		noPadding
		id="db-manager-drawer"
	>
		{#if uriState.effectiveInput && $workspaceStore}
			{#key uriState.selectedDatatable}
				<DBManagerContent
					bind:this={dbManagerContent}
					input={uriState.effectiveInput}
					bind:hasReplResult
					bind:selectedSchemaKey={uriState.selectedSchema}
					bind:selectedTableKey={uriState.selectedTable}
				>
					{#snippet dbSelector()}
						{#if uriState.isDatatableInput}
							{#if datatables.loading}
								<div class="flex items-center gap-2 text-tertiary ml-2">
									<LoaderCircle size={14} class="animate-spin" />
									<span class="text-sm">Loading...</span>
								</div>
							{:else if datatables.current.length >= 1}
								<Select
									transformInputSelectedText={(s) => `Datatable: ${s}`}
									items={datatableItems}
									bind:value={uriState.selectedDatatable}
									placeholder="Select data table"
									size="md"
								/>
							{/if}
						{/if}
					{/snippet}
				</DBManagerContent>
			{/key}
		{/if}
		{#snippet actions()}
			{#if isPostgresqlInput}
				<DropdownV2
					items={[
						{
							displayName: 'Export schemas',
							icon: Download,
							action: () => handleExportSchema()
						},
						{
							displayName: 'Import database',
							icon: Upload,
							action: () => {
								importDrawerOpen = true
							}
						}
					]}
				/>
			{/if}
			<Button
				loading={dbManagerContent?.isLoading() ?? false}
				on:click={() => {
					dbManagerContent?.refresh()
					dbManagerContent?.dbManager()?.dbTable()?.refresh()
				}}
				startIcon={{ icon: RefreshCcw }}
				size="xs"
				color="light"
			>
				Refresh
			</Button>

			<Button
				on:click={() => (expand = !expand)}
				startIcon={{ icon: expand ? Minimize : Expand }}
				size="xs"
				color="light"
			/>
		{/snippet}
	</DrawerContent>
</Drawer>

<Drawer bind:open={exportDrawerOpen} size="800px" offset={offset + 1}>
	<DrawerContent title="Export Schemas" on:close={() => (exportDrawerOpen = false)}>
		{#if exportResult}
			<div class="flex flex-col gap-2 h-full relative">
				<pre class="overflow-auto text-xs bg-surface-secondary p-4 rounded flex-1">
					{exportResult}
				</pre>
				<Button
					size="xs"
					color="light"
					startIcon={{ icon: Copy }}
					wrapperClasses="absolute top-2 right-2"
					btnClasses="bg-surface-tertiary"
					on:click={() => {
						navigator.clipboard.writeText(exportResult)
						sendUserToast('Copied to clipboard')
					}}
				>
					Copy
				</Button>
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:open={importDrawerOpen} size="600px" offset={offset + 1}>
	<DrawerContent title="Import Database" on:close={() => (importDrawerOpen = false)}>
		<div class="flex flex-col gap-4">
			<Alert type="warning" title="Warning">
				This will import the schemas from the selected source into the current database. Existing
				tables with the same names may be affected.
			</Alert>
			<div class="flex flex-col gap-2">
				<span class="text-sm font-medium">Source database</span>
				<ResourcePicker datatableAsPgResource bind:value={importSource} resourceType="postgresql" />
			</div>
			<div class="flex flex-col gap-2">
				<span class="text-sm font-medium">Import mode</span>
				<Select
					items={[
						{ value: 'schema_only', label: 'Schema only' },
						{ value: 'schema_and_data', label: 'Schema and data' }
					]}
					bind:value={importBehavior}
				/>
			</div>
			<Button
				disabled={!importSource}
				loading={importLoading}
				color="red"
				on:click={handleImportDatabase}
			>
				Import schemas into current database
			</Button>
		</div>
	</DrawerContent>
</Drawer>
