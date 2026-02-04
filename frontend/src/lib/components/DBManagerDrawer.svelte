<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Select from './select/Select.svelte'
	import { ArrowLeft, Expand, LoaderCircle, Minimize, RefreshCcw } from 'lucide-svelte'
	import type { DbInput } from './dbTypes'
	import DBManagerContent from './DBManagerContent.svelte'
	import { resource } from 'runed'

	interface Props {
		/** Z-index offset for the drawer, useful when opening from within modals */
		offset?: number
	}

	let { offset = 0 }: Props = $props()

	let input: DbInput | undefined = $state()
	let open = $derived(!!input)

	// For datatable inputs, track the selected datatable separately
	let selectedDatatable = $state<string | undefined>(undefined)

	// Check if input is a datatable type
	const isDatatableInput = $derived(
		input?.type === 'database' && input.resourcePath.startsWith('datatable://')
	)

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

	// Computed input that updates when selectedDatatable changes
	const effectiveInput: DbInput | undefined = $derived.by(() => {
		if (!input) return undefined
		if (!isDatatableInput || !selectedDatatable) return input
		return {
			...input,
			resourcePath: `datatable://${selectedDatatable}`
		}
	})

	const datatableItems = $derived(
		datatables.current.map((dt) => ({
			value: dt,
			label: dt
		}))
	)

	export function openDrawer(nInput: DbInput) {
		console.log('Opening DB Manager with input:', nInput)
		input = nInput
		if (isDatatableInput) {
			datatables.refetch()
		}
		// If it's a datatable input, extract the datatable name for the selector
		if (nInput.type === 'database' && nInput.resourcePath.startsWith('datatable://')) {
			selectedDatatable = nInput.resourcePath.replace('datatable://', '')
			datatables.refetch()
		} else {
			selectedDatatable = undefined
		}
	}
	export function closeDrawer() {
		input = undefined
		selectedDatatable = undefined
		dbManagerContent?.clearReplResult()
		if (window.location.hash.startsWith('#dbmanager:'))
			history.replaceState('', document.title, window.location.href.replace(/#dbmanager:.*$/, ''))
	}

	let windowWidth = $state(window.innerWidth)
	let expand = $state(false)

	$effect(() => {
		if (!open) expand = false
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
	on:close={closeDrawer}
>
	<DrawerContent
		title={hasReplResult ? 'Query Result' : 'Database Manager'}
		on:close={() => {
			if (hasReplResult) {
				dbManagerContent?.clearReplResult()
			} else {
				closeDrawer()
			}
		}}
		CloseIcon={hasReplResult ? ArrowLeft : undefined}
		noPadding
		id="db-manager-drawer"
	>
		{#if effectiveInput && $workspaceStore}
			{#key selectedDatatable}
				<DBManagerContent bind:this={dbManagerContent} input={effectiveInput} bind:hasReplResult>
					{#snippet dbSelector()}
						{#if isDatatableInput}
							{#if datatables.loading}
								<div class="flex items-center gap-2 text-tertiary ml-2">
									<LoaderCircle size={14} class="animate-spin" />
									<span class="text-sm">Loading...</span>
								</div>
							{:else if datatables.current.length >= 1}
								<Select
									transformInputSelectedText={(s) => `Datatable: ${s}`}
									items={datatableItems}
									bind:value={selectedDatatable}
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
