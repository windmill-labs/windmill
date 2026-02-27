<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Select from './select/Select.svelte'
	import { ArrowLeft, Expand, LoaderCircle, Minimize, RefreshCcw } from 'lucide-svelte'
	import DBManagerContent from './DBManagerContent.svelte'
	import { resource } from 'runed'
	import { untrack } from 'svelte'
	import type { DbManagerUriState } from './dbManagerDrawerModel.svelte'

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
