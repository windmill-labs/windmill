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
	import { SettingService, WorkspaceService } from '$lib/gen'
	import { type GetSettingsResponse } from '$lib/gen'

	import { superadmin, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import { clone } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import { deepEqual } from 'fast-equals'
	import Popover from '../meltComponents/Popover.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Section from '../Section.svelte'

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

	const ducklakeIsDirty: Record<string, boolean> = $derived(
		Object.fromEntries(
			ducklakeSettings.ducklakes.map((d) => {
				const saved = ducklakeSavedSettings.ducklakes.find((saved) => saved.name === d.name)
				return [d.name, !deepEqual(saved, d)] as const
			})
		)
	)

	const instanceCatalogDbNames = $derived(
		ducklakeSettings.ducklakes
			.filter((d) => d.catalog.resource_type === 'instance')
			.map((d) => d.catalog.resource_path ?? '')
	)
	const instanceCatalogStatuses = usePromise(SettingService.getDucklakeInstanceCatalogDbStatus)

	async function onSave() {
		try {
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
		Name: "Ducklakes are referenced in DuckDB scripts with the <code class='px-1 py-0.5 border rounded-md'>ATTACH 'ducklake://name' AS dl;</code> syntax",
		Catalog: 'Ducklake needs an SQL database to store metadata about the data',
		'Workspace storage':
			'Where the data is actually stored, in parquet format. You need to configure a workspace storage first'
	}

	let dbManagerDrawer: DbManagerDrawer | undefined = $state()
	let confirmationModal = createAsyncConfirmationModal()
</script>

<div class="flex flex-col gap-4 mb-8 mt-20">
	<div class="flex flex-col gap-1">
		<div class="text-primary text-lg font-semibold">Ducklake</div>
		<Description link="https://www.windmill.dev/docs/core_concepts/ducklake">
			Windmill has first class support for Ducklake. You can use and explore ducklakes like a normal
			SQL database, even though the data is actually stored in parquet files in S3 !
		</Description>
	</div>
</div>

{#if ducklakeSettings.ducklakes.some((d) => d.catalog.resource_type === 'instance')}
	<Alert title="Instance catalogs use the Windmill database" class="mb-4" type="info">
		Using an instance catalog is the fastest way to get started with Ducklake. They are public to
		the instance and can be re-used in other workspaces' Ducklake settings.
		<div>
			<Section
				label="Troubleshoot"
				collapsable
				headerClass="mt-6 border bg-surface px-3 py-1 rounded-md text-xs text-secondary"
				class="text-secondary"
				animate
			>
				This is what happens when you create a new Instance catalog with the name
				<code>ducklake_catalog</code>. This may be useful to debug issues in case the automatic
				setup fails in the middle, but in most cases Windmill will handle it for you.
				<br /><br />

				If the database <code>ducklake_catalog</code> already exists, Windmill assumes that the
				setup was successful and does not do anything. However, it is possible that it failed in the
				middle (in which case you should have seen an error pop up during setup). There is no
				rollback as the following operations do not work in a transaction.
				<br />
				This is what the setup does :
				<br /><br />
				Connect to the Windmill PostgreSQL as the default user (the one in your DATABASE_URL, usually
				'postgres') and run :
				<br />
				<code class="block p-2 border rounded-md bg-surface mt-2">
					CREATE DATABASE <code>ducklake_catalog</code>;<br />
					GRANT CONNECT ON DATABASE <code>ducklake_catalog</code> TO ducklake_user;
				</code>
				<br />
				Then, connect to the <code>ducklake_catalog</code> database with the same user as above (NOT
				ducklake_user) and run :
				<code class="block p-2 border rounded-md bg-surface mt-2">
					GRANT USAGE ON SCHEMA public TO ducklake_user;<br />
					GRANT CREATE ON SCHEMA public TO ducklake_user;<br />
					ALTER DEFAULT PRIVILEGES IN SCHEMA public<br />
					&nbsp;&nbsp;GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ducklake_user;
				</code>
				<br />
				After doing that, creating a new Ducklake with an Instance catalog named
				<code>ducklake_catalog</code> should not prompt you to run the automatic setup, and
				everything should work fine.
				<br /><br />
				Note : the ducklake_user is automatically created by Windmill in a migration. Its password is
				auto-generated and stored in the database table <code>global_settings</code> with the key
				<code>ducklake_settings.ducklake_user_pg_pwd</code>.
			</Section>
		</div>
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
						<Tooltip wrapperClass="absolute mt-3 right-4" placement="bottom-start">
							The <i>main</i> ducklake can be accessed with the
							<br />
							<code class="px-1 py-0.5 border rounded-md">ATTACH 'ducklake' AS dl;</code> shorthand
						</Tooltip>
					{/if}
					<TextInput bind:value={ducklake.name} inputProps={{ placeholder: 'Name' }} />
				</Cell>
				<Cell>
					<div class="flex gap-4">
						<div class="relative">
							{#if ducklake.catalog.resource_type === 'instance'}
								<Tooltip wrapperClass="absolute mt-3 right-2 z-20" placement="bottom-start">
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
									selectInputClass="min-h-9"
									class="min-h-9"
								/>
							{:else}
								<TextInput
									bind:value={ducklake.catalog.resource_path}
									inputProps={{ placeholder: 'PostgreSQL database name' }}
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
									else delete ducklake.storage.storage
								}
							}
							class="w-48"
							inputClass="!placeholder-secondary"
						/>
						<TextInput
							inputProps={{ placeholder: 'Data path (defaults to /)' }}
							bind:value={ducklake.storage.path}
						/>
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
									class="h-9"
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
<Button
	wrapperClasses="mt-4 mb-44 max-w-fit"
	on:click={onSave}
	disabled={ducklakeSavedSettings.ducklakes.length === ducklakeSettings.ducklakes.length &&
		Object.values(ducklakeIsDirty).every((v) => v === false)}
>
	Save ducklake settings
</Button>
<DbManagerDrawer bind:this={dbManagerDrawer} />

<ConfirmationModal {...confirmationModal.props} />
