<script module lang="ts">
	import { _ } from 'ag-grid-community'

	export type DucklakeMaintenanceType = {
		enabled: boolean
		schedule?: string
		retention_days?: number
		compaction?: boolean
		orphan_cleanup?: boolean
	}

	export type DucklakeSettingsType = {
		ducklakes: {
			name: string
			catalog: {
				resource_type: 'postgresql' | 'mysql' | 'instance'
				resource_path?: string // Name of the database when resource_type is instance
			}
			storage: {
				storage?: string
				path: string
			}
			extra_args?: string
			maintenance?: DucklakeMaintenanceType
		}[]
	}

	export function convertDucklakeSettingsFromBackend(
		settings: GetSettingsResponse['ducklake']
	): DucklakeSettingsType {
		const s: DucklakeSettingsType = { ducklakes: [] }
		if (settings?.ducklakes) {
			for (const [name, rest] of Object.entries(settings.ducklakes)) {
				s.ducklakes.push({ name, ...rest })
			}
		}
		return s
	}
	export function convertDucklakeSettingsToBackend(
		settings: DucklakeSettingsType
	): NonNullable<GetSettingsResponse['ducklake']> {
		const s: GetSettingsResponse['ducklake'] = { ducklakes: {} }
		for (const ducklake of settings.ducklakes) {
			const catalog = ducklake.catalog
			if (ducklake.name in s.ducklakes)
				throw 'Settings contain duplicate ducklake name: ' + ducklake.name
			if (!catalog.resource_path) throw 'No resource selected for ' + ducklake.name
			if (catalog.resource_type === 'instance' && catalog.resource_path === 'windmill')
				throw ducklake.name + ' catalog cannot be called "windmill"'
			if (ducklake.storage.path.startsWith('/'))
				ducklake.storage.path = ducklake.storage.path.slice(1)

			s.ducklakes[ducklake.name] = {
				catalog: ducklake.catalog,
				storage: ducklake.storage,
				extra_args: ducklake.extra_args || undefined,
				maintenance: ducklake.maintenance
			}
		}
		return s
	}
</script>

