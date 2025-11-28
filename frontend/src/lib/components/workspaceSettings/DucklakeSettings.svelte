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
	import { ArrowRight, TriangleAlert, Plus } from 'lucide-svelte'

	import Button from '../common/button/Button.svelte'

	import Description from '../Description.svelte'
	import { random_adj } from '../random_positive_adjetive'
	import { DataTable, Cell, Row } from '../table'
	import Head from '../table/Head.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import Select from '../select/Select.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { SettingService, WorkspaceService, type DucklakeInstanceCatalogDbStatus } from '$lib/gen'
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
	import LoggedWizardResult, { firstEmptyStepIsError } from '../wizards/LoggedWizardResult.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import { slide } from 'svelte/transition'

	const DEFAULT_DUCKLAKE_CATALOG_NAME = 'ducklake_catalog'

	type Props = {
		ducklakeSettings: DucklakeSettingsType
		ducklakeSavedSettings: DucklakeSettingsType
		onSave?: () => void
	}
	let {
		ducklakeSettings = $bindable(),
		ducklakeSavedSettings = $bindable(),
		onSave: onSaveProp = undefined
	}: Props = $props()

	let isInstanceCatalogEnabled = $derived($superadmin && !isCloudHosted())

	function onNewDucklake() {
		const name = ducklakeSettings.ducklakes.some((d) => d.name === 'main')
			? `${random_adj()}_ducklake`
			: 'main'
		ducklakeSettings.ducklakes.push({
			name,
			catalog: {
				resource_type: isInstanceCatalogEnabled ? 'instance' : 'postgresql',
				resource_path: isInstanceCatalogEnabled ? DEFAULT_DUCKLAKE_CATALOG_NAME : undefined
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

	let instanceCatalogSetupIsRunning = $state(false)
	const instanceCatalogStatuses = usePromise(SettingService.getDucklakeInstanceCatalogDbStatus, {
		clearValueOnRefresh: false
	})

	async function onSave() {
		try {
			if (
				isInstanceCatalogEnabled &&
				ducklakeSettings.ducklakes.some(
					(d) =>
						d.catalog.resource_type === 'instance' &&
						!instanceCatalogStatuses.value?.[d.catalog.resource_path ?? '']?.success
				)
			) {
				let confirm = await confirmationModal.ask({
					title: 'Some instance catalogs are not setup',
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
	let instanceCatalogPopover: Popover | undefined = $state()
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
	<div transition:slide={{ duration: 200 }} class="mb-4">
		<Alert title="Instance catalogs use the Windmill database" type="info">
			Using an instance catalog is the fastest way to get started with Ducklake. They are public to
			the instance and can be re-used in other workspaces' Ducklake settings.
		</Alert>
	</div>
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
						<Tooltip wrapperClass="absolute mt-[0.6rem] right-4" placement="bottom-start">
							The <i>main</i> ducklake can be accessed with the
							<br />
							<code class="px-1 py-0.5 border rounded-md">ATTACH 'ducklake' AS dl;</code> shorthand
						</Tooltip>
					{/if}
					<TextInput bind:value={ducklake.name} inputProps={{ placeholder: 'Name' }} />
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
										subtitle: isInstanceCatalogEnabled ? undefined : 'Superadmin only'
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
						<div class="flex items-center gap-1 w-80 relative">
							{#if ducklake.catalog.resource_type !== 'instance'}
								<ResourcePicker
									bind:value={ducklake.catalog.resource_path}
									resourceType={ducklake.catalog.resource_type}
									selectInputClass="min-h-9"
									class="min-h-9"
								/>
							{:else}
								{@const status =
									instanceCatalogStatuses.value?.[ducklake.catalog.resource_path ?? '']}
								<Select
									class="flex-1"
									inputClass="pr-20"
									bind:value={ducklake.catalog.resource_path}
									onCreateItem={(i) => (ducklake.catalog.resource_path = i)}
									placeholder="PostgreSQL database name"
									items={safeSelectItems(Object.keys(instanceCatalogStatuses.value ?? {}))}
									disabled={!isInstanceCatalogEnabled}
								/>

								<Popover
									class="absolute right-1.5"
									enableFlyTransition
									contentClasses="py-5 px-6 w-[34rem] bg-surface-secondary -translate-y-2 overflow-y-auto"
									closeOnOtherPopoverOpen
									closeOnOutsideClick
									bind:this={instanceCatalogPopover}
								>
									<svelte:fragment slot="trigger">
										<Button spacingSize="xs2" variant="default" btnClasses="h-6">
											{#if !status}
												<span class="text-yellow-600 dark:text-yellow-400">
													Setup <ArrowRight class="inline" size={14} />
												</span>
											{:else if !status.success}
												<span class="text-red-400 flex gap-1">
													Error <TriangleAlert class="inline" size={16} />
												</span>
											{:else}
												<div class="w-1.5 h-1.5 rounded-full bg-green-400"></div>
											{/if}
										</Button>
									</svelte:fragment>
									<svelte:fragment slot="content">
										{@render instanceCatalogWizard(status, ducklake.catalog.resource_path ?? '')}
									</svelte:fragment>
								</Popover>
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
							class="h-9"
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
					<Button size="sm" btnClasses="max-w-fit" variant="default" on:click={onNewDucklake}>
						<Plus /> New ducklake
					</Button>
				</div>
			</Cell>
		</Row>
	</tbody>
</DataTable>
<Button
	wrapperClasses="mt-4 mb-16 max-w-fit"
	on:click={onSave}
	disabled={ducklakeSavedSettings.ducklakes.length === ducklakeSettings.ducklakes.length &&
		Object.values(ducklakeIsDirty).every((v) => v === false)}
>
	Save ducklake settings
</Button>
<DbManagerDrawer bind:this={dbManagerDrawer} />

<ConfirmationModal {...confirmationModal.props} />

{#snippet instanceCatalogWizard(
	status: DucklakeInstanceCatalogDbStatus | undefined,
	dbname: string
)}
	{@const showManageCatalogButton =
		status?.logs.created_database === 'OK' || status?.logs.created_database === 'SKIP'}
	{#if !status}
		<div class="mb-4 text-secondary text-sm">
			{dbname} needs to be configured in the Windmill postgres instance
		</div>
	{/if}

	{#if status?.error}
		<div transition:slide={{ duration: 200 }} class="mb-4">
			<Alert title="Error setting up ducklake instance catalog" type="error">
				{status.error}
			</Alert>
		</div>
	{/if}

	<LoggedWizardResult
		class="max-h-[24rem] overflow-y-auto"
		steps={firstEmptyStepIsError(
			[
				{
					title: 'Super admin required',
					status: status?.logs.super_admin,
					description:
						'You need to be a super admin to setup an instance catalog, as it requires creating a new database in the Windmill PostgreSQL instance'
				},
				{
					title: 'Retrieve and parse database credentials',
					status: status?.logs.database_credentials,
					description:
						'Windmill uses the DATABASE_URL or DATABASE_URL_FILE environment variable to connect to the PostgreSQL instance. Make sure it is correctly set'
				},
				{
					title: 'Catalog name is valid',
					status: status?.logs.valid_dbname,
					description:
						'The catalog name must be alphanumeric (underscores allowed) and cannot be named the same as the Windmill database (usually "windmill")'
				},
				{
					title:
						'Create database' +
						(status?.logs.created_database === 'SKIP' ? ' (already exists, skipped)' : ''),
					status: status?.logs.created_database,
					description: `In the Windmill PostgreSQL instance, run: CREATE DATABASE "${dbname}".`
				},
				{
					title: `Connect to the ${dbname} database`,
					status: status?.logs.db_connect,
					description:
						"Connect to the newly created database with the default admin user (the one in DATABASE_URL, usually 'postgres') to run the next commands"
				},
				{
					title: 'Grant permissions to ducklake_user',
					status: status?.logs.grant_permissions,
					description:
						'Gives ducklake_user the required permissions to use the database as a Ducklake catalog. ducklake_user is already created during a migration and has an auto-generated password stored in global_settings.ducklake_settings.ducklake_user_pg_pwd. These are the commands : \n\n' +
						`GRANT CONNECT ON DATABASE "${dbname}" TO ducklake_user;\n` +
						'GRANT USAGE ON SCHEMA public TO ducklake_user;\n' +
						'GRANT CREATE ON SCHEMA public TO ducklake_user;\n' +
						'ALTER DEFAULT PRIVILEGES IN SCHEMA public \n' +
						'  	GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES\n    TO ducklake_user;'
				}
			],
			status?.error ?? undefined
		)}
	/>
	{#if showManageCatalogButton}
		<div class="text-primary text-xs mt-6">
			Note: the 'Manage catalog' button below is different from the Manage Ducklake button. This
			will show you the content of the PostgreSQL database used as a catalog, while the other button
			shows you the actual content of the ducklake (the parquet files).
		</div>
	{/if}
	<div class="flex gap-2 mt-2">
		<Button
			wrapperClasses="flex-1"
			size="sm"
			disabled={!isInstanceCatalogEnabled}
			onClick={async () => {
				if (instanceCatalogSetupIsRunning) return

				let wasAlreadySuccessful = status?.success ?? false
				if (status?.logs.created_database != 'OK' && status?.logs.created_database != 'SKIP') {
					instanceCatalogPopover?.close()
					let confirm = await confirmationModal.ask({
						title: 'Confirm setup',
						children: `This will create a new database ${dbname} in the Windmill PostgreSQL instance`,
						confirmationText: 'Setup catalog'
					})
					instanceCatalogPopover?.open()
					if (!confirm) return
				}

				try {
					instanceCatalogSetupIsRunning = true
					let result = await SettingService.setupDucklakeCatalogDb({ name: dbname })
					await instanceCatalogStatuses.refresh()
					if (result.success) {
						if (!wasAlreadySuccessful) sendUserToast('Setup successful')
						else sendUserToast('Check successful')
					} else {
						sendUserToast(result.error ?? 'An error occured', true)
					}
				} catch (e) {
					sendUserToast('Unexpected error, check console for details', true)
					console.error('Error setting up ducklake instance catalog', e)
				} finally {
					instanceCatalogSetupIsRunning = false
				}
			}}
			loading={instanceCatalogSetupIsRunning}
		>
			{#if !isInstanceCatalogEnabled}
				Only superadmins can setup instance catalogs
			{:else if status?.success}
				Check again
			{:else if status?.error}
				Try again
			{:else}
				Setup {dbname}
			{/if}
		</Button>
		{#if showManageCatalogButton}
			<ExploreAssetButton
				class="flex-1"
				asset={{ kind: 'resource', path: 'INSTANCE_DUCKLAKE_CATALOG/' + dbname }}
				_resourceMetadata={{ resource_type: 'postgresql' }}
				{dbManagerDrawer}
				disabled={!isInstanceCatalogEnabled}
				onClick={() => instanceCatalogPopover?.close()}
			/>
		{/if}
	</div>
{/snippet}
