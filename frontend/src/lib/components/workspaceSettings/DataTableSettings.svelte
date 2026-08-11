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
			origin?: DataTableOrigin | undefined
			setup_incomplete?: boolean | undefined
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
</script>

<script lang="ts">
	import { Plus, Settings, Loader2 } from 'lucide-svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import CustomInstanceDbWizardModal from './CustomInstanceDbWizardModal.svelte'

	import Button from '../common/button/Button.svelte'
	import SettingsPageHeader from '../settings/SettingsPageHeader.svelte'
	import Cell from '../table/Cell.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { getUnusedInstanceDbName } from './utils.svelte'
	import {
		SettingService,
		WorkspaceService,
		type DataTableOrigin,
		type GetSettingsResponse
	} from '$lib/gen'
	import { globalDbManagerDrawer, workspaceStore } from '$lib/stores'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { resource } from 'runed'
	import Alert from '../common/alert/Alert.svelte'
	import MissingWorkerTagAlert from '../jobs/MissingWorkerTagAlert.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import SupabaseIcon from '../icons/SupabaseIcon.svelte'
	import { Database } from 'lucide-svelte'
	import AddDataTableWizard, {
		takeParkedWizard,
		type WizardResume
	} from './AddDataTableWizard.svelte'
	import DataTableSettingsPanel from './DataTableSettingsPanel.svelte'
	import { fullyPrivileged } from './DataTableConnectionReport.svelte'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import { dataTableProvider, dataTableSubtitle } from './dataTableOrigin'
	import { useDataTableHealth } from './dataTableHealth.svelte'
	import { onMount } from 'svelte'

	type Props = {
		dataTableSettings: DataTableSettingsType
	}

	let { dataTableSettings = $bindable() }: Props = $props()

	const customInstanceDbs = resource([() => $workspaceStore], SettingService.listCustomInstanceDbs)
	const health = useDataTableHealth(() => $workspaceStore)

	let confirmationModal = createAsyncConfirmationModal()
	let panel: DataTableSettingsPanel | undefined = $state(undefined)
	let resourceEditor: ResourceEditorDrawer | undefined = $state(undefined)
	let openedInstanceDb: string | undefined = $state(undefined)
	let wizardOpen = $state(false)
	let wizardResume: WizardResume | undefined = $state(undefined)

	let tableHeadNames = ['Name', 'Database', 'Status', ''] as const
	let tableHeadTooltips: Partial<Record<(typeof tableHeadNames)[number], string | undefined>> = {
		Name: 'Data tables are referenced by their name. main is a special name that can be used as the default data table.',
		Database: 'The database where the data is stored.'
	}

	function defaultInstanceDbName(): string {
		const usedNames = [
			...Object.keys(customInstanceDbs.current ?? {}),
			...dataTableSettings.dataTables
				.filter((d) => d.database.resource_type === 'instance' && d.database.resource_path)
				.map((d) => d.database.resource_path!)
		]
		return getUnusedInstanceDbName('dt', $workspaceStore ?? '', usedNames)
	}

	// Supabase sends the user back here after authorizing; pick the wizard back up where it
	// was rather than making them start again.
	onMount(() => {
		const parked = takeParkedWizard()
		if (parked) {
			wizardResume = parked
			wizardOpen = true
		}
	})

	/** Every write in this tab lands immediately, so there is one way back to the truth. */
	async function reload() {
		const s = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		dataTableSettings = convertDataTableSettingsFromBackend(s.datatable)
		wizardResume = undefined
		await health.refetch()
	}

	// The tab writes through, so it never holds unsaved work. Kept because the settings page
	// asks every tab for one before navigating away.
	export function discard() {}
	export function unsavedChanges(): { savedValue: any; modifiedValue: any } {
		return { savedValue: {}, modifiedValue: {} }
	}

	function openManager(name: string) {
		globalDbManagerDrawer.val?.openDrawer(
			{ type: 'database', resourceType: 'postgresql', resourcePath: `datatable://${name}` },
			$workspaceStore
		)
	}
