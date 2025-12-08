<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import { isCloudHosted } from '$lib/cloud'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Button, Skeleton, Tab, Tabs } from '$lib/components/common'

	import DeployToSetting from '$lib/components/DeployToSetting.svelte'
	import ErrorOrRecoveryHandler from '$lib/components/ErrorOrRecoveryHandler.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import WorkspaceUserSettings from '$lib/components/settings/WorkspaceUserSettings.svelte'
	import { WORKSPACE_SHOW_SLACK_CMD, WORKSPACE_SHOW_WEBHOOK_CLI_SYNC } from '$lib/consts'
	import {
		OauthService,
		WorkspaceService,
		ResourceService,
		SettingService,
		type AIConfig,
		type ErrorHandler
	} from '$lib/gen'
	import {
		enterpriseLicense,
		superadmin,
		userStore,
		usersWorkspaceStore,
		workspaceStore,
		isCriticalAlertsUIOpen
	} from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { clone, emptyString } from '$lib/utils'
	import { RotateCw, Trash2, Slack, Save } from 'lucide-svelte'

	import PremiumInfo from '$lib/components/settings/PremiumInfo.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	import ChangeWorkspaceName from '$lib/components/settings/ChangeWorkspaceName.svelte'
	import ChangeWorkspaceId from '$lib/components/settings/ChangeWorkspaceId.svelte'
	import ChangeWorkspaceColor from '$lib/components/settings/ChangeWorkspaceColor.svelte'
	import {
		convertBackendSettingsToFrontendSettings,
		type S3ResourceSettings
	} from '$lib/workspace_settings'
	import { base } from '$lib/base'
	import Description from '$lib/components/Description.svelte'
	import ConnectionSection from '$lib/components/ConnectionSection.svelte'
	import AISettings from '$lib/components/workspaceSettings/AISettings.svelte'
	import StorageSettings from '$lib/components/workspaceSettings/StorageSettings.svelte'
	import GitSyncSection from '$lib/components/git_sync/GitSyncSection.svelte'
	import { untrack } from 'svelte'
	import { getHandlerType } from '$lib/components/triggers/utils'
	import DucklakeSettings, {
		convertDucklakeSettingsFromBackend,
		type DucklakeSettingsType
	} from '$lib/components/workspaceSettings/DucklakeSettings.svelte'
	import { AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import CollapseLink from '$lib/components/CollapseLink.svelte'
	import DataTableSettings, {
		convertDataTableSettingsFromBackend,
		type DataTableSettingsType
	} from '$lib/components/workspaceSettings/DataTableSettings.svelte'
	import WorkspaceDependenciesSettings from '$lib/components/workspaceSettings/WorkspaceDependenciesSettings.svelte'

	let slackInitialPath: string = $state('')
	let slackScriptPath: string = $state('')
	let teamsInitialPath: string = $state('')
	let teamsScriptPath: string = $state('')
	let slack_team_name: string | undefined = $state()
	let teams_team_id: string | undefined = $state()
	let teams_team_name: string | undefined = $state()
	let useCustomSlackApp: boolean = $state(false)
	let slackOAuthClientId: string = $state('')
	let slackOAuthClientSecret: string = $state('')
	let slackOAuthConfigLoaded: boolean = $state(false)
	let itemKind: 'flow' | 'script' = $state('flow')
	let plan: string | undefined = $state(undefined)
	let customer_id: string | undefined = $state(undefined)
	let webhook: string | undefined = $state(undefined)
	let workspaceToDeployTo: string | undefined = $state(undefined)
	let errorHandlerSelected: ErrorHandler = $state('slack')
	let errorHandlerScriptPath: string | undefined = $state(undefined)
	let errorHandlerItemKind: 'flow' | 'script' = $state('script')
	let errorHandlerExtraArgs: Record<string, any> = $state({})
	let errorHandlerMutedOnCancel: boolean | undefined = $state(undefined)
	let criticalAlertUIMuted: boolean | undefined = $state(undefined)
	let initialCriticalAlertUIMuted: boolean | undefined = $state(undefined)

	let aiProviders: Exclude<AIConfig['providers'], undefined> = $state({})
	let codeCompletionModel: string | undefined = $state(undefined)
	let defaultModel: string | undefined = $state(undefined)
	let customPrompts: Record<string, string> = $state({})
	let maxTokensPerModel: Record<string, number> = $state({})

	// Track initial AI config for unsaved changes detection
	let initialAiProviders: Exclude<AIConfig['providers'], undefined> = $state({})
	let initialCodeCompletionModel: string | undefined = $state(undefined)
	let initialDefaultModel: string | undefined = $state(undefined)
	let initialCustomPrompts: Record<string, string> = $state({})
	let initialMaxTokensPerModel: Record<string, number> = $state({})

	let s3ResourceSettings: S3ResourceSettings = $state({
		resourceType: 's3',
		resourcePath: undefined,
		publicResource: undefined,
		secondaryStorage: undefined
	})
	let initialS3ResourceSettings: S3ResourceSettings = $state({
		resourceType: 's3',
		resourcePath: undefined,
		publicResource: undefined,
		secondaryStorage: undefined
	})

	let dataTableSettings: DataTableSettingsType = $state({ dataTables: [] })
	let dataTableSettingsComponent: DataTableSettings | undefined = $state(undefined)

	let ducklakeSettings: DucklakeSettingsType = $state({ ducklakes: [] })
	let ducklakeSavedSettings: DucklakeSettingsType = $state(untrack(() => ducklakeSettings))

	let workspaceDefaultAppPath: string | undefined = $state(undefined)
	let workspaceEncryptionKey: string | undefined = $state(undefined)
	let editedWorkspaceEncryptionKey: string | undefined = $state(undefined)
	let workspaceReencryptionInProgress: boolean = $state(false)
	let encryptionKeyRegex = /^[a-zA-Z0-9]{64}$/
	let slack_tabs: 'slack_commands' | 'teams_commands' = $state('slack_commands')
	let tab = $state(
		($page.url.searchParams.get('tab') as
			| 'users'
			| 'slack'
			| 'premium'
			| 'general'
			| 'webhook'
			| 'deploy_to'
			| 'error_handler'
			| 'ai'
			| 'windmill_data_tables'
			| 'windmill_lfs'
			| 'git_sync'
			| 'default_app'
			| 'encryption'
			| 'dependencies') ?? 'users'
	)
	let usingOpenaiClientCredentialsOauth = $state(false)

	let loadedSettings = $state(false)

	async function editWorkspaceCommand(platform: 'slack' | 'teams'): Promise<void> {
		if (platform === 'slack') {
			if (slackInitialPath === slackScriptPath) return
			slackInitialPath = slackScriptPath
		} else {
			if (teamsInitialPath === teamsScriptPath) return
			teamsInitialPath = teamsScriptPath
		}

		let scriptPath = platform === 'slack' ? slackScriptPath : teamsScriptPath
		let commandScriptKey = platform === 'slack' ? 'slack_command_script' : 'teams_command_script'
		let updateCommandScript =
			platform === 'slack' ? WorkspaceService.editSlackCommand : WorkspaceService.editTeamsCommand

		if (scriptPath) {
			await updateCommandScript({
				workspace: $workspaceStore!,
				requestBody: { [commandScriptKey]: `${itemKind}/${scriptPath}` }
			})
			sendUserToast(`${platform} command script set to ${scriptPath}`)
		} else {
			await WorkspaceService.editSlackCommand({
				workspace: $workspaceStore!,
				requestBody: { [commandScriptKey]: undefined }
			})
			sendUserToast(`${platform} command script removed`)
		}
	}

	async function editSlackCommand(): Promise<void> {
		await editWorkspaceCommand('slack')
	}

	async function editTeamsCommand(): Promise<void> {
		await editWorkspaceCommand('teams')
	}

	async function editWebhook(): Promise<void> {
		// in JS, an empty string is also falsy
		if (webhook) {
			await WorkspaceService.editWebhook({
				workspace: $workspaceStore!,
				requestBody: { webhook }
			})
			sendUserToast(`webhook set to ${webhook}`)
		} else {
			await WorkspaceService.editWebhook({
				workspace: $workspaceStore!,
				requestBody: { webhook: undefined }
			})
			sendUserToast(`webhook removed`)
		}
	}

	async function editWorkspaceDefaultApp(appPath: string | undefined): Promise<void> {
		if (emptyString(appPath)) {
			await WorkspaceService.editWorkspaceDefaultApp({
				workspace: $workspaceStore!,
				requestBody: {
					default_app_path: undefined
				}
			})
			sendUserToast('Workspace default app reset')
		} else {
			await WorkspaceService.editWorkspaceDefaultApp({
				workspace: $workspaceStore!,
				requestBody: {
					default_app_path: appPath
				}
			})
			sendUserToast('Workspace default app set')
		}
	}

	async function loadWorkspaceEncryptionKey(): Promise<void> {
		let resp = await WorkspaceService.getWorkspaceEncryptionKey({
			workspace: $workspaceStore!
		})
		workspaceEncryptionKey = resp.key
		editedWorkspaceEncryptionKey = resp.key
	}

	async function setWorkspaceEncryptionKey(): Promise<void> {
		if (
			emptyString(editedWorkspaceEncryptionKey) ||
			workspaceEncryptionKey === editedWorkspaceEncryptionKey
		) {
			return
		}
		const timeStart = new Date().getTime()
		workspaceReencryptionInProgress = true
		await WorkspaceService.setWorkspaceEncryptionKey({
			workspace: $workspaceStore!,
			requestBody: {
				new_key: editedWorkspaceEncryptionKey ?? '' // cannot be undefined at this point
			}
		})
		await loadWorkspaceEncryptionKey()
		const timeEnd = new Date().getTime()
		sendUserToast('All workspace secrets have been re-encrypted with the new key')
		setTimeout(
			() => {
				workspaceReencryptionInProgress = false
			},
			1000 - (timeEnd - timeStart)
		)
	}

	async function loadSettings(): Promise<void> {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		slack_team_name = settings.slack_name
		teams_team_id = settings.teams_team_id
		teams_team_name = settings.teams_team_name
		if (settings.slack_command_script) {
			itemKind = settings.slack_command_script.split('/')[0] as 'flow' | 'script'
		}
		if (settings.teams_command_script) {
			itemKind = settings.teams_command_script.split('/')[0] as 'flow' | 'script'
		}
		slackScriptPath = (settings.slack_command_script ?? '').split('/').slice(1).join('/')
		teamsScriptPath = (settings.teams_command_script ?? '').split('/').slice(1).join('/')
		slackInitialPath = slackScriptPath
		teamsInitialPath = teamsScriptPath
		plan = settings.plan
		customer_id = settings.customer_id
		workspaceToDeployTo = settings.deploy_to
		webhook = settings.webhook

		aiProviders = settings.ai_config?.providers ?? {}
		defaultModel = settings.ai_config?.default_model?.model
		codeCompletionModel = settings.ai_config?.code_completion_model?.model
		customPrompts = settings.ai_config?.custom_prompts ?? {}
		maxTokensPerModel = settings.ai_config?.max_tokens_per_model ?? {}
		for (const mode of Object.values(AIMode)) {
			if (!(mode in customPrompts)) {
				customPrompts[mode] = ''
			}
		}

		// Store initial AI config state for unsaved changes detection
		initialAiProviders = clone(aiProviders)
		initialDefaultModel = defaultModel
		initialCodeCompletionModel = codeCompletionModel
		initialCustomPrompts = clone(customPrompts)
		initialMaxTokensPerModel = clone(maxTokensPerModel)
		errorHandlerItemKind = settings.error_handler
			? (settings.error_handler.split('/')[0] as 'flow' | 'script')
			: 'script'
		errorHandlerScriptPath = (settings.error_handler ?? '').split('/').slice(1).join('/')
		errorHandlerMutedOnCancel = settings.error_handler_muted_on_cancel
		criticalAlertUIMuted = settings.mute_critical_alerts
		initialCriticalAlertUIMuted = settings.mute_critical_alerts
		if (emptyString($enterpriseLicense)) {
			errorHandlerSelected = 'custom'
		} else {
			errorHandlerSelected = getHandlerType(errorHandlerScriptPath)
		}
		errorHandlerExtraArgs = settings.error_handler_extra_args ?? {}
		workspaceDefaultAppPath = settings.default_app

		s3ResourceSettings = convertBackendSettingsToFrontendSettings(
			settings.large_file_storage,
			!!$enterpriseLicense
		)
		initialS3ResourceSettings = clone(s3ResourceSettings)
		dataTableSettings = convertDataTableSettingsFromBackend(settings.datatable)
		ducklakeSettings = convertDucklakeSettingsFromBackend(settings.ducklake)
		ducklakeSavedSettings = clone(ducklakeSettings)

		if (settings.deploy_ui != undefined && settings.deploy_ui != null) {
			deployUiSettings = {
				include_path:
					(settings.deploy_ui.include_path?.length ?? 0 > 0)
						? (settings.deploy_ui.include_path ?? [])
						: [],
				include_type: {
					scripts: (settings.deploy_ui.include_type?.indexOf('script') ?? -1) >= 0,
					flows: (settings.deploy_ui.include_type?.indexOf('flow') ?? -1) >= 0,
					apps: (settings.deploy_ui.include_type?.indexOf('app') ?? -1) >= 0,
					resources: (settings.deploy_ui.include_type?.indexOf('resource') ?? -1) >= 0,
					variables: (settings.deploy_ui.include_type?.indexOf('variable') ?? -1) >= 0,
					secrets: (settings.deploy_ui.include_type?.indexOf('secret') ?? -1) >= 0,
					triggers: (settings.deploy_ui.include_type?.indexOf('trigger') ?? -1) >= 0
				}
			}
		}

		// check openai_client_credentials_oauth
		usingOpenaiClientCredentialsOauth = await ResourceService.existsResourceType({
			workspace: $workspaceStore!,
			path: 'openai_client_credentials_oauth'
		})

		loadedSettings = true
	}

	async function loadSlackOAuthConfig(): Promise<void> {
		if (!$workspaceStore) return

		try {
			const config = await WorkspaceService.getWorkspaceSlackOauthConfig({
				workspace: $workspaceStore
			})
			useCustomSlackApp = !!config.slack_oauth_client_id
			slackOAuthClientId = config.slack_oauth_client_id || ''
			slackOAuthClientSecret = config.slack_oauth_client_secret || ''
			slackOAuthConfigLoaded = !!config.slack_oauth_client_id
		} catch (e) {
			console.error('Failed to load Slack OAuth config:', e)
		}
	}

	async function saveAndConnectSlack(): Promise<void> {
		if (!$workspaceStore) return

		if (!slackOAuthClientId || !slackOAuthClientSecret) {
			sendUserToast('Both client ID and client secret are required', true)
			return
		}

		try {
			// Disconnect existing Slack connection (if any) before saving workspace-specific config
			if (slack_team_name) {
				await OauthService.disconnectSlack({ workspace: $workspaceStore })
			}

			await WorkspaceService.setWorkspaceSlackOauthConfig({
				workspace: $workspaceStore,
				requestBody: {
					slack_oauth_client_id: slackOAuthClientId,
					slack_oauth_client_secret: slackOAuthClientSecret
				}
			})

			// Redirect to OAuth flow
			window.location.href = `${base}/api/oauth/connect_slack?workspace=${$workspaceStore}`
		} catch (e) {
			sendUserToast('Failed to save Slack OAuth configuration', true)
			console.error(e)
		}
	}

	async function deleteSlackOAuthConfig(): Promise<void> {
		if (!$workspaceStore) return

		try {
			// Delete workspace OAuth config
			await WorkspaceService.deleteWorkspaceSlackOauthConfig({ workspace: $workspaceStore })

			// Also disconnect any existing Slack connection
			if (slack_team_name) {
				await OauthService.disconnectSlack({ workspace: $workspaceStore })
			}

			useCustomSlackApp = false
			slackOAuthClientId = ''
			slackOAuthClientSecret = ''
			slackOAuthConfigLoaded = false
			await loadSettings()
			sendUserToast('Workspace Slack app deleted')
		} catch (e) {
			sendUserToast('Failed to delete Slack OAuth configuration', true)
			console.error(e)
		}
	}

	let deployUiSettings:
		| {
				include_path: string[]
				include_type: {
					scripts: boolean
					flows: boolean
					apps: boolean
					resources: boolean
					variables: boolean
					secrets: boolean
					triggers: boolean
				}
		  }
		| undefined = $state()

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				loadSettings()
				loadSlackOAuthConfig()
			})
		}
	})

	async function editErrorHandler() {
		if (errorHandlerScriptPath) {
			await WorkspaceService.editErrorHandler({
				workspace: $workspaceStore!,
				requestBody: {
					error_handler: `${errorHandlerItemKind}/${errorHandlerScriptPath}`,
					error_handler_extra_args: errorHandlerExtraArgs,
					error_handler_muted_on_cancel: errorHandlerMutedOnCancel
				}
			})
			sendUserToast(`workspace error handler set to ${errorHandlerScriptPath}`)
		} else {
			await WorkspaceService.editErrorHandler({
				workspace: $workspaceStore!,
				requestBody: {
					error_handler: undefined,
					error_handler_extra_args: undefined,
					error_handler_muted_on_cancel: undefined
				}
			})
			sendUserToast(`workspace error handler removed`)
		}
	}

	async function editCriticalAlertMuteSetting() {
		await SettingService.workspaceMuteCriticalAlertsUi({
			workspace: $workspaceStore!,
			requestBody: {
				mute_critical_alerts: criticalAlertUIMuted
			}
		})
		sendUserToast(
			`Critical alert UI mute setting for workspace is set to ${criticalAlertUIMuted}\nreloading page...`
		)
		// reload page after change of setting
		setTimeout(() => {
			window.location.reload()
		}, 3000)
	}

	function updateFromSearchTab(searchTab: string | null, currentTab: string) {
		if (searchTab && searchTab !== currentTab) {
			tab = searchTab as typeof tab
		}
	}

	$effect(() => {
		updateFromSearchTab(
			$page.url.searchParams.get('tab'),
			untrack(() => tab)
		)
	})

	// Function to check if there are unsaved changes in AI settings
	function getAiSettingsInitialAndModifiedValues() {
		// Only check for unsaved changes when on the AI tab
		if (tab !== 'ai') {
			return {
				savedValue: undefined,
				modifiedValue: undefined
			}
		}

		const savedValue = {
			aiProviders: initialAiProviders,
			defaultModel: initialDefaultModel,
			codeCompletionModel: initialCodeCompletionModel,
			customPrompts: initialCustomPrompts,
			maxTokensPerModel: initialMaxTokensPerModel
		}

		const modifiedValue = {
			aiProviders: aiProviders,
			defaultModel: defaultModel,
			codeCompletionModel: codeCompletionModel,
			customPrompts: customPrompts,
			maxTokensPerModel: maxTokensPerModel
		}

		return { savedValue, modifiedValue }
	}

	// Function to discard unsaved AI settings changes
	function discardAiSettingsChanges() {
		aiProviders = clone(initialAiProviders)
		defaultModel = initialDefaultModel
		codeCompletionModel = initialCodeCompletionModel
		customPrompts = clone(initialCustomPrompts)
		maxTokensPerModel = clone(initialMaxTokensPerModel)
	}

	// Function to check if there are unsaved changes in storage settings
	function getStorageSettingsInitialAndModifiedValues() {
		// Only check for unsaved changes when on the windmill_lfs tab
		if (tab !== 'windmill_lfs') {
			return {
				savedValue: undefined,
				modifiedValue: undefined
			}
		}

		const savedValue = {
			s3ResourceSettings: initialS3ResourceSettings,
			ducklakeSettings: ducklakeSavedSettings
		}

		const modifiedValue = {
			s3ResourceSettings: s3ResourceSettings,
			ducklakeSettings: ducklakeSettings
		}

		return { savedValue, modifiedValue }
	}

	// Function to discard unsaved storage settings changes
	function discardStorageSettingsChanges() {
		s3ResourceSettings = clone(initialS3ResourceSettings)
		ducklakeSettings = clone(ducklakeSavedSettings)
	}

	// Combined function to check for unsaved changes across all tabs
	function getAllUnsavedChanges() {
		if (dataTableSettingsComponent) {
			return dataTableSettingsComponent.unsavedChanges()
		}

		// Check AI settings
		const aiChanges = getAiSettingsInitialAndModifiedValues()
		if (aiChanges.savedValue && aiChanges.modifiedValue) {
			return aiChanges
		}

		// Check storage settings
		const storageChanges = getStorageSettingsInitialAndModifiedValues()
		if (storageChanges.savedValue && storageChanges.modifiedValue) {
			return storageChanges
		}

		return {
			savedValue: {},
			modifiedValue: {}
		}
	}

	// Combined function to discard changes based on current tab
	function discardAllChanges() {
		if (tab === 'ai') {
			discardAiSettingsChanges()
		} else if (tab === 'windmill_lfs') {
			discardStorageSettingsChanges()
		}
	}
