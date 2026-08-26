<script lang="ts" module>
	import { randomUUID } from '$lib/utils/uuid'

	export type DataTableSettingsType = {
		dataTables: {
			// Stable client-side id so the UI can track renames (A -> B) across a
			// save rather than seeing them as a delete + add. Never sent to the
			// backend config.
			id: string
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
				s.dataTables.push({
					id: randomUUID(),
					name,
					...rest
				})
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
</script>

<script lang="ts">
	import { Plus, PlugZap } from 'lucide-svelte'

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
	import {
		isCustomInstanceDbEnabled,
		getUnusedInstanceDbName,
		isDataTableWizardEnabled
	} from './utils.svelte'
	import { random_adj } from '../random_positive_adjetive'
	import { sendUserToast } from '$lib/toast'
	import {
		SettingService,
		WorkspaceService,
		type GetSettingsResponse,
		type TestDataTableConnectionResponse
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { resource } from 'runed'
	import CustomInstanceDbSelect from './CustomInstanceDbSelect.svelte'
	import { Popover } from '../meltComponents'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import DataTableMigrationsButton from './DataTableMigrationsButton.svelte'
	import { deepEqual } from 'fast-equals'
	import { clone } from '$lib/utils'
	import SettingsFooter from './SettingsFooter.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import MissingWorkerTagAlert from '../jobs/MissingWorkerTagAlert.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import AddDataTableWizard from './AddDataTableWizard.svelte'
	import { takeParkedWizard, type WizardResume } from './wizardParking'
	import { Database } from 'lucide-svelte'
	import { onMount } from 'svelte'

	type Props = {
		dataTableSettings: DataTableSettingsType
	}

	let { dataTableSettings = $bindable() }: Props = $props()

	// Result of the last "Test connection", shown under the table: the grant
	// statements have to stay selectable, which rules out a toast.
	let connectionCheck = $state<
		| {
				name: string
				loading: boolean
				report?: TestDataTableConnectionResponse
				error?: string
		  }
		| undefined
	>(undefined)

	// Identifies the request the single result slot is waiting on. The data table
	// name is not enough: A -> B -> A leaves two A requests in flight, and the
	// first to be issued can be the last to land.
	let latestCheck = 0

	async function testConnection(name: string) {
		const check = ++latestCheck
		connectionCheck = { name, loading: true }
		try {
			const report = await WorkspaceService.testDataTableConnection({
				workspace: $workspaceStore ?? '',
				datatableName: name
			})
			if (check !== latestCheck) return
			connectionCheck = { name, loading: false, report }
		} catch (err) {
			if (check !== latestCheck) return
			connectionCheck = { name, loading: false, error: err?.body ?? err?.message ?? String(err) }
		}
	}

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

	const customInstanceDbs = resource([() => $workspaceStore], SettingService.listCustomInstanceDbs)

	function defaultInstanceDbName(): string {
		const usedNames = [
			...Object.keys(customInstanceDbs.current ?? {}),
			...tempSettings.dataTables
				.filter((d) => d.database.resource_type === 'instance' && d.database.resource_path)
				.map((d) => d.database.resource_path!)
		]
		return getUnusedInstanceDbName('dt', $workspaceStore ?? '', usedNames)
	}

	// Kept for the flag-off path: adding a data table is a row in this table that the user
	// fills in and saves, rather than a wizard.
	function onNewDataTable() {
		const name = tempSettings.dataTables.some((d) => d.name === 'main')
			? `${random_adj()}_datatable`
			: 'main'
		tempSettings.dataTables.push({
			id: randomUUID(),
			name,
			database: {
				resource_type: $isCustomInstanceDbEnabled ? 'instance' : 'postgresql',
				resource_path: $isCustomInstanceDbEnabled ? defaultInstanceDbName() : undefined
			}
		})
	}

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
			// Track renames/deletions by stable id (against the saved baseline) so
			// the backend can cascade or delete each data table's migrations.
			const savedById = new Map(dataTableSettings.dataTables.map((d) => [d.id, d.name]))
			const tempIds = new Set(tempSettings.dataTables.map((d) => d.id))
			const renames = tempSettings.dataTables
				.filter((d) => savedById.has(d.id) && savedById.get(d.id) !== d.name)
				.map((d) => ({ from: savedById.get(d.id)!, to: d.name }))
			const deleted_datatables = dataTableSettings.dataTables
				.filter((d) => !tempIds.has(d.id))
				.map((d) => d.name)
			await WorkspaceService.editDataTableConfig({
				workspace: $workspaceStore!,
				requestBody: { settings, renames, deleted_datatables }
			})
			dataTableSettings = clone(tempSettings)
			sendUserToast('Data table settings saved successfully')
		} catch (e) {
			sendUserToast(e, true)
			console.error('Error saving data table settings', e)
			throw e
		}
	}

	const wizardEnabled = isDataTableWizardEnabled()
	let wizardOpen = $state(false)
	/** Opened through the wizard's own `open()`, which is what sets a fresh run up. */
	let wizard: { open: (parked?: WizardResume) => void } | undefined = $state(undefined)
	let wizardResume: WizardResume | undefined = $state(undefined)

	// Supabase sends the user back here after authorizing; pick the wizard back up where it
	// was rather than making them start again.
	onMount(() => {
		if (!wizardEnabled) return
		const parked = takeParkedWizard()
		if (parked) {
			wizardResume = parked
			// Handed in, not left to the `resume` prop: the wizard rebuilds the run synchronously
			// inside this call, and a parked run that arrived late would come back as a fresh one.
			wizard?.open(parked)
		}
	})

	/**
	 * The wizard persists what it creates, so the server is authoritative afterwards and the
	 * whole baseline comes from it. `tempSettings` derives from that baseline, so this discards
	 * uncommitted edits in the table -- which is why the wizard cannot be opened while there
	 * are any (see the disabled entry points below).
	 */
	async function reloadAfterWizard() {
		const s = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		dataTableSettings = convertDataTableSettingsFromBackend(s.datatable)
		wizardResume = undefined
	}

	let confirmationModal = createAsyncConfirmationModal()
	let dirtyMap = $derived.by(() => {
		const map: Record<string, boolean> = {}
		for (let i = 0; i < tempSettings.dataTables.length; i++) {
			let temp = tempSettings.dataTables[i]
			let dt = dataTableSettings.dataTables.find((d) => d.id === temp.id)
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
	description="Relational storage the whole workspace shares under one name. Scripts, flows and apps address it as <span class='font-mono'>datatable://main</span> instead of picking a PostgreSQL resource, so nobody needs access to the credentials to query it, and you can point that name at another database without touching a line of code. Browse and edit tables, and version schema changes as migrations, from here."
	link="https://www.windmill.dev/docs/core_concepts/persistent_storage/data_tables"
/>

{#if isCloudHosted()}
	<Alert type="info" title="Instance database not available on cloud" class="mb-4" size="xs">
		On Windmill Cloud, data tables cannot use the Windmill instance database. Select
		<span class="font-semibold">PostgreSQL</span> and provide an external PostgreSQL resource (e.g. Supabase
		or Neon) instead.
	</Alert>
{/if}

<MissingWorkerTagAlert tag="postgresql" subject="Browsing and querying data tables" class="mb-4" />

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
				{#if wizardEnabled}
					<Cell colspan={tableHeadNames.length} class="py-8">
						<div class="flex flex-col items-center gap-3 text-center">
							<Database size={24} class="text-secondary" />
							<div class="flex flex-col gap-1 items-center">
								<span class="font-semibold text-sm">No data table yet</span>
								<p class="text-xs text-secondary max-w-sm">
									Give your scripts a database to store and query data.
									{#if isCloudHosted()}
										Set one up free in about a minute.
									{:else}
										Use the Windmill database, or bring your own.
									{/if}
								</p>
							</div>
							<Button
								size="sm"
								variant="accent"
								disabled={hasUnsavedChanges}
								title={hasUnsavedChanges ? 'Save or discard your changes first' : undefined}
								on:click={() => wizard?.open()}
							>
								Add a data table
							</Button>
						</div>
					</Cell>
				{:else}
					<Cell colspan={tableHeadNames.length} class="text-center py-6">
						No data table in this workspace yet
					</Cell>
				{/if}
			</Row>
		{/if}
		{#each tempSettings.dataTables as dataTable, dataTableIndex (dataTable.id)}
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
										disabled: isCloudHosted(),
										subtitle: $isCustomInstanceDbEnabled
											? undefined
											: isCloudHosted()
												? 'Not available on cloud'
												: 'Superadmin only'
									}
								]}
								bind:value={
									() => dataTable.database.resource_type,
									(resource_type) => {
										dataTable.database = {
											resource_type,
											resource_path:
												resource_type === 'instance' ? defaultInstanceDbName() : undefined
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
									{customInstanceDbs}
									bind:value={dataTable.database.resource_path}
									tag="datatable"
								/>
							{/if}
						</div>
					</div>
				</Cell>

				<Cell class="whitespace-nowrap">
					<div class="flex gap-2">
						<DataTableMigrationsButton
							workspace={$workspaceStore ?? ''}
							datatable={dataTable.name}
							disabled={!!dirtyMap[dataTable.name]}
						/>
						<Button
							size="xs"
							color="light"
							variant="border"
							startIcon={{ icon: PlugZap }}
							iconOnly
							disabled={!!dirtyMap[dataTable.name]}
							loading={connectionCheck?.name === dataTable.name && connectionCheck.loading}
							title="Test connection: check the database is reachable and its user can create tables"
							on:click={() => testConnection(dataTable.name)}
						/>
						{#if dirtyMap[dataTable.name]}
							<Popover
								openOnHover
								contentClasses="p-2 text-sm text-secondary italic"
								class="cursor-not-allowed"
							>
								{#snippet trigger()}
									<ExploreAssetButton
										asset={{ kind: 'datatable', path: dataTable.name }}
										disabled
									/>
								{/snippet}
								{#snippet content()}
									Please save settings first
								{/snippet}
							</Popover>
						{:else}
							<ExploreAssetButton asset={{ kind: 'datatable', path: dataTable.name }} />
						{/if}
					</div>
				</Cell>
				<Cell class="w-12">
					<CloseButton small on:close={() => removeDataTable(dataTableIndex)} />
				</Cell>
			</Row>
		{/each}
		{#if !wizardEnabled || tempSettings.dataTables.length > 0}
			<Row class="!border-0">
				<Cell colspan={tableHeadNames.length} class="pt-0 pb-2">
					<div class="flex justify-center">
						<Button
							size="sm"
							btnClasses="max-w-fit"
							variant="default"
							disabled={wizardEnabled && hasUnsavedChanges}
							title={wizardEnabled && hasUnsavedChanges
								? 'Save or discard your changes first'
								: undefined}
							on:click={() => (wizardEnabled ? wizard?.open() : onNewDataTable())}
						>
							<Plus />
							{wizardEnabled ? 'Add a data table' : 'New Data Table'}
						</Button>
					</div>
				</Cell>
			</Row>
		{/if}
	</tbody>
</DataTable>

{#if connectionCheck && !connectionCheck.loading}
	{@const report = connectionCheck.report}
	{#if connectionCheck.error}
		<Alert type="error" title="Could not connect to {connectionCheck.name}" class="mt-4" size="xs">
			{connectionCheck.error}
		</Alert>
	{:else if report}
		{@const fullyPrivileged = report.can_create_table && report.can_create_schema}
		<Alert
			type={fullyPrivileged ? 'success' : 'warning'}
			title={fullyPrivileged
				? `${connectionCheck.name} is reachable and its user can create tables and schemas`
				: `${connectionCheck.name} is reachable but its user is missing privileges`}
			class="mt-4"
			size="xs"
		>
			<div class="flex flex-col gap-2">
				<div>
					Connects as <span class="font-mono">{report.user}</span>{#if report.schema}, resolving
						unqualified statements to schema <span class="font-mono">{report.schema}</span>{/if}.
				</div>
				{#if report.suggested_search_path}
					<div>
						Its search_path resolves to no schema, so unqualified statements fail with
						<span class="font-mono">no schema has been selected to create in</span> whatever
						privileges the role holds. Point it at one, e.g.
						<span class="font-mono select-all">{report.suggested_search_path}</span>.
					</div>
				{/if}
				<ul class="list-disc list-inside">
					<li>
						Create tables{report.schema ? ` in ${report.schema}` : ''}:
						<span class="font-semibold">{report.can_create_table ? 'yes' : 'no'}</span>
					</li>
					<li>
						Create schemas:
						<span class="font-semibold">{report.can_create_schema ? 'yes' : 'no'}</span>
					</li>
					<li>
						Migration bookkeeping table exists:
						<span class="font-semibold">{report.migrations_table_exists ? 'yes' : 'no'}</span>
					</li>
				</ul>
				{#if report.suggested_grants.length > 0}
					<div>
						Windmill connects as the role that lacks these privileges, so it cannot grant them
						itself. Run as a schema owner or superuser on that database:
					</div>
					<pre class="whitespace-pre-wrap select-all text-xs"
						>{report.suggested_grants.map((g) => `${g};`).join('\n')}</pre
					>
					{#if report.schema && !report.can_create_table && !report.migrations_table_exists}
						<div>
							Alternatively, create the <span class="font-mono">_wm_migrations</span> bookkeeping table
							yourself and grant only SELECT, INSERT, UPDATE, DELETE on it.
						</div>
					{/if}
				{/if}
			</div>
		</Alert>
	{/if}
{/if}

<SettingsFooter
	class="mt-8"
	{hasUnsavedChanges}
	{onSave}
	{onDiscard}
	saveLabel="Save data table settings"
/>

<ConfirmationModal {...confirmationModal.props} />

{#if wizardEnabled}
	<AddDataTableWizard
		bind:this={wizard}
		bind:opened={
			() => wizardOpen,
			(v) => {
				wizardOpen = v
				// Drop the parked run once the wizard closes: leaving it set would force the next
				// open straight back to the Supabase setup step.
				if (!v) wizardResume = undefined
			}
		}
		existingNames={tempSettings.dataTables.map((d) => d.name)}
		existingDataTables={tempSettings.dataTables.map((d) => ({
			name: d.name,
			resourcePath: d.database.resource_path
		}))}
		resume={wizardResume}
		onDone={reloadAfterWizard}
		{customInstanceDbs}
		{confirmationModal}
		{defaultInstanceDbName}
	/>
{/if}
