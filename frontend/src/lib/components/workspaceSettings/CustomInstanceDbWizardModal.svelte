<script lang="ts">
	import {
		SettingService,
		type CustomInstanceDb,
		type CustomInstanceDbTag,
		type ListCustomInstanceDbsResponse
	} from '$lib/gen'
	import { slide } from 'svelte/transition'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import LoggedWizardResult, { firstEmptyStepIsError } from '../wizards/LoggedWizardResult.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import type { ResourceReturn } from 'runed'
	import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import ExploreAssetButton from '../ExploreAssetButton.svelte'
	import { ArrowRight, InfoIcon } from 'lucide-svelte'
	import type { Snippet } from 'svelte'
	import { truncate } from '$lib/utils'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import { superadmin } from '$lib/stores'

	type Props = {
		customInstanceDbs: ResourceReturn<ListCustomInstanceDbsResponse>
		confirmationModal: ConfirmationModalHandle
		dbManagerDrawer: any | undefined
		bottomHint?: Snippet | undefined
		opened: { status: CustomInstanceDb | undefined; dbname: string } | undefined
		tag?: CustomInstanceDbTag
	}

	let {
		customInstanceDbs,
		confirmationModal,
		dbManagerDrawer,
		bottomHint,
		opened = $bindable(),
		tag
	}: Props = $props()

	let customInstanceDbSetupIsRunning = $state(false)
	let preventClose = false
</script>

<Modal2
	bind:isOpen={() => !!opened, (v) => !v && !preventClose && (opened = undefined)}
	target="#content"
	title={'Custom Instance Database Setup'}
	contentClasses="flex flex-col"
	fixedWidth="md"
	fixedHeight="md"
>
	{#if opened}
		{@const status = opened?.status}
		{@const dbname = opened?.dbname}
		{@const enableManageButton =
			status?.logs.created_database === 'OK' || status?.logs.created_database === 'SKIP'}
		<div class="flex h-full divide-x gap-4">
			<div class="basis-2/5 grow-0 shrink-0 flex flex-col">
				<div class="flex-1 flex flex-col">
					<span class="text-sm font-bold mb-2 overflow break-all">{dbname}</span>
					<span class="text-sm">
						Custom instance databases are databases created in the Windmill PostgreSQL instance.
						Their credentials are automatically managed by Windmill and are never exposed to users.
						Only super admins can create them.
					</span>
				</div>
				<div class="pt-6">
					{#if bottomHint}
						<div class="text-secondary text-2xs mb-2">
							{@render bottomHint()}
						</div>
					{/if}
					<ExploreAssetButton
						class="flex-1"
						asset={{ kind: 'resource', path: 'CUSTOM_INSTANCE_DB/' + dbname }}
						_resourceMetadata={{ resource_type: 'postgresql' }}
						{dbManagerDrawer}
						disabled={!$isCustomInstanceDbEnabled || !enableManageButton}
						onClick={() => (opened = undefined)}
					/>
				</div>
			</div>
			<div class="flex-1 shrink-0 flex flex-col pl-4 gap-2">
				<div class="flex-1 overflow-y-scroll">
					{#if status?.error}
						<div transition:slide={{ duration: 200 }} class="mb-4">
							<Alert title="Error setting up custom instance database" type="error">
								{status.error}
							</Alert>
						</div>
					{/if}

					<LoggedWizardResult
						steps={firstEmptyStepIsError(
							[
								{
									title: 'Super admin required',
									status: status?.logs.super_admin,
									description:
										'You need to be a super admin to create a new database in the Windmill PostgreSQL instance'
								},
								{
									title: 'Retrieve and parse database credentials',
									status: status?.logs.database_credentials,
									description:
										'Windmill uses the DATABASE_URL or DATABASE_URL_FILE environment variable to connect to the PostgreSQL instance. Make sure it is correctly set'
								},
								{
									title: 'Database name is valid',
									status: status?.logs.valid_dbname,
									description:
										'The database name must be alphanumeric (underscores allowed) and cannot be named the same as the Windmill database (usually "windmill")'
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
										'Gives custom_instance_user the required permissions to use the database. custom_instance_user is already created during a migration and has an auto-generated password stored in global_settings.custom_instance_pg_databases.user_pwd. These are the commands : \n\n' +
										`GRANT CONNECT ON DATABASE "${dbname}" TO custom_instance_user;\n` +
										'GRANT USAGE ON SCHEMA public TO custom_instance_user;\n' +
										'GRANT CREATE ON SCHEMA public TO custom_instance_user;\n' +
										`GRANT CREATE ON DATABASE "${dbname}" TO custom_instance_user;\n` +
										'ALTER DEFAULT PRIVILEGES IN SCHEMA public \n' +
										'  	GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES\n    TO custom_instance_user;\n' +
										'ALTER ROLE custom_instance_user CREATEROLE;'
								}
							],
							status?.error ?? undefined
						)}
					/>
				</div>
				{#if $superadmin}
					<Tooltip>
						<Button
							endIcon={{ icon: InfoIcon }}
							onClick={async () => {
								await SettingService.refreshCustomInstanceUserPwd()
								sendUserToast('custom_instance_user password refreshed')
							}}>Refresh custom_instance_user password</Button
						>
						{#snippet text()}
							Try this if there is an issue with your custom instance database password.
						{/snippet}
					</Tooltip>
				{/if}
				<Button
					size="sm"
					id="run-custom-instance-db-setup-button"
					variant={!status?.success ? 'accent' : 'default'}
					endIcon={status?.success ? undefined : { icon: ArrowRight }}
					disabled={!$isCustomInstanceDbEnabled}
					onClick={async () => {
						if (customInstanceDbSetupIsRunning) return

						let wasAlreadySuccessful = status?.success ?? false
						if (status?.logs.created_database != 'OK' && status?.logs.created_database != 'SKIP') {
							preventClose = true
							let confirm = await confirmationModal.ask({
								title: 'Confirm setup',
								children: `This will create a new database ${dbname} in the Windmill PostgreSQL instance`,
								confirmationText: 'Setup database'
							})
							preventClose = false
							if (!confirm) return
						}

						try {
							customInstanceDbSetupIsRunning = true
							let result = await SettingService.setupCustomInstanceDb({
								name: dbname,
								requestBody: { tag }
							})
							await customInstanceDbs.refetch()
							if (result.success) {
								if (!wasAlreadySuccessful) sendUserToast('Setup successful')
								else sendUserToast('Check successful')
							} else {
								sendUserToast(result.error ?? 'An error occurred', true)
							}
						} catch (e) {
							sendUserToast('Unexpected error, check console for details', true)
							console.error('Error setting up custom instance database', e)
						} finally {
							customInstanceDbSetupIsRunning = false
						}
					}}
					loading={customInstanceDbSetupIsRunning}
				>
					{#if !$isCustomInstanceDbEnabled}
						Only superadmins can setup custom instance databases
					{:else if status?.success}
						Check again
					{:else if status?.error}
						Try again
					{:else}
						Setup {truncate(dbname, 24)}
					{/if}
				</Button>
			</div>
		</div>
	{/if}
</Modal2>