<script lang="ts">
	import { AlertTriangle, ExternalLink, Plus, SettingsIcon, Wrench } from 'lucide-svelte'

	import Button from '../common/button/Button.svelte'
	import SettingsFooter from './SettingsFooter.svelte'

	import Description from '../Description.svelte'
	import { random_adj } from '../random_positive_adjetive'
	import { DataTable, Cell, Row } from '../table'
	import Head from '../table/Head.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import Select from '../select/Select.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { ScheduleService, SettingService, WorkspaceService } from '$lib/gen'
	import type { GetSettingsResponse } from '$lib/gen'

	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { base } from '$app/paths'
	import Toggle from '../Toggle.svelte'
	import { sendUserToast } from '$lib/toast'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import Tooltip from '../Tooltip.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import { clone } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import { deepEqual } from 'fast-equals'
	import Popover from '../meltComponents/Popover.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { slide } from 'svelte/transition'
	import { isCustomInstanceDbEnabled, getUnusedInstanceDbName } from './utils.svelte'
	import { resource } from 'runed'
	import CustomInstanceDbSelect from './CustomInstanceDbSelect.svelte'
	import Label from '../Label.svelte'

	type Props = {
		ducklakeSettings: DucklakeSettingsType
		ducklakeSavedSettings: DucklakeSettingsType
		onSave?: () => void
		onDiscard?: () => void
	}
	let {
		ducklakeSettings = $bindable(),
		ducklakeSavedSettings = $bindable(),
		onSave: onSaveProp = undefined,
		onDiscard = undefined
	}: Props = $props()

	function defaultInstanceDbName(): string {
		const usedNames = [
			...Object.keys(customInstanceDbs.current ?? {}),
			...ducklakeSettings.ducklakes
				.filter((d) => d.catalog.resource_type === 'instance' && d.catalog.resource_path)
				.map((d) => d.catalog.resource_path!)
		]
		return getUnusedInstanceDbName('dl', $workspaceStore ?? '', usedNames)
	}

	function onNewDucklake() {
		const name = ducklakeSettings.ducklakes.some((d) => d.name === 'main')
			? `${random_adj()}_ducklake`
			: 'main'
		ducklakeSettings.ducklakes.push({
			name,
			catalog: {
				resource_type: $isCustomInstanceDbEnabled ? 'instance' : 'postgresql',
				resource_path: $isCustomInstanceDbEnabled ? defaultInstanceDbName() : undefined
			},
			storage: {
				storage: undefined,
				path: ''
			}
		})
	}

	function removeDucklake(index: number) {
		ducklakeSettings.ducklakes.splice(index, 1)
	}

	const ducklakeIsDirty: Record<string, boolean> = $derived(
		Object.fromEntries(
			ducklakeSettings.ducklakes.map((d) => {
				const saved = ducklakeSavedSettings.ducklakes.find((saved) => saved.name === d.name)
				return [d.name, !deepEqual(saved, d)] as const
			})
		)
	)

	let hasUnsavedChanges = $derived(
		ducklakeSavedSettings.ducklakes.length !== ducklakeSettings.ducklakes.length ||
			!Object.values(ducklakeIsDirty).every((v) => v === false)
	)

	const customInstanceDbs = resource([() => $workspaceStore], SettingService.listCustomInstanceDbs)

	async function onSave() {
		try {
			if (
				$isCustomInstanceDbEnabled &&
				ducklakeSettings.ducklakes.some(
					(d) =>
						d.catalog.resource_type === 'instance' &&
						!customInstanceDbs.current?.[d.catalog.resource_path ?? '']?.success
				)
			) {
				let confirm = await confirmationModal.ask({
					title: 'Some instance databases are not setup',
					children: 'Are you sure you want to save without setting them up ?',
					confirmationText: 'Save anyway'
				})
				if (!confirm) return
			}
			const settings = convertDucklakeSettingsToBackend(ducklakeSettings)
			await WorkspaceService.editDucklakeConfig({
				workspace: $workspaceStore!,
				requestBody: { settings }
			})
			ducklakeSavedSettings = clone(ducklakeSettings)
			sendUserToast('Ducklake settings saved successfully')
			onSaveProp?.()
		} catch (e) {
			sendUserToast(e, true)
			console.error('Error saving ducklake settings', e)
			throw e
		}
	}

	function editableMaintenance(
		ducklake: DucklakeSettingsType['ducklakes'][number]
	): DucklakeMaintenanceType {
		if (!ducklake.maintenance) ducklake.maintenance = { enabled: false }
		return ducklake.maintenance
	}

	// Health of the managed maintenance schedules (push failures auto-disable
	// them with an error; the schedules UI hides the reserved prefix, so this
	// is the only surface where that state is visible).
	const maintenanceHealth = resource(
		[() => $workspaceStore, () => ducklakeSavedSettings],
		async ([workspace, saved]) => {
			const out: Record<string, { enabled: boolean; error?: string }> = {}
			if (!workspace) return out
			await Promise.all(
				(saved?.ducklakes ?? [])
					.filter((d) => d.maintenance?.enabled)
					.map(async (d) => {
						try {
							const s = await ScheduleService.getSchedule({
								workspace,
								path: `f/ducklake_maintenance/${d.name}`
							})
							out[d.name] = { enabled: s.enabled ?? false, error: s.error ?? undefined }
						} catch {
							// no schedule row (e.g. save predates the feature): nothing to report
						}
					})
			)
			return out
		}
	)

	let secondaryStorageNames = usePromise(
		() => SettingService.getSecondaryStorageNames({ workspace: $workspaceStore! }),
		{ loadInit: false }
	)
	$effect(() => {
		$workspaceStore
		secondaryStorageNames.refresh()
	})

	let tableHeadNames = ['Name', 'Catalog', 'Workspace storage', 'Maintenance', '', ''] as const

	let tableHeadTooltips: Partial<Record<(typeof tableHeadNames)[number], string | undefined>> = {
		Name: "Ducklakes are referenced in DuckDB scripts with the <code class='px-1 py-0.5 border rounded-md'>ATTACH 'ducklake://name' AS dl;</code> syntax",
		Catalog: 'Ducklake needs an SQL database to store metadata about the data',
		'Workspace storage':
			'Where the data is actually stored, in parquet format. You need to configure a workspace storage first',
		Maintenance:
			'Scheduled snapshot expiry, small-file compaction and orphaned-file cleanup, run as jobs on a managed per-lake schedule (EE)'
	}

	let confirmationModal = createAsyncConfirmationModal()
