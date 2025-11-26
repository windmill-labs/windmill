<script lang="ts">
	import {
		SettingService,
		type CustomInstanceDbStatus,
		type GetCustomInstanceDbStatusResponse
	} from '$lib/gen'
	import { slide } from 'svelte/transition'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import LoggedWizardResult, { firstEmptyStepIsError } from '../wizards/LoggedWizardResult.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import type { ResourceReturn } from 'runed'
	import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import type DBManagerDrawer from '../DBManagerDrawer.svelte'

	type Props = {
		instanceCatalogStatuses: ResourceReturn<GetCustomInstanceDbStatusResponse>
		confirmationModal: ConfirmationModalHandle
		dbManagerDrawer: DBManagerDrawer | undefined
		opened: { status: CustomInstanceDbStatus | undefined; dbname: string } | undefined
	}

	let {
		instanceCatalogStatuses,
		confirmationModal,
		dbManagerDrawer,
		opened = $bindable()
	}: Props = $props()

	let instanceCatalogSetupIsRunning = $state(false)
	let preventClose = false
</script>

<Modal2
	bind:isOpen={() => !!opened, (v) => !v && !preventClose && (opened = undefined)}
	target="#content"
	title={opened?.dbname ?? '??'}
	contentClasses="flex flex-col"
	fixedWidth="sm"
	fixedHeight="lg"
>
	{#if opened}
		{@const status = opened?.status}
		{@const dbname = opened?.dbname}
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
			class="overflow-y-auto"
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
						title: 'Grant permissions to custom_instance_user',
						status: status?.logs.grant_permissions,
						description:
							'Gives custom_instance_user the required permissions to use the database as a Ducklake catalog. custom_instance_user is already created during a migration and has an auto-generated password stored in global_settings.custom_instance_pg_databases.user_pwd. These are the commands : \n\n' +
							`GRANT CONNECT ON DATABASE "${dbname}" TO custom_instance_user;\n` +
							'GRANT USAGE ON SCHEMA public TO custom_instance_user;\n' +
							'GRANT CREATE ON SCHEMA public TO custom_instance_user;\n' +
							'ALTER DEFAULT PRIVILEGES IN SCHEMA public \n' +
							'  	GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES\n    TO custom_instance_user;'
					}
				],
				status?.error ?? undefined
			)}
		/>
		<div class="mt-auto pt-6">
			{#if showManageCatalogButton}
				<div class="text-primary text-xs">
					Note: the 'Manage catalog' button below is different from the Manage Ducklake button. This
					will show you the content of the PostgreSQL database used as a catalog, while the other
					button shows you the actual content of the ducklake (the parquet files).
				</div>
			{/if}
			<div class="flex gap-2 mt-2">
				<Button
					wrapperClasses="flex-1"
					size="sm"
					disabled={!$isCustomInstanceDbEnabled}
					onClick={async () => {
						if (instanceCatalogSetupIsRunning) return

						let wasAlreadySuccessful = status?.success ?? false
						if (status?.logs.created_database != 'OK' && status?.logs.created_database != 'SKIP') {
							preventClose = true
							let confirm = await confirmationModal.ask({
								title: 'Confirm setup',
								children: `This will create a new database ${dbname} in the Windmill PostgreSQL instance`,
								confirmationText: 'Setup catalog'
							})
							preventClose = false
							if (!confirm) return
						}

						try {
							instanceCatalogSetupIsRunning = true
							let result = await SettingService.setupCustomInstanceDb({ name: dbname })
							await instanceCatalogStatuses.refetch()
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
					{#if !$isCustomInstanceDbEnabled}
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
						asset={{ kind: 'resource', path: 'CUSTOM_INSTANCE_DB/' + dbname }}
						_resourceMetadata={{ resource_type: 'postgresql' }}
						{dbManagerDrawer}
						disabled={!$isCustomInstanceDbEnabled}
						onClick={() => (opened = undefined)}
					/>
				{/if}
			</div>
		</div>
	{/if}
</Modal2>

<ConfirmationModal {...confirmationModal.props} />
