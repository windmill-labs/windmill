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
				resource_type: 'postgresql' | 'instance' | 'external_instance'
				resource_path?: string | undefined
			}
			/** Set on a fork's entry: it names the workspace whose data table governs this one, and
			 * owns no database of its own. Read-only here — only forking writes it, and the server
			 * carries it across a save rather than taking it from this form. */
			reference?: { workspace_id: string; datatable: string }
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
					...rest,
					// A pointer entry owns no database. The row renders read-only in that case, so this
					// placeholder is never shown or sent.
					database: rest.database ?? { resource_type: 'instance' }
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
			// A pointer owns no database, so it has nothing to validate and nothing to send: the
			// server keeps the stored reference whatever this payload says.
			if (dataTable.reference) {
				s.datatables[dataTable.name] = {}
				continue
			}
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
	import { Database, History, KeyRound, Plus, PlugZap, Trash2 } from 'lucide-svelte'

	import Button from '../common/button/Button.svelte'

	import ResourcePicker from '../ResourcePicker.svelte'
	import SettingsPageHeader from '../settings/SettingsPageHeader.svelte'
	import Select from '../select/Select.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Tooltip from '../Tooltip.svelte'
	import {
		isCustomInstanceDbEnabled,
		managedInstanceLabels,
		shortManagedInstanceLabel,
		getUnusedInstanceDbName
	} from './utils.svelte'
	import { sendUserToast } from '$lib/toast'
	import {
		SettingService,
		WorkspaceService,
		type GetSettingsResponse,
		type TestDataTableConnectionResponse
	} from '$lib/gen'
	import { enterpriseLicense, superadmin, workspaceStore } from '$lib/stores'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { resource } from 'runed'
	import CustomInstanceDbSelect from './CustomInstanceDbSelect.svelte'
	import ExternalInstanceDbSelect from './ExternalInstanceDbSelect.svelte'
	import { Popover } from '../meltComponents'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import DataTableMigrationsButton from './DataTableMigrationsButton.svelte'
	import DataTablePermissionsButton from './DataTablePermissionsButton.svelte'
	import InstanceRolesButton from './InstanceRolesButton.svelte'
	import { deepEqual } from 'fast-equals'
	import { apiErrorMessage, clone } from '$lib/utils'
	import SettingsFooter from './SettingsFooter.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import EmptyState from '../common/emptyState/EmptyState.svelte'
	import Label from '../Label.svelte'
	import MissingWorkerTagAlert from '../jobs/MissingWorkerTagAlert.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import AddDataTableWizard from './AddDataTableWizard.svelte'
	import DataTableConnectionReport from './DataTableConnectionReport.svelte'
	import { takeParkedWizard, type WizardResume } from './wizardParking'
	import { onMount } from 'svelte'

	type Props = {
		dataTableSettings: DataTableSettingsType
	}

	let { dataTableSettings = $bindable() }: Props = $props()

	// Result of the last "Test connection", shown in that data table's card: the grant
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

	let tempSettings: DataTableSettingsType = $derived.by(() => {
		let s = $state($state.snapshot(dataTableSettings))
		return s
	})

	function removeDataTable(index: number) {
		tempSettings.dataTables.splice(index, 1)
	}

	const customInstanceDbs = resource([() => $workspaceStore], SettingService.listCustomInstanceDbs)

	// Both endpoints are superadmin-only, and the kind is theirs to pick, so a workspace admin
	// never loads them — and sees the option disabled rather than an empty picker.
	const externalInstanceStatus = resource([() => $superadmin], ([isSuperadmin]) =>
		isSuperadmin ? SettingService.getExternalInstancePgStatus() : Promise.resolve(undefined)
	)
	const externalInstanceDbs = resource([() => $superadmin], ([isSuperadmin]) =>
		isSuperadmin ? SettingService.listExternalInstancePgDatabases() : Promise.resolve({})
	)
	let externalInstanceConfigured = $derived(externalInstanceStatus.current?.configured === true)
	// Superadmin-only like the ones above, and absent means on.
	const instancePgDisabled = resource([() => $superadmin], ([isSuperadmin]) =>
		isSuperadmin
			? SettingService.getGlobal({ key: 'instance_pg_disabled' }).catch(() => undefined)
			: Promise.resolve(undefined)
	)
	// Both substrates answer only to a superadmin, so nobody else can be told whether one is on
	// offer: they see a managed kind only where an entry already sits on it.
	let instancePossible = $derived(
		!!$superadmin && !isCloudHosted() && !instancePgDisabled.current
	)
	let instanceAvailable = $derived(instancePossible)

	// A kind already saved stays listed whatever the instance offers now, or the entry would read
	// as something it is not.
	// Qualified per form, not per row: two rows, one on each substrate, would otherwise both
	// read `Managed instance` and the label would say nothing.
	let anyInstanceRow = $derived(
		tempSettings.dataTables.some((d) => d.database.resource_type === 'instance')
	)
	let anyExternalRow = $derived(
		tempSettings.dataTables.some((d) => d.database.resource_type === 'external_instance')
	)
	function kindItems(current: string | undefined) {
		const showInstance = instancePossible || current === 'instance'
		const showExternal =
			(externalInstanceConfigured && !!$superadmin) || current === 'external_instance'
		const labels = managedInstanceLabels(
			instancePossible || anyInstanceRow,
			(externalInstanceConfigured && !!$superadmin) || anyExternalRow
		)
		const items: { value: string; label: string; disabled?: boolean; subtitle?: string }[] = [
			{ value: 'postgresql', label: 'Postgres Resource' }
		]
		if (showInstance) {
			items.push({
				value: 'instance',
				label: labels.instance,
				disabled: !instanceAvailable,
				subtitle: instanceAvailable
					? undefined
					: !$superadmin
						? 'Superadmin only'
						: isCloudHosted()
							? 'Not available on cloud'
							: "Windmill's database is disabled"
			})
		}
		if (showExternal) {
			items.push({
				value: 'external_instance',
				label: labels.external,
				disabled: !externalInstanceConfigured || !$superadmin,
				subtitle: !$superadmin
					? 'Superadmin only'
					: externalInstanceConfigured
						? undefined
						: 'No external cluster configured'
			})
		}
		return items
	}

	function defaultExternalDbName(): string {
		const usedNames = [
			...Object.keys(externalInstanceDbs.current ?? {}),
			...tempSettings.dataTables
				.filter((d) => d.database.resource_type === 'external_instance' && d.database.resource_path)
				.map((d) => d.database.resource_path!)
		]
		return getUnusedInstanceDbName('dt', $workspaceStore ?? '', usedNames)
	}

	function defaultInstanceDbName(): string {
		const usedNames = [
			...Object.keys(customInstanceDbs.current ?? {}),
			...tempSettings.dataTables
				.filter((d) => d.database.resource_type === 'instance' && d.database.resource_path)
				.map((d) => d.database.resource_path!)
		]
		return getUnusedInstanceDbName('dt', $workspaceStore ?? '', usedNames)
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
			const result = await WorkspaceService.editDataTableConfig({
				workspace: $workspaceStore!,
				requestBody: { settings, renames, deleted_datatables }
			})
			dataTableSettings = clone(tempSettings)
			// A delete can leave another workspace's data table governed by nothing. Swallowing
			// that is what made it silent for the person who caused it.
			const stranded = result?.stranded_references ?? []
			if (stranded.length > 0) {
				sendUserToast(
					`These data tables were governed by one you deleted and no longer resolve: ${stranded
						.map((s) => `${s.workspace_id}/${s.datatable}`)
						.join(', ')}. Their databases still exist; a superadmin can point them at another ` +
						`workspace's data table.`,
					'warning',
					[],
					undefined,
					20000
				)
			} else {
				sendUserToast('Data table settings saved successfully')
			}
		} catch (e) {
			sendUserToast(apiErrorMessage(e), true)
			console.error('Error saving data table settings', e)
			throw e
		}
	}

	let wizardOpen = $state(false)
	/** Opened through the wizard's own `open()`, which is what sets a fresh run up. */
	let wizard: { open: (parked?: WizardResume) => void } | undefined = $state(undefined)
	let wizardResume: WizardResume | undefined = $state(undefined)

	// Supabase sends the user back here after authorizing; pick the wizard back up where it
	// was rather than making them start again.
	onMount(() => {
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
	 * uncommitted edits in the cards -- which is why the wizard cannot be opened while there
	 * are any (see the disabled entry points below).
	 */
	async function reloadAfterWizard() {
		const s = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		dataTableSettings = convertDataTableSettingsFromBackend(s.datatable)
		wizardResume = undefined
	}

	let confirmationModal = createAsyncConfirmationModal()
	// Each mounts its own modal or drawer; the card's buttons open them.
	let migrationsButtons = $state<Record<string, DataTableMigrationsButton | undefined>>({})
	let permissionsButtons = $state<Record<string, DataTablePermissionsButton | undefined>>({})
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
>
	{#snippet actions()}
		<InstanceRolesButton
			unavailable={!$enterpriseLicense
				? { reason: 'Instance roles are an Enterprise Edition feature.', ee: true }
				: isCloudHosted()
					? { reason: 'Instance roles are only available on self-hosted instances.', ee: false }
					: !$superadmin
						? { reason: 'Only instance superadmins can manage instance roles.', ee: false }
						: undefined}
		/>
	{/snippet}
</SettingsPageHeader>

{#if isCloudHosted()}
	<Alert type="info" title="Instance database not available on cloud" class="mb-4" size="xs">
		On Windmill Cloud, data tables cannot use the Windmill instance database. Select
		<span class="font-semibold">PostgreSQL</span> and provide an external PostgreSQL resource (e.g. Supabase
		or Neon) instead.
	</Alert>
{/if}

<MissingWorkerTagAlert tag="postgresql" subject="Browsing and querying data tables" class="mb-4" />

{#if tempSettings.dataTables.length == 0}
	<EmptyState
		icon={Database}
		title="No data table yet"
		description={`Give your scripts a database to store and query data. ${
			isCloudHosted()
				? 'Set one up free in about a minute.'
				: 'Use the Windmill database, or bring your own.'
		}`}
		action={{
			label: 'Add a data table',
			icon: Plus,
			variant: 'accent',
			disabled: hasUnsavedChanges,
			title: hasUnsavedChanges ? 'Save or discard your changes first' : undefined,
			onClick: () => wizard?.open()
		}}
	/>
{:else}
	<div class="flex flex-col gap-4">
		{#each tempSettings.dataTables as dataTable, dataTableIndex (dataTable.id)}
			{@const dirty = !!dirtyMap[dataTable.name]}
			<div class="rounded-md border border-border-light bg-surface-tertiary">
				<div class="flex flex-col gap-4 p-4">
					<div class="flex items-start gap-4">
						<Label
							label="Name"
							class="flex-1"
							tooltip="Data tables are referenced by their name. main is a special name that can be used as the default data table."
						>
							{#if dataTable.reference}
								<span class="font-mono text-sm">{dataTable.name}</span>
							{:else}
								<TextInput
									class="max-w-64"
									bind:value={dataTable.name}
									inputProps={{ placeholder: 'Name', id: 'name' }}
								/>
							{/if}
						</Label>
						<!-- A fork's pointer entry is written by forking and kept by the server, not this form. -->
						{#if !dataTable.reference}
							<Button
								unifiedSize="md"
								variant="subtle"
								destructive
								startIcon={{ icon: Trash2 }}
								iconOnly
								title="Remove"
								on:click={() => removeDataTable(dataTableIndex)}
							/>
						{/if}
					</div>
					<Label label="Database" tooltip="The database where the data is stored.">
						{#if dataTable.reference}
							<div class="flex items-center gap-1 text-sm text-secondary">
								<span>Governed by</span>
								<span class="font-mono">{dataTable.reference.workspace_id}</span>
								<span>/</span>
								<span class="font-mono">{dataTable.reference.datatable}</span>
								<Tooltip>
									This fork uses its parent's data table rather than a copy of it, so the database and
									its roles are decided in that workspace.
								</Tooltip>
							</div>
						{:else}
							<div class="flex gap-2">
								<div class="relative">
									{#if dataTable.database.resource_type === 'instance'}
										<Tooltip
											wrapperClass="absolute mt-[0.6rem] right-2 z-20"
											placement="bottom-start"
										>
											Use Windmill's PostgreSQL instance
										</Tooltip>
									{:else if dataTable.database.resource_type === 'external_instance'}
										<Tooltip
											wrapperClass="absolute mt-[0.6rem] right-2 z-20"
											placement="bottom-start"
										>
											Use a database Windmill manages on the external PostgreSQL cluster
										</Tooltip>
									{/if}
									<Select
										items={kindItems(dataTable.database.resource_type)}
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
										transformInputSelectedText={shortManagedInstanceLabel}
										id="database-type-select"
										class="w-44"
									/>
								</div>
								<div class="flex items-center gap-1 w-80 relative">
									{#if dataTable.database.resource_type === 'external_instance'}
										<ExternalInstanceDbSelect
											class="flex-1"
											{externalInstanceDbs}
											bind:value={dataTable.database.resource_path}
											tag="datatable"
										/>
									{:else if dataTable.database.resource_type !== 'instance'}
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
						{/if}
					</Label>
					{#if connectionCheck?.name === dataTable.name && !connectionCheck.loading}
						<DataTableConnectionReport
							name={connectionCheck.name}
							report={connectionCheck.report}
							error={connectionCheck.error}
						/>
					{/if}
				</div>
				<!-- Everything down here acts on the saved data table, which unsaved edits are not. -->
				<div class="flex flex-wrap items-center gap-2 border-t border-border-light px-4 py-3">
					{#if dirty}
						<Popover
							openOnHover
							contentClasses="p-2 text-sm text-secondary italic"
							class="cursor-not-allowed"
						>
							{#snippet trigger()}
								<ExploreAssetButton asset={{ kind: 'datatable', path: dataTable.name }} disabled />
							{/snippet}
							{#snippet content()}
								Please save settings first
							{/snippet}
						</Popover>
					{:else}
						<ExploreAssetButton asset={{ kind: 'datatable', path: dataTable.name }} />
					{/if}
					<Button
						unifiedSize="md"
						variant="default"
						startIcon={{ icon: History }}
						disabled={dirty}
						title={dirty ? 'Save the settings first' : 'Version schema changes as migrations'}
						on:click={() => migrationsButtons[dataTable.name]?.open()}
					>
						Migrations
					</Button>
					<!-- Shown even where it cannot be used, disabled with the reason, so the feature can be
					found. -->
					<Button
						unifiedSize="md"
						variant="default"
						startIcon={{ icon: KeyRound }}
						disabled={!$enterpriseLicense || isCloudHosted() || dirty}
						title={!$enterpriseLicense
							? 'Data table roles are an Enterprise Edition feature.'
							: isCloudHosted()
								? 'Data table roles are only available on self-hosted instances.'
								: dirty
									? 'Save the settings first'
									: 'Roles: who may connect as which Postgres role'}
						on:click={() => permissionsButtons[dataTable.name]?.open()}
					>
						{$enterpriseLicense ? 'Roles' : 'Roles (EE)'}
					</Button>
					<Button
						unifiedSize="md"
						variant="default"
						startIcon={{ icon: PlugZap }}
						disabled={dirty}
						loading={connectionCheck?.name === dataTable.name && connectionCheck.loading}
						title="Check the database is reachable and its user can create tables"
						on:click={() => testConnection(dataTable.name)}
					>
						Test connection
					</Button>
				</div>
				<DataTableMigrationsButton
					bind:this={migrationsButtons[dataTable.name]}
					hideTrigger
					workspace={$workspaceStore ?? ''}
					datatable={dataTable.name}
				/>
				{#if $enterpriseLicense && !isCloudHosted()}
					<DataTablePermissionsButton
						bind:this={permissionsButtons[dataTable.name]}
						hideTrigger
						workspace={$workspaceStore ?? ''}
						datatable={dataTable.name}
					/>
				{/if}
			</div>
		{/each}
		<div class="flex justify-center">
			<Button
				unifiedSize="md"
				variant="default"
				startIcon={{ icon: Plus }}
				disabled={hasUnsavedChanges}
				title={hasUnsavedChanges ? 'Save or discard your changes first' : undefined}
				on:click={() => wizard?.open()}
			>
				Add a data table
			</Button>
		</div>
	</div>
{/if}

<SettingsFooter
	class="mt-8"
	{hasUnsavedChanges}
	{onSave}
	{onDiscard}
	saveLabel="Save data table settings"
/>

<ConfirmationModal {...confirmationModal.props} />

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
	{externalInstanceDbs}
	externalInstanceAvailable={externalInstanceConfigured && !!$superadmin}
	{instanceAvailable}
	offerExternalSetup
	refreshManagedInstances={() => {
		externalInstanceStatus.refetch()
		externalInstanceDbs.refetch()
		instancePgDisabled.refetch()
	}}
	{defaultExternalDbName}
	{confirmationModal}
	{defaultInstanceDbName}
/>
