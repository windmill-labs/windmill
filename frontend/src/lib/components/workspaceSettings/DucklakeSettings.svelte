<script module lang="ts">
	import { _ } from 'ag-grid-community'

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
				extra_args: ducklake.extra_args || undefined
			}
		}
		return s
	}
</script>

<script lang="ts">
	import { Plus, SettingsIcon } from 'lucide-svelte'

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
	import { SettingService, WorkspaceService } from '$lib/gen'
	import type { GetSettingsResponse } from '$lib/gen'

	import { globalDbManagerDrawer, workspaceStore } from '$lib/stores'
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
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import { resource } from 'runed'
	import CustomInstanceDbSelect from './CustomInstanceDbSelect.svelte'
	import Label from '../Label.svelte'

	const DEFAULT_DUCKLAKE_CATALOG_NAME = 'ducklake_catalog'

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

	function onNewDucklake() {
		const name = ducklakeSettings.ducklakes.some((d) => d.name === 'main')
			? `${random_adj()}_ducklake`
			: 'main'
		ducklakeSettings.ducklakes.push({
			name,
			catalog: {
				resource_type: $isCustomInstanceDbEnabled ? 'instance' : 'postgresql',
				resource_path: $isCustomInstanceDbEnabled ? DEFAULT_DUCKLAKE_CATALOG_NAME : undefined
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

	const customInstanceDbs = resource([], SettingService.listCustomInstanceDbs)

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

	let secondaryStorageNames = usePromise(
		() => SettingService.getSecondaryStorageNames({ workspace: $workspaceStore! }),
		{ loadInit: false }
	)
	$effect(() => {
		$workspaceStore
		secondaryStorageNames.refresh()
	})

	let tableHeadNames = ['Name', 'Catalog', 'Workspace storage', '', ''] as const

	let tableHeadTooltips: Partial<Record<(typeof tableHeadNames)[number], string | undefined>> = {
		Name: "Ducklakes are referenced in DuckDB scripts with the <code class='px-1 py-0.5 border rounded-md'>ATTACH 'ducklake://name' AS dl;</code> syntax",
		Catalog: 'Ducklake needs an SQL database to store metadata about the data',
		'Workspace storage':
			'Where the data is actually stored, in parquet format. You need to configure a workspace storage first'
	}

	let dbManagerDrawer = $derived(globalDbManagerDrawer.val)
	let confirmationModal = createAsyncConfirmationModal()
</script>

<div class="flex flex-col gap-4 mb-8 mt-20">
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
				<Cell first class="w-48 relative">
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
					<div class="flex gap-2">
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
												resource_type === 'instance' ? DEFAULT_DUCKLAKE_CATALOG_NAME : undefined
										}
									}
								}
								class="w-28"
							/>
						</div>
						<div class="flex flex-1">
							{#if ducklake.catalog.resource_type !== 'instance'}
								<ResourcePicker
									class="flex-1"
									bind:value={ducklake.catalog.resource_path}
									resourceType={ducklake.catalog.resource_type}
								/>
							{:else}
								<CustomInstanceDbSelect
									class="flex-1"
									bind:value={ducklake.catalog.resource_path}
									{customInstanceDbs}
									{confirmationModal}
									{dbManagerDrawer}
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
					<div class="flex gap-2">
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
				<Cell class="w-12">
					<div class="flex gap-2">
						<Popover contentClasses="p-4" enableFlyTransition closeOnOtherPopoverOpen>
							<svelte:fragment slot="trigger">
								<div class="relative">
									<Button variant="default" iconOnly size="sm" endIcon={{ icon: SettingsIcon }} />
									{#if ducklake.extra_args}
										<div
											class="absolute -top-0.5 -right-0.5 w-2 h-2 bg-accent rounded-full border border-surface"
										></div>
									{/if}
								</div>
							</svelte:fragment>
							<svelte:fragment slot="content">
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
							</svelte:fragment>
						</Popover>
						{#if ducklakeIsDirty[ducklake.name]}
							<Popover
								openOnHover
								contentClasses="p-2 text-sm text-secondary italic"
								class="cursor-not-allowed"
							>
								<svelte:fragment slot="trigger">
									<ExploreAssetButton asset={{ kind: 'ducklake', path: '' }} disabled />
								</svelte:fragment>
								<svelte:fragment slot="content">Please save settings first</svelte:fragment>
							</Popover>
						{:else}
							<ExploreAssetButton
								asset={{ kind: 'ducklake', path: ducklake.name }}
								{dbManagerDrawer}
							/>
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