</script>

<div class="flex flex-col gap-4 mb-8">
	<div class="flex flex-col gap-1">
		<div class="text-primary text-lg font-semibold">Ducklake</div>
		<Description link="https://www.windmill.dev/docs/core_concepts/persistent_storage/ducklake">
			Windmill has first class support for Ducklake. You can use and explore ducklakes like a normal
			SQL database, even though the data is actually stored in parquet files in S3 !
		</Description>
	</div>
</div>

{#if ducklakeSettings.ducklakes.some((d) => d.catalog.resource_type === 'instance')}
	<div transition:slide={{ duration: 200 }} class="mb-4">
		<Alert title="Instance databases use the Windmill database" type="info">
			Using an instance database is the fastest way to get started with Ducklake. They are public to
			the instance and can be re-used in other workspaces' Ducklake settings.
		</Alert>
	</div>
{/if}

<DataTable containerClass="ducklake-settings-table">
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
		{#if ducklakeSettings.ducklakes.length == 0}
			<Row>
				<Cell colspan={tableHeadNames.length} class="text-center py-6">
					No ducklake in this workspace yet
				</Cell>
			</Row>
		{/if}
		{#each ducklakeSettings.ducklakes as ducklake, ducklakeIndex}
			<Row>
				<Cell first class="w-44 relative">
					{#if ducklake.name === 'main'}
						<Tooltip wrapperClass="absolute mt-[0.6rem] right-4" placement="bottom-start">
							The <i>main</i> ducklake can be accessed with the
							<br />
							<code class="px-1 py-0.5 border rounded-md">ATTACH 'ducklake' AS dl;</code> shorthand
						</Tooltip>
					{/if}
					<TextInput
						bind:value={ducklake.name}
						inputProps={{ placeholder: 'Name' }}
						class="ducklake-name"
					/>
				</Cell>
				<Cell>
					<div class="flex gap-1">
						<div class="relative">
							{#if ducklake.catalog.resource_type === 'instance'}
								<Tooltip wrapperClass="absolute mt-[0.6rem] right-2 z-20" placement="bottom-start">
									Use Windmill's PostgreSQL instance as a catalog
								</Tooltip>
							{/if}
							<Select
								items={[
									{ value: 'postgresql', label: 'PostgreSQL' },
									{ value: 'mysql', label: 'MySQL' },
									{
										value: 'instance',
										label: 'Instance',
										subtitle: $isCustomInstanceDbEnabled ? undefined : 'Superadmin only'
									}
								]}
								bind:value={
									() => ducklake.catalog.resource_type,
									(resource_type) => {
										ducklake.catalog = {
											resource_type,
											resource_path:
												resource_type === 'instance' ? defaultInstanceDbName() : undefined
										}
									}
								}
								class="w-24"
							/>
						</div>
						<div class="flex flex-1">
							{#if ducklake.catalog.resource_type !== 'instance'}
								<ResourcePicker
									class="flex-1 min-w-32"
									bind:value={ducklake.catalog.resource_path}
									resourceType={ducklake.catalog.resource_type}
								/>
							{:else}
								<CustomInstanceDbSelect
									class="flex-1 min-w-32"
									bind:value={ducklake.catalog.resource_path}
									{customInstanceDbs}
									{confirmationModal}
									tag="ducklake"
								>
									{#snippet wizardBottomHint()}
										Note: this is different from the Manage Ducklake button. This will show you the
										content of the PostgreSQL database used as a catalog, while the other button
										shows you the content of the ducklake (the parquet files).
									{/snippet}
								</CustomInstanceDbSelect>
							{/if}
						</div>
					</div>
				</Cell>
				<Cell>
					<div class="flex gap-1">
						<Select
							placeholder="Default storage"
							items={[
								{ value: undefined, label: 'Default storage' },
								...(secondaryStorageNames.value?.map((value) => ({ value })) ?? [])
							]}
							bind:value={
								() => ducklake.storage.storage,
								(s) => {
									if (s) ducklake.storage.storage = s
									else delete ducklake.storage.storage
								}
							}
							class="ducklake-workspace-storage-select w-48"
							inputClass="!placeholder-secondary"
						/>
						<TextInput
							inputProps={{ placeholder: 'Data path (defaults to /)' }}
							class="ducklake-storage-data-path"
							bind:value={ducklake.storage.path}
						/>
					</div>
				</Cell>
				<Cell class="w-32">
					<Popover contentClasses="p-4" enableFlyTransition closeOnOtherPopoverOpen>
						{#snippet trigger()}
							<div class="relative">
								<Button
									variant="default"
									size="sm"
									startIcon={{ icon: Wrench }}
									btnClasses="whitespace-nowrap ducklake-maintenance-btn"
								>
									{ducklake.maintenance?.enabled
										? `${ducklake.maintenance?.retention_days ?? 7}d retention`
										: 'Off'}
								</Button>
								{#if maintenanceHealth.current?.[ducklake.name] && (maintenanceHealth.current[ducklake.name].error || !maintenanceHealth.current[ducklake.name].enabled)}
									<AlertTriangle
										size={14}
										class="absolute -top-1.5 -right-1.5 text-yellow-500 bg-surface rounded-full"
									/>
								{/if}
							</div>
						{/snippet}
						{#snippet content()}
							<div class="flex flex-col gap-3 w-96">
								<Toggle
									size="sm"
									disabled={!$enterpriseLicense && !ducklake.maintenance?.enabled}
									eeOnly
									options={{
										right: 'Scheduled maintenance',
										rightTooltip:
											'Expire old snapshots, merge adjacent small parquet files and delete orphaned files on a schedule. Runs appear as jobs — run history is the audit trail.'
									}}
									bind:checked={
										() => ducklake.maintenance?.enabled ?? false,
										(v) => (editableMaintenance(ducklake).enabled = v)
									}
								/>
								{#if ducklake.maintenance?.enabled}
									<Label
										label="Snapshot retention (days)"
										tooltip="Snapshots older than this window are expired on each maintenance run. 0 keeps only the current snapshot."
									>
										<TextInput
											inputProps={{ type: 'number', min: 0, step: 1 }}
											bind:value={
												() => ducklake.maintenance?.retention_days ?? 7,
												(v) => {
													const n = typeof v === 'number' ? v : parseInt(v ?? '')
													editableMaintenance(ducklake).retention_days = isNaN(n)
														? undefined
														: Math.max(0, n)
												}
											}
										/>
									</Label>
									{#if (ducklake.maintenance?.retention_days ?? 7) === 0}
										<p class="text-2xs text-yellow-600 dark:text-yellow-500">
											Retention 0 expires every snapshot but the current one on each run —
											time-travel is effectively disabled for this lake.
										</p>
									{/if}
									<Label
										label="Cadence (cron, UTC)"
										tooltip="v2 cron expression evaluated in UTC. Leave empty for the default: daily at 03h with a per-lake minute offset."
									>
										<TextInput
											inputProps={{ placeholder: 'daily at 03h (default)' }}
											bind:value={
												() => ducklake.maintenance?.schedule ?? '',
												(v) => (editableMaintenance(ducklake).schedule = v ? String(v) : undefined)
											}
										/>
									</Label>
									<Toggle
										size="xs"
										options={{
											right: 'Compaction',
											rightTooltip:
												'Merge adjacent small parquet files (ducklake_merge_adjacent_files)'
										}}
										bind:checked={
											() => ducklake.maintenance?.compaction ?? true,
											(v) => (editableMaintenance(ducklake).compaction = v)
										}
									/>
									<Toggle
										size="xs"
										options={{
											right: 'Orphaned file cleanup',
											rightTooltip:
												'Delete files present in storage but unknown to the catalog, older than max(retention, 1 day)'
										}}
										bind:checked={
											() => ducklake.maintenance?.orphan_cleanup ?? true,
											(v) => (editableMaintenance(ducklake).orphan_cleanup = v)
										}
									/>
									<p class="text-2xs text-secondary">
										Expired snapshots are gone for good: time-travel reads (<code
											>AT (VERSION => n)</code
										>) and recorded snapshot references older than the retention window become
										non-queryable. Freed files are physically deleted with a ~1 day lag.
									</p>
									{#if maintenanceHealth.current?.[ducklake.name]?.error || maintenanceHealth.current?.[ducklake.name]?.enabled === false}
										<Alert title="Maintenance schedule needs attention" type="warning" size="xs">
											{maintenanceHealth.current?.[ducklake.name]?.error ??
												'The managed schedule is disabled.'}
											Re-save the ducklake settings to retry.
										</Alert>
									{/if}
									{#if ducklakeSavedSettings.ducklakes.find((d) => d.name === ducklake.name)?.maintenance?.enabled}
										<a
											href={`${base}/runs/?schedule_path=${encodeURIComponent(
												`f/ducklake_maintenance/${ducklake.name}`
											)}&job_kinds=previews`}
											target="_blank"
											class="text-2xs inline-flex items-center gap-1"
										>
											View maintenance runs <ExternalLink size={12} />
										</a>
									{/if}
								{/if}
							</div>
						{/snippet}
					</Popover>
				</Cell>
				<Cell class="w-12">
					<div class="flex gap-1">
						<Popover contentClasses="p-4" enableFlyTransition closeOnOtherPopoverOpen>
							{#snippet trigger()}
								<div class="relative">
									<Button variant="default" iconOnly size="sm" endIcon={{ icon: SettingsIcon }} />
									{#if ducklake.extra_args}
										<div
											class="absolute -top-0.5 -right-0.5 w-2 h-2 bg-accent rounded-full border border-surface"
										></div>
									{/if}
								</div>
							{/snippet}
							{#snippet content()}
								<Label
									label="Extra args"
									tooltip="Additional arguments to pass in the ATTACH command. The argument list is substituted as-is. Separate them with commas."
								>
									<TextInput
										bind:value={ducklake.extra_args}
										class="min-w-96"
										underlyingInputEl="textarea"
										inputProps={{ placeholder: "METADATA_SCHEMA 'schema', ENCRYPTED true" }}
									/>
								</Label>
							{/snippet}
						</Popover>
						{#if ducklakeIsDirty[ducklake.name]}
							<Popover
								openOnHover
								contentClasses="p-2 text-sm text-secondary italic"
								class="cursor-not-allowed"
							>
								{#snippet trigger()}
									<ExploreAssetButton asset={{ kind: 'ducklake', path: '' }} disabled />
								{/snippet}
								{#snippet content()}
									Please save settings first
								{/snippet}
							</Popover>
						{:else}
							<ExploreAssetButton asset={{ kind: 'ducklake', path: ducklake.name }} />
						{/if}
					</div>
				</Cell>
				<Cell class="w-12">
					<CloseButton small on:close={() => removeDucklake(ducklakeIndex)} />
				</Cell>
			</Row>
		{/each}
		<Row class="!border-0">
			<Cell colspan={tableHeadNames.length} class="pt-0 pb-2">
				<div class="flex justify-center">
					<Button size="sm" btnClasses="max-w-fit" variant="default" on:click={onNewDucklake}>
						<Plus /> New ducklake
					</Button>
				</div>
			</Cell>
		</Row>
	</tbody>
</DataTable>
<SettingsFooter
	class="mt-6 mb-16"
	inline
	{hasUnsavedChanges}
	{onSave}
	onDiscard={() => onDiscard?.()}
	saveLabel="Save ducklake settings"
/>

<ConfirmationModal {...confirmationModal.props} />
