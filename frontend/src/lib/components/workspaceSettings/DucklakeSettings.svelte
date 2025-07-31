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
				storage: ducklake.storage
			}
		}
		return s
	}
</script>

<script>
	import { Plus } from 'lucide-svelte'

	import Button from '../common/button/Button.svelte'

	import Description from '../Description.svelte'
	import { random_adj } from '../random_positive_adjetive'
	import { DataTable, Cell, Row } from '../table'
	import Head from '../table/Head.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import Select from '../select/Select.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { SettingService, WorkspaceService, type GetSettingsResponse } from '$lib/gen'
	import { superadmin, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import { clone, pluralize } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import { deepEqual } from 'fast-equals'
	import Popover from '../meltComponents/Popover.svelte'

	const DEFAULT_DUCKLAKE_CATALOG_NAME = 'ducklake_catalog'

	type Props = {
		ducklakeSettings: DucklakeSettingsType
		ducklakeSavedSettings: DucklakeSettingsType
	}
	let { ducklakeSettings = $bindable(), ducklakeSavedSettings = $bindable() }: Props = $props()

	let isWmDbEnabled = $derived($superadmin && !isCloudHosted())

	function onNewDucklake() {
		const name = ducklakeSettings.ducklakes.some((d) => d.name === 'main')
			? `${random_adj()}_ducklake`
			: 'main'
		ducklakeSettings.ducklakes.push({
			name,
			catalog: {
				resource_type: isWmDbEnabled ? 'instance' : 'postgresql',
				resource_path: isWmDbEnabled ? DEFAULT_DUCKLAKE_CATALOG_NAME : undefined
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

	const windmillDbNames = $derived(
		ducklakeSettings.ducklakes
			.filter((d) => d.catalog.resource_type === 'instance')
			.map((d) => d.catalog.resource_path ?? '')
	)

	const ducklakeIsDirty: Record<string, boolean> = $derived(
		Object.fromEntries(
			ducklakeSettings.ducklakes.map((d) => {
				const saved = ducklakeSavedSettings.ducklakes.find((saved) => saved.name === d.name)
				console.log('ducklakeIsDirty', d.name, saved, d)
				return [d.name, !deepEqual(saved, d)] as const
			})
		)
	)

	async function onSave() {
		try {
			if (windmillDbNames.length) {
				// Ensure that all instance dbs exist
				const nonExistentDbs = await SettingService.databasesExist({ requestBody: windmillDbNames })
				if (nonExistentDbs.length) {
					let confirmed = await confirmationModal.ask({
						title: "The following databases do not exist in Windmill's Postgres instance",
						confirmationText: `Create ${pluralize(nonExistentDbs.length, 'database')}`,
						children: `<span>
							Confirm running the following in the instance's Postgres :<br />
							${nonExistentDbs.map((db) => `<pre class='border mt-1 p-2 rounded-md'>CREATE DATABASE "${db}";</pre>`).join('\n')}
						</span>`
					})
					if (!confirmed) return
					await Promise.all(
						nonExistentDbs.map((name) => SettingService.createDucklakeDatabase({ name }))
					)
				}
			}
			const settings = convertDucklakeSettingsToBackend(ducklakeSettings)
			await WorkspaceService.editDucklakeConfig({
				workspace: $workspaceStore!,
				requestBody: { settings }
			})
			ducklakeSavedSettings = clone(ducklakeSettings)
			sendUserToast('Ducklake settings saved successfully')
		} catch (e) {
			sendUserToast(e, true)
			console.error('Error saving ducklake settings', e)
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
		Name: "Ducklakes are referenced in DuckDB scripts with the <code class='px-1 py-0.5 border rounded-md'>ATTACH 'ducklake://name'</code> syntax",
		Catalog: 'Ducklake needs an SQL database to store metadata about the data',
		'Workspace storage': 'Where the data is actually stored, in parquet format'
	}

	let dbManagerDrawer: DbManagerDrawer | undefined = $state()
	let confirmationModal = createAsyncConfirmationModal()
</script>

<div class="flex flex-col gap-4 my-8">
	<div class="flex flex-col gap-1">
		<div class="text-primary text-lg font-semibold">Ducklake</div>
		<Description link="https://www.windmill.dev/docs/core_concepts/ducklake">
			Windmill has first class support for Ducklake. You can use and explore ducklakes like a normal
			SQL database, even through the data is actually stored in parquet files in S3 !
		</Description>
	</div>
</div>

{#if ducklakeSettings.ducklakes.some((d) => d.catalog.resource_type !== 'instance')}
	<Alert title="Resource details are shared" class="mb-4" type="warning">
		Using a database resource as a catalog will share the database credentials with all users in the
		workspace.
	</Alert>
{/if}

{#if ducklakeSettings.ducklakes.some((d) => d.catalog.resource_type === 'instance')}
	<Alert title="Instance catalogs use the Windmill database" class="mb-4" type="info">
		Using an instance catalog is the fastest way to get started with Ducklake. They are public to
		the instance and can be re-used in other workspaces' Ducklake settings.
	</Alert>
{/if}

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
	<tbody class="divide-y bg-surface">
		{#if ducklakeSettings.ducklakes.length == 0}
			<Row>
				<Cell colspan={tableHeadNames.length} class="text-center">
					No ducklake in this workspace yet
				</Cell>
			</Row>
		{/if}
		{#each ducklakeSettings.ducklakes as ducklake, ducklakeIndex}
			<Row>
				<Cell first class="w-48 relative">
					{#if ducklake.name === 'main'}
						<Tooltip wrapperClass="absolute mt-2.5 right-4" placement="bottom-start">
							The <i>main</i> ducklake can be accessed with the
							<br />
							<code class="px-1 py-0.5 border rounded-md">ATTACH 'ducklake'</code> shorthand
						</Tooltip>
					{/if}
					<input bind:value={ducklake.name} placeholder="Name" />
				</Cell>
				<Cell>
					<div class="flex gap-4">
						<div class="relative">
							{#if ducklake.catalog.resource_type === 'instance'}
								<Tooltip wrapperClass="absolute mt-2.5 right-2 z-20" placement="bottom-start">
									Use Windmill's PostgreSQL instance as a catalog
								</Tooltip>
							{/if}
							<Select
								items={[
									{ value: 'postgresql', label: 'PostgreSQL' },
									{ value: 'mysql', label: 'MySQL' },
									...(isWmDbEnabled ? [{ value: 'instance', label: 'Instance' }] : [])
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
						<div class="flex items-center gap-1 w-80">
							{#if ducklake.catalog.resource_type !== 'instance'}
								<ResourcePicker
									bind:value={ducklake.catalog.resource_path}
									resourceType={ducklake.catalog.resource_type}
								/>
							{:else}
								<input
									bind:value={ducklake.catalog.resource_path}
									placeholder="PostgreSQL database name"
								/>
							{/if}
						</div>
					</div>
				</Cell>
				<Cell>
					<div class="flex gap-4">
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
								}
							}
							class="w-48"
							inputClass="!placeholder-primary"
						/>
						<input placeholder="Data path (defaults to /)" bind:value={ducklake.storage.path} />
					</div>
				</Cell>
				<Cell class="w-12">
					{#if ducklakeIsDirty[ducklake.name]}
						<Popover
							openOnHover
							contentClasses="p-2 text-sm text-secondary italic"
							class="cursor-not-allowed"
						>
							<svelte:fragment slot="trigger">
								<ExploreAssetButton
									asset={{ kind: 'ducklake', path: ducklake.name }}
									{dbManagerDrawer}
									disabled
								/>
							</svelte:fragment>
							<svelte:fragment slot="content">Please save settings first</svelte:fragment>
						</Popover>
					{:else}
						<ExploreAssetButton
							asset={{ kind: 'ducklake', path: ducklake.name }}
							{dbManagerDrawer}
						/>
					{/if}
				</Cell>
				<Cell class="w-12">
					<CloseButton small on:close={() => removeDucklake(ducklakeIndex)} />
				</Cell>
			</Row>
		{/each}
		<Row class="!border-0">
			<Cell colspan={tableHeadNames.length} class="pt-0 pb-2">
				<div class="flex justify-center">
					<Button size="sm" btnClasses="max-w-fit" variant="border" on:click={onNewDucklake}>
						<Plus /> New ducklake
					</Button>
				</div>
			</Cell>
		</Row>
	</tbody>
</DataTable>
<Button wrapperClasses="mt-4 max-w-fit" on:click={onSave}>Save ducklake settings</Button>
<DbManagerDrawer bind:this={dbManagerDrawer} />

<ConfirmationModal {...confirmationModal.props} />
