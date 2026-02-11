<script lang="ts" module>
	export type DataTableSettingsType = {
		dataTables: {
			name: string
			database: {
				resource_type: 'postgresql' | 'instance'
				resource_path?: string | undefined
			}
		}[]
	}

	export function convertDataTableSettingsFromBackend(
		settings: GetSettingsResponse['datatable']
	): DataTableSettingsType {
		const s: DataTableSettingsType = { dataTables: [] }
		if (settings?.datatables) {
			for (const [name, rest] of Object.entries(settings.datatables)) {
				s.dataTables.push({ name, ...rest })
			}
		}
		return s
	}
	export function convertDataTableSettingsToBackend(
		settings: DataTableSettingsType
	): NonNullable<GetSettingsResponse['datatable']> {
		const s: GetSettingsResponse['datatable'] = { datatables: {} }
		for (const dataTable of settings.dataTables) {
			const database = dataTable.database
			if (dataTable.name in s.datatables)
				throw 'Settings contain duplicate dataTable name: ' + dataTable.name
			if (!database.resource_path) throw 'No resource selected for ' + dataTable.name
			if (database.resource_type === 'instance' && database.resource_path === 'windmill')
				throw dataTable.name + ' database cannot be called "windmill"'

			s.datatables[dataTable.name] = {
				database: dataTable.database
			}
		}
		return s
	}

	let DEFAULT_DATATABLE_DB_NAME = 'datatable_db'
</script>

<script lang="ts">
	import { Plus } from 'lucide-svelte'

	import Button from '../common/button/Button.svelte'

	import CloseButton from '../common/CloseButton.svelte'

	import ResourcePicker from '../ResourcePicker.svelte'
	import SettingsPageHeader from '../settings/SettingsPageHeader.svelte'
	import Select from '../select/Select.svelte'
	import Cell from '../table/Cell.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import { random_adj } from '../random_positive_adjetive'
	import { sendUserToast } from '$lib/toast'
	import { SettingService, WorkspaceService, type GetSettingsResponse } from '$lib/gen'
	import { globalDbManagerDrawer, workspaceStore } from '$lib/stores'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { resource } from 'runed'
	import CustomInstanceDbSelect from './CustomInstanceDbSelect.svelte'
	import { Popover } from '../meltComponents'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import { deepEqual } from 'fast-equals'
	import { clone } from '$lib/utils'
	import SettingsFooter from './SettingsFooter.svelte'

	type Props = {
		dataTableSettings: DataTableSettingsType
	}

	let { dataTableSettings = $bindable() }: Props = $props()

	let tableHeadNames = ['Name', 'Database', '', ''] as const
	let tableHeadTooltips: Partial<Record<(typeof tableHeadNames)[number], string | undefined>> = {
		Name: 'Data tables are referenced by their name. main is a special name that can be used as the default data table.',
		Database: 'The database where the data is stored.'
	}

	let tempSettings: DataTableSettingsType = $derived.by(() => {
		let s = $state($state.snapshot(dataTableSettings))
		return s
	})

	function removeDataTable(index: number) {
		tempSettings.dataTables.splice(index, 1)
	}

	function onNewDataTable() {
		const name = tempSettings.dataTables.some((d) => d.name === 'main')
			? `${random_adj()}_datatable`
			: 'main'
		tempSettings.dataTables.push({
			name,
			database: {
				resource_type: $isCustomInstanceDbEnabled ? 'instance' : 'postgresql',
				resource_path: $isCustomInstanceDbEnabled ? DEFAULT_DATATABLE_DB_NAME : undefined
			}
		})
	}

	const customInstanceDbs = resource([], SettingService.listCustomInstanceDbs)

	async function onSave() {
		try {
			if (
				$isCustomInstanceDbEnabled &&
				tempSettings.dataTables.some(
					(d) =>
						d.database.resource_type === 'instance' &&
						!customInstanceDbs.current?.[d.database.resource_path ?? '']?.success
				)
			) {
				let confirm = await confirmationModal.ask({
					title: 'Some databases are not setup',
					children: 'Are you sure you want to save without setting them up ?',
					confirmationText: 'Save anyway'
				})
				if (!confirm) return
			}
			const settings = convertDataTableSettingsToBackend(tempSettings)
			await WorkspaceService.editDataTableConfig({
				workspace: $workspaceStore!,
				requestBody: { settings }
			})
			dataTableSettings = clone(tempSettings)
			sendUserToast('Data table settings saved successfully')
		} catch (e) {
			sendUserToast(e, true)
			console.error('Error saving data table settings', e)
			throw e
		}
	}

	let confirmationModal = createAsyncConfirmationModal()
	let dbManagerDrawer = $derived(globalDbManagerDrawer.val)
	let dirtyMap = $derived.by(() => {
		const map: Record<string, boolean> = {}
		for (let i = 0; i < tempSettings.dataTables.length; i++) {
			let temp = tempSettings.dataTables[i]
			let dt = dataTableSettings.dataTables.find((d) => d.name === temp.name)
			map[temp.name] = !deepEqual(dt, temp)
		}
		return map
	})

	function onDiscard() {
		tempSettings.dataTables = $state.snapshot(dataTableSettings.dataTables)
	}

	export function discard() {
		onDiscard()
	}

	export function unsavedChanges(): { savedValue: any; modifiedValue: any } {
		return { savedValue: dataTableSettings, modifiedValue: tempSettings }
	}

	let hasUnsavedChanges = $derived.by(() => {
		return !deepEqual(dataTableSettings, tempSettings)
	})