</script>

<CenteredPage>
	{#if $userStore?.is_admin || $superadmin}
		<PageHeader title="Workspace settings: {$workspaceStore}"
			>{#if $superadmin}
				<Button variant="default" size="sm" on:click={() => goto('#superadmin-settings')}>
					Instance settings
				</Button>
			{/if}</PageHeader
		>

		<div class="overflow-x-auto scrollbar-hidden">
			<Tabs
				bind:selected={tab}
				deferSelectedUpdate={true}
				on:selected={(e) => {
					// setQueryWithoutLoad($page.url, [{ key: 'tab', value: tab }], 0)
					const params = new URLSearchParams($page.url.searchParams)
					const newTab = e.detail
					params.set('tab', newTab)
					goto(`?${params.toString()}`)
				}}
			>
				<Tab
					small
					value="users"
					aiId="workspace-settings-users"
					aiDescription="Users workspace settings"
					label="Users"
				/>
				<Tab
					small
					value="git_sync"
					aiId="workspace-settings-git-sync"
					aiDescription="Git sync workspace settings"
					label="Git Sync"
				/>
				<Tab
					small
					value="deploy_to"
					aiId="workspace-settings-deploy-to"
					aiDescription="Deployment UI workspace settings"
					label="Deployment UI"
				/>

				{#if WORKSPACE_SHOW_SLACK_CMD}
					<Tab
						small
						value="slack"
						aiId="workspace-settings-slack"
						aiDescription="Slack / Teams workspace settings"
						label="Slack / Teams"
					/>
				{/if}
				{#if isCloudHosted()}
					<Tab
						small
						value="premium"
						aiId="workspace-settings-premium"
						aiDescription="Premium plans workspace settings"
						label="Premium Plans"
					/>
				{/if}
				{#if WORKSPACE_SHOW_WEBHOOK_CLI_SYNC}
					<Tab
						small
						value="webhook"
						aiId="workspace-settings-webhook"
						aiDescription="Webhook workspace settings"
						label="Webhook"
					/>
				{/if}
				<Tab
					small
					value="error_handler"
					aiId="workspace-settings-error-handler"
					aiDescription="Error handler workspace settings"
					label="Error Handler"
				/>
				<Tab
					small
					value="ai"
					aiId="workspace-settings-ai"
					aiDescription="Windmill AI workspace settings"
					label="Windmill AI"
				/>
				<Tab
					small
					value="windmill_data_tables"
					aiId="workspace-settings-windmill-data-tables"
					aiDescription="Data tables workspace settings"
					label="Data Tables"
				/>
				<Tab
					small
					value="windmill_lfs"
					aiId="workspace-settings-windmill-lfs"
					aiDescription="Object Storage (S3) workspace settings"
					label="Object Storage (S3)"
				/>
				<Tab
					small
					value="default_app"
					aiId="workspace-settings-default-app"
					aiDescription="Default app workspace settings"
					label="Default App"
				/>

				<Tab
					small
					value="encryption"
					aiId="workspace-settings-encryption"
					aiDescription="Encryption workspace settings"
					label="Encryption"
				/>

				<Tab
					small
					value="general"
					aiId="workspace-settings-general"
					aiDescription="General workspace settings"
					label="General"
				/>
				<Tab
					small
					value="dependencies"
					aiId="workspace-settings-dependencies"
					aiDescription="Workspace dependencies settings"
					label="Dependencies"
				/>
			</Tabs>
		</div>
		{#if !loadedSettings}
			<Skeleton layout={[1, [40]]} />
		{:else if tab == 'users'}
			<WorkspaceUserSettings />
		{:else if tab == 'deploy_to'}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-sm font-semibold text-emphasis">
						Link this Workspace to another Staging / Prod Workspace
					</div>
					<Description link="https://www.windmill.dev/docs/core_concepts/staging_prod">
						Connecting this workspace with another staging/production workspace enables web-based
						deployment to that workspace.
					</Description>
				</div>
			</div>
			{#if $enterpriseLicense}
				<DeployToSetting bind:workspaceToDeployTo bind:deployUiSettings />
			{:else}
				<div class="my-2"
					><Alert type="warning" title="Enterprise license required"
						>Deploy to staging/prod from the web UI is only available with an enterprise license</Alert
					></div
				>
			{/if}
		{:else if tab == 'premium'}
			<PremiumInfo {customer_id} {plan} />
		{:else if tab == 'slack'}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-sm font-semibold text-emphasis"
						>Workspace connections to Slack and Teams</div
					>
					<Description link="https://www.windmill.dev/docs/integrations/slack">
						With workspace connections, you can trigger scripts or flows with a '/windmill' command
						with your Slack or Teams bot.
					</Description>
				</div>

				<Tabs bind:selected={slack_tabs}>
					<Tab value="slack_commands" label="Slack" />
					<Tab value="teams_commands" label="Teams" />
				</Tabs>

				{#if slack_tabs === 'slack_commands'}
					<ConnectionSection
						platform="slack"
						teamName={slack_team_name}
						bind:scriptPath={slackScriptPath}
						bind:initialPath={slackInitialPath}
						bind:itemKind
						onDisconnect={async () => {
							await OauthService.disconnectSlack({ workspace: $workspaceStore ?? '' })
							loadSettings()
							sendUserToast('Disconnected Slack')
						}}
						onSelect={editSlackCommand}
						connectHref="{base}/api/oauth/connect_slack"
						createScriptHref="{base}/scripts/add?hub=hub%2F28071%2Fslack%2Fexample_of_responding_to_a_slack_command_slack"
						createFlowHref="{base}/flows/add?hub=28"
						documentationLink="https://www.windmill.dev/docs/integrations/slack"
						onLoadSettings={loadSettings}
						display_name={slack_team_name}
						hideConnectButton={useCustomSlackApp && !slackOAuthConfigLoaded}
					>
						{#snippet workspaceConfig()}
							<!-- Workspace OAuth Configuration Section -->
							<div class="flex flex-col">
								{#if slackOAuthConfigLoaded}
									<!-- Show saved config with delete button -->
									<div class="flex flex-col gap-1 w-fit">
										<div class="text-sm text-primary font-medium">Workspace specific Slack app</div>
										<div
											class="p-2 rounded-md border border-gray-200 dark:border-gray-700 bg-surface-secondary"
										>
											<div class="flex items-center gap-3">
												<div class="flex items-center gap-2">
													<span class="text-sm text-primary">Client ID:</span>
													<span class="text-xs text-secondary font-mono pt-1"
														>{slackOAuthClientId}</span
													>
												</div>
												<Button size="xs" onclick={deleteSlackOAuthConfig} btnClasses="w-fit">
													<Trash2 size={14} class="mr-1" />
													Delete
												</Button>
											</div>
										</div>
									</div>
								{:else}
									<!-- Show toggle and form to create config -->
									<label class="text-sm flex gap-2 items-center font-medium text-primary">
										<Toggle bind:checked={useCustomSlackApp} size="sm" />
										<span class="text-xs text-secondary">Use workspace specific Slack app</span>
									</label>

									{#if useCustomSlackApp}
										<div class="p-2 rounded border border-gray-200 dark:border-gray-700">
											<label class="block pb-2">
												<span class="text-primary font-semibold text-sm">Client ID</span>
												<input
													class="windmill-input"
													type="text"
													placeholder="1234567890.1234567890"
													bind:value={slackOAuthClientId}
												/>
											</label>

											<label class="block pb-2">
												<span class="text-primary font-semibold text-sm">Client secret</span>
												<input
													class="windmill-input"
													type="password"
													placeholder="Enter client secret"
													bind:value={slackOAuthClientSecret}
												/>
											</label>

											<CollapseLink text="Instructions">
												<div class="text-xs text-secondary p-2">
													Create a Slack app at{' '}
													<a
														href="https://api.slack.com/apps"
														target="_blank"
														rel="noopener noreferrer"
														class="text-blue-600 dark:text-blue-400 hover:underline"
													>
														Slack API
													</a>. Set the redirect URI to:{' '}
													<code class="bg-gray-100 dark:bg-gray-800 px-1 py-0.5 rounded">
														{window.location.origin}{base}/oauth/callback_slack
													</code>
												</div>
											</CollapseLink>

											<div class="pt-2">
												<Button
													size="xs"
													variant="accent"
													onclick={saveAndConnectSlack}
													disabled={!slackOAuthClientId || !slackOAuthClientSecret}
													startIcon={{ icon: Slack }}
													btnClasses="w-fit"
												>
													Connect to Slack
												</Button>
											</div>
										</div>
									{/if}
								{/if}
							</div>
						{/snippet}
					</ConnectionSection>
				{:else if slack_tabs === 'teams_commands'}
					{#if !$enterpriseLicense}
						<div class="pt-4"></div>
						<Alert type="warning" title="Workspace Teams commands is an EE feature">
							Workspace Teams commands is a Windmill EE feature. It enables using your current Slack
							/ Teams connection to run a custom script and send notifications.
						</Alert>
						<div class="pb-2"></div>
					{/if}
					<ConnectionSection
						platform="teams"
						teamName={teams_team_id}
						bind:scriptPath={teamsScriptPath}
						bind:initialPath={teamsInitialPath}
						bind:itemKind
						onDisconnect={async () => {
							await OauthService.disconnectTeams({ workspace: $workspaceStore ?? '' })
							loadSettings()
							sendUserToast('Disconnected Teams')
						}}
						onSelect={editTeamsCommand}
						connectHref={undefined}
						createScriptHref="{base}/scripts/add?hub=hub%2F11591%2Fteams%2FExample%20of%20responding%20to%20a%20Microsoft%20Teams%20command"
						createFlowHref="{base}/flows/add?hub=58"
						documentationLink="https://www.windmill.dev/docs/integrations/teams"
						onLoadSettings={loadSettings}
						display_name={teams_team_name}
					/>
				{/if}
			</div>
		{:else if tab == 'general'}
			<div class="flex flex-col gap-4 my-6">
				<div class="flex flex-col gap-1">
					<div class="text-sm font-semibold text-emphasis">General</div>
					<Description link="https://www.windmill.dev/docs/core_concepts/workspace_settings">
						Configure general workspace settings.
					</Description>
				</div>
			</div>

			<div class="flex flex-col gap-6">
				<ChangeWorkspaceName />
				<ChangeWorkspaceId />
				<ChangeWorkspaceColor />
			</div>

			<div class="text-xs font-semibold text-emphasis mt-6 mb-1">Export workspace</div>
			<div class="flex justify-start">
				<Button
					size="sm"
					href="{base}/api/w/{$workspaceStore ?? ''}/workspaces/tarball?archive_type=zip"
					target="_blank"
				>
					Export workspace as zip file
				</Button>
			</div>

			<div class="mt-12"></div>
			<span class="text-sm font-semibold text-emphasis">Delete workspace</span>
			{#if !$superadmin}
				<p class="text-2xs text-secondary"> Only instance superadmins can delete a workspace. </p>
			{/if}
			{#if $workspaceStore === 'admins' || $workspaceStore === 'starter'}
				<p class="text-2xs text-secondary">
					This workspace cannot be deleted as it has a special function. Consult the documentation
					for more information.
				</p>
			{/if}
			<div class="flex gap-2">
				<Button
					destructive
					disabled={$workspaceStore === 'admins' || $workspaceStore === 'starter'}
					unifiedSize="md"
					btnClasses="mt-2"
					on:click={async () => {
						await WorkspaceService.archiveWorkspace({ workspace: $workspaceStore ?? '' })
						sendUserToast(`Archived workspace ${$workspaceStore}`)
						workspaceStore.set(undefined)
						usersWorkspaceStore.set(undefined)
						goto('/user/workspaces')
					}}
				>
					Archive workspace
				</Button>

				{#if $superadmin}
					<Button
						color="red"
						disabled={$workspaceStore === 'admins' || $workspaceStore === 'starter'}
						size="sm"
						btnClasses="mt-2"
						on:click={async () => {
							await WorkspaceService.deleteWorkspace({ workspace: $workspaceStore ?? '' })
							sendUserToast(`Deleted workspace ${$workspaceStore}`)
							workspaceStore.set(undefined)
							usersWorkspaceStore.set(undefined)
							goto('/user/workspaces')
						}}
					>
						Delete workspace (superadmin)
					</Button>
				{/if}
			</div>
		{:else if tab == 'webhook'}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-xs font-semibold text-emphasis"> Workspace Webhook</div>
					<Description
						link="https://www.windmill.dev/docs/core_concepts/webhooks#workspace-webhook"
					>
						Connect your Windmill workspace to an external service to sync or get notified about any
						change.
					</Description>
				</div>
			</div>
			<div class="flex flex-col gap-4 my-4">
				<div class="flex flex-col gap-1">
					<div class="text-xs font-semibold text-emphasis"> URL to send requests to</div>
					<div class="text-primary text-xs">
						This URL will be POSTed to with a JSON body depending on the type of event. The type is
						indicated by the type field. The other fields are dependent on the type.
					</div>
				</div>
			</div>
			<div class="flex gap-2">
				<input class="justify-start" type="text" bind:value={webhook} />
				<Button color="blue" btnClasses="justify-end" on:click={editWebhook}>Set webhook</Button>
			</div>
		{:else if tab == 'error_handler'}
			{#if !$enterpriseLicense}
				<div class="pt-4"></div>
				<Alert type="warning" title="Workspace error handler is an EE feature">
					Workspace error handler is a Windmill EE feature. It enables using your current Slack
					connection or a custom script to send notifications anytime any job would fail.
				</Alert>
				<div class="pb-2"></div>
			{/if}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-sm font-semibold text-emphasis"> Workspace Error Handler</div>
					<Description
						link="https://www.windmill.dev/docs/core_concepts/error_handling#workspace-error-handler"
					>
						Define a script or flow to be executed automatically in case of error in the workspace.
					</Description>
				</div>
			</div>
			<div class="flex flex-col gap-4 my-4">
				<div class="flex flex-col gap-1">
					<div class="text-xs font-semibold text-emphasis">
						Script or flow to run as error handler</div
					>
				</div>
			</div>
			<ErrorOrRecoveryHandler
				isEditable={true}
				errorOrRecovery="error"
				showScriptHelpText={true}
				bind:handlerSelected={errorHandlerSelected}
				bind:handlerPath={errorHandlerScriptPath}
				customScriptTemplate="/scripts/add?hub=hub%2F9083%2Fwindmill%2Fworkspace_error_handler_template"
				bind:customHandlerKind={errorHandlerItemKind}
				bind:handlerExtraArgs={errorHandlerExtraArgs}
			>
				{#snippet customTabTooltip()}
					<Tooltip>
						<div class="flex gap-20 items-start mt-3">
							<div class="text-sm">
								The following args will be passed to the error handler:
								<ul class="mt-1 ml-2">
									<li><b>path</b>: The path of the script or flow that errored.</li>
									<li>
										<b>email</b>: The email of the user who ran the script or flow that errored.
									</li>
									<li><b>error</b>: The error details.</li>
									<li><b>job_id</b>: The job id.</li>
									<li><b>is_flow</b>: Whether the error comes from a flow.</li>
									<li><b>workspace_id</b>: The workspace id of the failed script or flow.</li>
								</ul>
								<br />
								The error handler will be executed by the automatically created group g/error_handler.
								If your error handler requires variables or resources, you need to add them to the group.
							</div>
						</div>
					</Tooltip>
				{/snippet}
			</ErrorOrRecoveryHandler>

			<div class="flex flex-col mt-5 gap-5 items-start">
				<Toggle
					disabled={!$enterpriseLicense ||
						((errorHandlerSelected === 'slack' || errorHandlerSelected === 'teams') &&
							!emptyString(errorHandlerScriptPath) &&
							emptyString(errorHandlerExtraArgs['channel']))}
					bind:checked={errorHandlerMutedOnCancel}
					options={{ right: 'Do not run error handler for canceled jobs' }}
				/>
				<Button
					disabled={!$enterpriseLicense ||
						((errorHandlerSelected === 'slack' || errorHandlerSelected === 'teams') &&
							!emptyString(errorHandlerScriptPath) &&
							emptyString(errorHandlerExtraArgs['channel']))}
					size="sm"
					on:click={editErrorHandler}
				>
					Save
				</Button>
			</div>
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-sm font-semibold text-emphasis"> Workspace Critical Alerts</div>
					<Description link="https://www.windmill.dev/docs/core_concepts/critical_alerts">
						Critical alerts within the scope of a workspace are sent to the workspace admins through
						a UI notification.
					</Description>
					<div class="flex flex-col mt-5 gap-5 items-start">
						<Button
							disabled={!$enterpriseLicense}
							size="sm"
							on:click={() => isCriticalAlertsUIOpen.set(true)}
						>
							Show critical alerts
						</Button>
						<Toggle
							disabled={!$enterpriseLicense}
							bind:checked={criticalAlertUIMuted}
							options={{ right: 'Mute critical alerts UI for this workspace' }}
						/>
						<Button
							disabled={!$enterpriseLicense || criticalAlertUIMuted == initialCriticalAlertUIMuted}
							size="sm"
							on:click={editCriticalAlertMuteSetting}
						>
							Save mute setting
						</Button>
					</div>
				</div>
			</div>
		{:else if tab == 'ai'}
			<AISettings
				bind:aiProviders
				bind:codeCompletionModel
				bind:defaultModel
				bind:customPrompts
				bind:maxTokensPerModel
				bind:usingOpenaiClientCredentialsOauth
				onSave={() => {
					// Update initial state after successful save
					initialAiProviders = clone(aiProviders)
					initialDefaultModel = defaultModel
					initialCodeCompletionModel = codeCompletionModel
					initialCustomPrompts = clone(customPrompts)
					initialMaxTokensPerModel = clone(maxTokensPerModel)
				}}
			/>
		{:else if tab == 'windmill_data_tables'}
			<DataTableSettings bind:dataTableSettings bind:this={dataTableSettingsComponent} />
		{:else if tab == 'windmill_lfs'}
			<StorageSettings
				bind:s3ResourceSettings
				onSave={() => {
					initialS3ResourceSettings = clone(s3ResourceSettings)
				}}
			/>
			<DucklakeSettings
				bind:ducklakeSettings
				bind:ducklakeSavedSettings
				onSave={() => {
					ducklakeSavedSettings = clone(ducklakeSettings)
				}}
			/>
		{:else if tab == 'git_sync'}
			{#if $workspaceStore}
				<GitSyncSection />
			{:else}
				<div class="flex items-center justify-center p-8">
					<div class="text-sm text-secondary">Loading workspace...</div>
				</div>
			{/if}
		{:else if tab == 'dependencies'}
			<WorkspaceDependenciesSettings />
		{:else if tab == 'default_app'}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-sm font-semibold text-emphasis">Workspace Default App</div>
					<Description>
						If configured, users who are operators in this workspace will be redirected to this app
						automatically when logging into this workspace.
					</Description>
					<Description link="https://www.windmill.dev/docs/apps/default_app">
						Make sure the default app is shared with all the operators of this workspace before
						turning this feature on.
					</Description>
				</div>
			</div>
			{#if !$enterpriseLicense}
				<Alert type="warning" title="Windmill EE only feature">
					Default app can only be set on Windmill Enterprise Edition.
				</Alert>
			{/if}
			<Alert type="info" title="Default app must be accessible to all operators">
				Make sure the default app is shared with all the operators of this workspace before turning
				this feature on.
			</Alert>
			<div class="mt-5 flex gap-1">
				{#key workspaceDefaultAppPath}
					<ScriptPicker
						initialPath={workspaceDefaultAppPath}
						itemKind="app"
						on:select={(ev) => {
							editWorkspaceDefaultApp(ev?.detail?.path)
						}}
					/>
				{/key}
			</div>
		{:else if tab == 'encryption'}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-sm font-semibold text-emphasis">Workspace Secret Encryption</div>
					<Description>
						When updating the encryption key of a workspace, all secrets will be re-encrypted with
						the new key and the previous key will be replaced by the new one.
					</Description>
					<Description
						link="https://www.windmill.dev/docs/core_concepts/workspace_secret_encryption"
					>
						If you're manually updating the key to match another workspace key from another Windmill
						instance, make sure not to use the 'SECRET_SALT' environment variable or, if you're
						using it, make sure it the salt matches across both instances.
					</Description>
				</div>
			</div>
			<div class="mt-5 flex gap-1 mb-10">
				<Button
					color="blue"
					disabled={editedWorkspaceEncryptionKey === workspaceEncryptionKey ||
						!encryptionKeyRegex.test(editedWorkspaceEncryptionKey ?? '')}
					startIcon={{
						icon: workspaceReencryptionInProgress ? RotateCw : Save,
						classes: workspaceReencryptionInProgress ? 'animate-spin' : ''
					}}
					on:click={() => {
						setWorkspaceEncryptionKey()
					}}>Save & Re-encrypt workspace</Button
				>
			</div>
			<label for="workspace-encryption-key" class="text-xs font-semibold text-emphasis mt-1">
				Workspace encryption key
			</label>
			<div class="flex gap-2 mt-1">
				<TextInput
					inputProps={{
						id: 'workspace-encryption-key',
						placeholder: '*'.repeat(64)
					}}
					bind:value={editedWorkspaceEncryptionKey}
				/>
				<Button
					variant="default"
					unifiedSize="md"
					on:click={() => {
						loadWorkspaceEncryptionKey()
					}}>Load current key</Button
				>
			</div>
			{#if !emptyString(editedWorkspaceEncryptionKey) && !encryptionKeyRegex.test(editedWorkspaceEncryptionKey ?? '')}
				<div class="text-xs text-red-600">
					Key invalid - it should be 64 characters long and only contain letters and numbers.
				</div>
			{/if}
		{/if}
	{:else}
		<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4" role="alert">
			<p class="font-bold">Not an admin</p>
			<p>Workspace settings are only available for admin of workspaces</p>
		</div>
	{/if}
</CenteredPage>

<UnsavedConfirmationModal
	getInitialAndModifiedValues={getAllUnsavedChanges}
	onDiscardChanges={discardAllChanges}
	triggerOnSearchParamsChange={true}
	tabMode={true}
/>

<style>
</style>