</script>

<SettingsPageHeader
	title="Data tables"
	description="Relational storage the whole workspace shares under one name. Scripts, flows and apps address it as <span class='font-mono'>datatable://main</span> instead of picking a PostgreSQL resource, so nobody needs access to the credentials to query it, and you can point that name at another database without touching a line of code. Browse and edit tables, and version schema changes as migrations, from here."
	link="https://www.windmill.dev/docs/core_concepts/persistent_storage/data_tables"
/>

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
		{#if dataTableSettings.dataTables.length == 0}
			<Row>
				<Cell colspan={tableHeadNames.length} class="py-8">
					<div class="flex flex-col items-center gap-3 text-center">
						<Database size={24} class="text-secondary" />
						<div class="flex flex-col gap-1 items-center">
							<span class="font-semibold text-sm">No database yet</span>
							<p class="text-xs text-secondary max-w-sm">
								A data table stores relational data. Give it a database to run on.
								{#if isCloudHosted()}
									Set one up free in about a minute.
								{:else}
									Use the Windmill database, or bring your own.
								{/if}
							</p>
						</div>
						<Button size="sm" variant="accent" on:click={() => (wizardOpen = true)}>
							Add a database
						</Button>
					</div>
				</Cell>
			</Row>
		{/if}
		{#each dataTableSettings.dataTables as dataTable (dataTable.id)}
			{@const provider = dataTableProvider(dataTable.database, dataTable.origin)}
			{@const status = health.current?.[dataTable.name]}
			<!-- Instance data tables have no resource: Windmill holds those credentials, and
			their setup, password rotation and drop live in the instance database modal. -->
			{@const resourcePath =
				dataTable.database.resource_type === 'postgresql'
					? dataTable.database.resource_path
					: undefined}
			{@const instanceDb =
				dataTable.database.resource_type === 'instance'
					? dataTable.database.resource_path
					: undefined}
			{@const icon = provider === 'supabase' ? SupabaseIcon : Database}
			<Row>
				<Cell first class="w-48">
					<!-- Managing the data is the daily action, so it is the row's own click.
					Connection settings are rare and sit behind the gear. A data table that is not
					usable yet opens the panel instead: there is nothing to manage. -->
					<button
						class="text-left font-medium text-xs hover:text-blue-500"
						onclick={() =>
							dataTable.setup_incomplete ? panel?.open(dataTable) : openManager(dataTable.name)}
					>
						{dataTable.name}
					</button>
				</Cell>
				<Cell>
					<div class="flex items-center gap-2 min-w-0 text-xs text-secondary">
						{#if resourcePath}
							<Button
								size="xs2"
								variant="subtle"
								wrapperClasses="min-w-0"
								btnClasses="min-w-0 font-mono text-secondary"
								startIcon={{ icon }}
								title="Edit {resourcePath}"
								on:click={() => resourceEditor?.initEdit(resourcePath)}
							>
								<span class="truncate">
									{dataTableSubtitle(dataTable.database, dataTable.origin)}
								</span>
							</Button>
						{:else if instanceDb}
							<Button
								size="xs2"
								variant="subtle"
								wrapperClasses="min-w-0"
								btnClasses="min-w-0 font-mono text-secondary"
								startIcon={{ icon }}
								title="Instance database setup for {instanceDb}"
								on:click={() => (openedInstanceDb = instanceDb)}
							>
								<span class="truncate">
									{dataTableSubtitle(dataTable.database, dataTable.origin)}
								</span>
							</Button>
						{:else}
							{@const Icon = icon}
							<Icon size={14} class="shrink-0" />
							<span class="truncate font-mono">
								{dataTableSubtitle(dataTable.database, dataTable.origin)}
							</span>
						{/if}
					</div>
				</Cell>
				<Cell class="whitespace-nowrap">
					{#if dataTable.setup_incomplete}
						<span class="inline-flex items-center gap-2 text-xs text-yellow-600">
							<span class="w-2 h-2 rounded-full bg-yellow-500 shrink-0"></span> Setup incomplete
						</span>
					{:else if health.loading && !status}
						<span class="inline-flex items-center gap-2 text-xs text-secondary">
							<Loader2 size={14} class="animate-spin" /> Checking
						</span>
					{:else if status?.ok && !fullyPrivileged(status.report)}
						<!-- Reachable, but the first migration is what would discover the missing
						grant. The panel opens on the report, which carries the GRANTs to run. -->
						<button
							class="inline-flex items-center gap-2 text-xs text-yellow-600 hover:underline"
							onclick={() => panel?.open(dataTable, status?.report)}
						>
							<span class="w-2 h-2 rounded-full bg-yellow-500 shrink-0"></span> Limited permissions
						</button>
					{:else if status?.ok}
						<span class="inline-flex items-center gap-2 text-xs text-green-600">
							<span class="w-2 h-2 rounded-full bg-green-500 shrink-0"></span> Connected
						</span>
					{:else if status}
						<span class="inline-flex items-center gap-2 text-xs text-red-500">
							<span class="w-2 h-2 rounded-full bg-red-500 shrink-0"></span> Connection failed
						</span>
					{/if}
				</Cell>
				<Cell class="whitespace-nowrap">
					<div class="flex items-center justify-end gap-2">
						<ExploreAssetButton
							asset={{ kind: 'datatable', path: dataTable.name }}
							disabled={dataTable.setup_incomplete}
						/>
						<Button
							unifiedSize="md"
							variant="default"
							startIcon={{ icon: Settings }}
							iconOnly
							title="Connection settings"
							on:click={() => panel?.open(dataTable, status?.report)}
						/>
					</div>
				</Cell>
			</Row>
		{/each}
		{#if dataTableSettings.dataTables.length > 0}
			<Row class="!border-0">
				<Cell colspan={tableHeadNames.length} class="pt-0 pb-2">
					<div class="flex justify-center">
						<Button
							size="sm"
							btnClasses="max-w-fit"
							variant="default"
							on:click={() => (wizardOpen = true)}
						>
							<Plus /> Add a database
						</Button>
					</div>
				</Cell>
			</Row>
		{/if}
	</tbody>
</DataTable>

{#if isCloudHosted()}
	<Alert type="info" title="Instance database not available on cloud" class="mt-4" size="xs">
		On Windmill Cloud, data tables cannot use the Windmill instance database. Connect Supabase or
		bring your own PostgreSQL instead.
	</Alert>
{/if}

<ConfirmationModal {...confirmationModal.props} />

<!-- Editing the connection can fix or break a data table, so re-probe when it closes. -->
<ResourceEditorDrawer bind:this={resourceEditor} on:refresh={() => health.refetch()} />

<CustomInstanceDbWizardModal
	{customInstanceDbs}
	{confirmationModal}
	tag="datatable"
	bind:opened={
		() =>
			openedInstanceDb
				? { dbname: openedInstanceDb, status: customInstanceDbs.current?.[openedInstanceDb] }
				: undefined,
		(v) => {
			if (!v) {
				openedInstanceDb = undefined
				health.refetch()
			}
		}
	}
/>

<DataTableSettingsPanel
	bind:this={panel}
	{customInstanceDbs}
	{confirmationModal}
	existingNames={dataTableSettings.dataTables.map((d) => d.name)}
	onChanged={reload}
/>

<AddDataTableWizard
	bind:opened={
		() => wizardOpen,
		(v) => {
			wizardOpen = v
			// Drop the parked run once the wizard closes: leaving it set would force the next
			// open straight back to the Supabase setup step.
			if (!v) wizardResume = undefined
		}
	}
	existingNames={dataTableSettings.dataTables.map((d) => d.name)}
	existingDataTables={dataTableSettings.dataTables.map((d) => ({
		name: d.name,
		resourcePath: d.database.resource_path,
		projectRef: d.origin?.project_ref
	}))}
	resume={wizardResume}
	onDone={reload}
	{customInstanceDbs}
	{confirmationModal}
	{defaultInstanceDbName}
/>