</script>

<SettingsPageHeader
	title="Data tables"
	description="Store relational data out of the box. Interact with a fully managed PostgreSQL database directly from the Windmill SDK."
	link="https://www.windmill.dev/docs/core_concepts/persistent_storage/data_tables"
/>

<DataTable>
	<Head>
		<tr>
			{#each tableHeadNames as name, i}
				<Cell head first={i == 0} last={i == tableHeadNames.length - 1}>
					{name}
					{#if tableHeadTooltips[name]}
						<Tooltip>
							{@html tableHeadTooltips[name]}
						</Tooltip>
					{/if}
				</Cell>
			{/each}
		</tr>
	</Head>
	<tbody class="divide-y bg-surface-tertiary">
		{#if tempSettings.dataTables.length == 0}
			<Row>
				<Cell colspan={tableHeadNames.length} class="text-center py-6">
					No data table in this workspace yet
				</Cell>
			</Row>
		{/if}
		{#each tempSettings.dataTables as dataTable, dataTableIndex}
			<Row>
				<Cell first class="w-48 relative">
					<TextInput bind:value={dataTable.name} inputProps={{ placeholder: 'Name', id: 'name' }} />
				</Cell>
				<Cell>
					<div class="flex gap-2">
						<div class="relative">
							{#if dataTable.database.resource_type === 'instance'}
								<Tooltip wrapperClass="absolute mt-[0.6rem] right-2 z-20" placement="bottom-start">
									Use Windmill's PostgreSQL instance
								</Tooltip>
							{/if}
							<Select
								items={[
									{ value: 'postgresql', label: 'PostgreSQL' },
									{
										value: 'instance',
										label: 'Instance',
										subtitle: $isCustomInstanceDbEnabled ? undefined : 'Superadmin only'
									}
								]}
								bind:value={
									() => dataTable.database.resource_type,
									(resource_type) => {
										dataTable.database = {
											resource_type,
											resource_path:
												resource_type === 'instance' ? DEFAULT_DATATABLE_DB_NAME : undefined
										}
									}
								}
								id="database-type-select"
								class="w-28"
							/>
						</div>
						<div class="flex items-center gap-1 w-80 relative">
							{#if dataTable.database.resource_type !== 'instance'}
								<ResourcePicker
									class="flex-1"
									bind:value={dataTable.database.resource_path}
									resourceType={dataTable.database.resource_type}
								/>
							{:else}
								<CustomInstanceDbSelect
									class="flex-1"
									{confirmationModal}
									{dbManagerDrawer}
									{customInstanceDbs}
									bind:value={dataTable.database.resource_path}
									tag="datatable"
								/>
							{/if}
						</div>
					</div>
				</Cell>
				<Cell class="w-12">
					{#if dirtyMap[dataTable.name]}
						<Popover
							openOnHover
							contentClasses="p-2 text-sm text-secondary italic"
							class="cursor-not-allowed"
						>
							<svelte:fragment slot="trigger">
								<ExploreAssetButton
									class="h-9"
									asset={{ kind: 'datatable', path: dataTable.name }}
									{dbManagerDrawer}
									disabled
								/>
							</svelte:fragment>
							<svelte:fragment slot="content">Please save settings first</svelte:fragment>
						</Popover>
					{:else}
						<ExploreAssetButton
							class="h-9"
							asset={{ kind: 'datatable', path: dataTable.name }}
							{dbManagerDrawer}
						/>
					{/if}
				</Cell>
				<Cell class="w-12">
					<CloseButton small on:close={() => removeDataTable(dataTableIndex)} />
				</Cell>
			</Row>
		{/each}
		<Row class="!border-0">
			<Cell colspan={tableHeadNames.length} class="pt-0 pb-2">
				<div class="flex justify-center">
					<Button size="sm" btnClasses="max-w-fit" variant="default" on:click={onNewDataTable}>
						<Plus /> New Data Table
					</Button>
				</div>
			</Cell>
		</Row>
	</tbody>
</DataTable>

<SettingsFooter
	class="mt-8"
	{hasUnsavedChanges}
	{onSave}
	{onDiscard}
	saveLabel="Save data table settings"
/>

<ConfirmationModal {...confirmationModal.props} />
