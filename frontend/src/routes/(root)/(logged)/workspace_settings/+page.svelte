<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import { isCloudHosted } from '$lib/cloud'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Button, Section, Skeleton, Tab, Tabs } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	import DeployToSetting from '$lib/components/DeployToSetting.svelte'
	import ErrorOrRecoveryHandler from '$lib/components/ErrorOrRecoveryHandler.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import WorkspaceUserSettings from '$lib/components/settings/WorkspaceUserSettings.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
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
	import { clone, emptyString, encodeState, hasUnsavedChanges } from '$lib/utils'
	import { Slack } from 'lucide-svelte'
	import SidebarNavigation from '$lib/components/common/sidebar/SidebarNavigation.svelte'

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
	import { validateWebhookUrl, validateEncryptionKey } from '$lib/validators/workspaceSettings'
	import DataTableSettings, {
		convertDataTableSettingsFromBackend,
		type DataTableSettingsType
	} from '$lib/components/workspaceSettings/DataTableSettings.svelte'
	import WorkspaceDependenciesSettings from '$lib/components/workspaceSettings/WorkspaceDependenciesSettings.svelte'
	import SettingsFooter from '$lib/components/workspaceSettings/SettingsFooter.svelte'
	import WorkspaceRulesets from '$lib/components/workspaceSettings/WorkspaceRulesets.svelte'
	import SettingCard from '$lib/components/instanceSettings/SettingCard.svelte'

	let slackInitialPath: string = $state('')
	let slackScriptPath: string = $state('')
	let teamsInitialPath: string = $state('')
	let teamsScriptPath: string = $state('')
	let slack_team_name: string | undefined = $state()
	let teams_team_id: string | undefined = $state()
	let teams_team_name: string | undefined = $state()
	let useCustomSlackApp: boolean = $state(false)
	let slackAppType: 'instance' | 'workspace' = $state('instance')

	// Keep slackAppType and useCustomSlackApp in sync
	$effect(() => {
		if (slackAppType === 'workspace') {
			useCustomSlackApp = true
		} else {
			useCustomSlackApp = false
		}
	})
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
	let errorHandlerMutedOnUserPath: boolean | undefined = $state(undefined)
	let successHandlerScriptPath: string | undefined = $state(undefined)
	let criticalAlertUIMuted: boolean | undefined = $state(undefined)
	let initialCriticalAlertUIMuted: boolean | undefined = $state(undefined)
	let publicAppRateLimitPerMinute: number | undefined = $state(undefined)
	let initialPublicAppRateLimitPerMinute: number | undefined = $state(undefined)

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

	// Track initial deploy settings for unsaved changes detection
	let initialWorkspaceToDeployTo: string | undefined = $state(undefined)
	let initialDeployUiSettings: {
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
	} = $state({
		include_path: [],
		include_type: {
			scripts: true,
			flows: true,
			apps: true,
			resources: true,
			variables: true,
			secrets: true,
			triggers: true
		}
	})

	// Track initial webhook for unsaved changes detection
	let initialWebhook: string | undefined = $state(undefined)

	// Track initial encryption key for unsaved changes detection
	let initialEditedWorkspaceEncryptionKey: string | undefined = $state(undefined)

	// Track initial error handler settings for unsaved changes detection
	let initialErrorHandlerSelected: ErrorHandler = $state('slack')
	let initialErrorHandlerScriptPath: string | undefined = $state(undefined)
	let initialErrorHandlerItemKind: 'flow' | 'script' = $state('script')
	let initialErrorHandlerExtraArgs: Record<string, any> = $state({})
	let initialErrorHandlerMutedOnCancel: boolean | undefined = $state(undefined)
	let initialErrorHandlerMutedOnUserPath: boolean | undefined = $state(undefined)

	// Track initial success handler for unsaved changes detection
	let initialSuccessHandlerScriptPath: string | undefined = $state(undefined)

	let s3ResourceSettings: S3ResourceSettings = $state({
		resourceType: 's3',
		resourcePath: undefined,
		publicResource: undefined,
		secondaryStorage: undefined
	})
	let s3ResourceSavedSettings: S3ResourceSettings = $state({
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
	let initialWorkspaceDefaultAppPath: string | undefined = $state(undefined)
	let workspaceEncryptionKey: string | undefined = $state(undefined)
	let editedWorkspaceEncryptionKey: string | undefined = $state(undefined)
	let workspaceReencryptionInProgress: boolean = $state(false)

	// Validation state
	let webhookValidationError: string | undefined = $state(undefined)
	let encryptionKeyValidationError: string | undefined = $state(undefined)

	// Derived state for checking unsaved changes in error handler
	let hasErrorHandlerChanges = $derived.by(() => {
		if (tab !== 'error_handler') return false
		const changes = getErrorHandlerSettingsInitialAndModifiedValues()
		if (!changes.savedValue || !changes.modifiedValue) return false
		return hasUnsavedChanges(changes.savedValue, changes.modifiedValue)
	})

	// Derived state for checking unsaved changes in success handler
	let hasSuccessHandlerChanges = $derived.by(() => {
		if (tab !== 'error_handler') return false
		return hasUnsavedChanges(
			{ successHandlerScriptPath: initialSuccessHandlerScriptPath },
			{ successHandlerScriptPath: successHandlerScriptPath }
		)
	})

	// Derived state for checking unsaved changes in critical alert mute setting
	let hasCriticalAlertMuteChanges = $derived.by(() => {
		if (tab !== 'critical_alerts') return false

		// Normalize undefined to false for comparison
		const currentValue = criticalAlertUIMuted ?? false
		const initialValue = initialCriticalAlertUIMuted ?? false

		return currentValue !== initialValue
	})

	// Derived state for checking unsaved changes in AI settings
	let hasAiSettingsChanges = $derived.by(() => {
		if (tab !== 'ai') return false
		const changes = getAiSettingsInitialAndModifiedValues()
		if (!changes.savedValue || !changes.modifiedValue) return false
		return hasUnsavedChanges(changes.savedValue, changes.modifiedValue)
	})

	// Derived state for checking unsaved changes in deployment settings
	let hasDeploySettingsChanges = $derived.by(() => {
		if (tab !== 'deploy_to') return false
		const changes = getDeploySettingsInitialAndModifiedValues()
		if (!changes.savedValue || !changes.modifiedValue) return false
		return hasUnsavedChanges(changes.savedValue, changes.modifiedValue)
	})

	// Derived state for checking unsaved changes in webhook settings
	let hasWebhookChanges = $derived.by(() => {
		if (tab !== 'webhook') return false
		const changes = getWebhookSettingsInitialAndModifiedValues()
		if (!changes.savedValue || !changes.modifiedValue) return false
		return hasUnsavedChanges(changes.savedValue, changes.modifiedValue)
	})

	// Derived state for checking unsaved changes in encryption key settings
	let hasEncryptionKeyChanges = $derived.by(() => {
		if (tab !== 'encryption') return false
		const changes = getEncryptionKeySettingsInitialAndModifiedValues()
		if (!changes.savedValue || !changes.modifiedValue) return false
		return hasUnsavedChanges(changes.savedValue, changes.modifiedValue)
	})

	// Derived state for checking unsaved changes in default app settings
	let hasDefaultAppChanges = $derived.by(() => {
		if (tab !== 'default_app') return false
		const changes = getDefaultAppSettingsInitialAndModifiedValues()
		if (!changes.savedValue || !changes.modifiedValue) return false
		return hasUnsavedChanges(changes.savedValue, changes.modifiedValue)
	})

	// Validation effects
	$effect(() => {
		if (webhook !== undefined) {
			const validation = validateWebhookUrl(webhook)
			webhookValidationError = validation.error
		}
	})

	$effect(() => {
		if (editedWorkspaceEncryptionKey !== undefined) {
			const validation = validateEncryptionKey(editedWorkspaceEncryptionKey)
			encryptionKeyValidationError = validation.error
		}
	})
	// All state derived from URL - no local state needed
	let tab = $derived.by(() => {
		const selectedTab = $page.url.searchParams.get('tab') as
			| 'users'
			| 'slack'
			| 'teams'
			| 'premium'
			| 'general'
			| 'webhook'
			| 'deploy_to'
			| 'error_handler'
			| 'success_handler'
			| 'critical_alerts'
			| 'ai'
			| 'windmill_data_tables'
			| 'windmill_lfs'
			| 'git_sync'
			| 'default_app'
			| 'native_triggers'
			| 'encryption'
			| 'dependencies'
			| 'rulesets'
		// Both 'slack' and 'teams' URLs map to 'slack' tab
		if (selectedTab === 'teams') {
			return 'slack'
		}
		// Both 'success_handler' and 'error_handler' URLs map to 'error_handler' tab
		if (selectedTab === 'success_handler') {
			return 'error_handler'
		}
		return selectedTab || 'users'
	})

	let slack_tabs: 'slack_commands' | 'teams_commands' = $derived(
		$page.url.searchParams.get('tab') === 'teams' ? 'teams_commands' : 'slack_commands'
	)

	let usingOpenaiClientCredentialsOauth = $state(false)

	let loadedSettings = $state(false)
	let oauths: Record<string, any> = $state({})

	// OAuth validation functions
	function isSlackOAuthConfigured(slackConfig: any): boolean {
		return slackConfig && slackConfig.id?.trim() && slackConfig.secret?.trim()
	}

	function isTeamsOAuthConfigured(teamsConfig: any): boolean {
		return (
			teamsConfig &&
			teamsConfig.id?.trim() &&
			teamsConfig.secret?.trim() &&
			teamsConfig.tenant?.trim()
		)
	}

	const isSlackOAuthEnabled = $derived(isSlackOAuthConfigured(oauths?.slack))
	const isTeamsOAuthEnabled = $derived(isTeamsOAuthConfigured(oauths?.teams))

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
		// Validate webhook URL if provided
		if (webhook && webhook.trim() !== '') {
			const validation = validateWebhookUrl(webhook)
			if (!validation.isValid) {
				sendUserToast(`Invalid webhook URL: ${validation.error}`, true)
				return
			}
		}

		try {
			if (webhook && webhook.trim() !== '') {
				await WorkspaceService.editWebhook({
					workspace: $workspaceStore!,
					requestBody: { webhook }
				})
				sendUserToast(`webhook set to ${webhook}`)
				initialWebhook = webhook
			} else {
				await WorkspaceService.editWebhook({
					workspace: $workspaceStore!,
					requestBody: { webhook: undefined }
				})
				sendUserToast(`webhook removed`)
				initialWebhook = ''
				webhook = ''
			}
		} catch (error) {
			sendUserToast(`Failed to save webhook: ${error}`, true)
		}
	}

	async function editWorkspaceDefaultApp(): Promise<void> {
		if (emptyString(workspaceDefaultAppPath)) {
			await WorkspaceService.editWorkspaceDefaultApp({
				workspace: $workspaceStore!,
				requestBody: {
					default_app_path: undefined
				}
			})
		} else {
			await WorkspaceService.editWorkspaceDefaultApp({
				workspace: $workspaceStore!,
				requestBody: {
					default_app_path: workspaceDefaultAppPath
				}
			})
		}
		sendUserToast('Default app settings saved')
		initialWorkspaceDefaultAppPath = workspaceDefaultAppPath
	}

	async function saveDefaultAppSettings(): Promise<void> {
		if (workspaceDefaultAppPath !== initialWorkspaceDefaultAppPath) {
			await editWorkspaceDefaultApp()
		}
		if (publicAppRateLimitPerMinute !== initialPublicAppRateLimitPerMinute) {
			await editPublicAppRateLimit()
		}
	}

	async function loadWorkspaceEncryptionKey(): Promise<void> {
		let resp = await WorkspaceService.getWorkspaceEncryptionKey({
			workspace: $workspaceStore!
		})
		workspaceEncryptionKey = resp.key
		editedWorkspaceEncryptionKey = resp.key
		initialEditedWorkspaceEncryptionKey = resp.key
	}

	async function setWorkspaceEncryptionKey(): Promise<void> {
		if (
			emptyString(editedWorkspaceEncryptionKey) ||
			workspaceEncryptionKey === editedWorkspaceEncryptionKey
		) {
			return
		}

		// Validate encryption key
		const validation = validateEncryptionKey(editedWorkspaceEncryptionKey!)
		if (!validation.isValid) {
			sendUserToast(`Invalid encryption key: ${validation.error}`, true)
			return
		}

		try {
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
		} catch (error) {
			workspaceReencryptionInProgress = false
			sendUserToast(`Failed to set encryption key: ${error}`, true)
		}
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
		const errorHandler = settings.error_handler as
			| { path?: string; extra_args?: any; muted_on_cancel?: boolean; muted_on_user_path?: boolean }
			| undefined
		const errorHandlerPath = errorHandler?.path ?? ''
		errorHandlerItemKind = errorHandlerPath
			? (errorHandlerPath.split('/')[0] as 'flow' | 'script')
			: 'script'
		errorHandlerScriptPath = errorHandlerPath.split('/').slice(1).join('/')
		errorHandlerMutedOnCancel = errorHandler?.muted_on_cancel
		errorHandlerMutedOnUserPath = errorHandler?.muted_on_user_path
		criticalAlertUIMuted = settings.mute_critical_alerts
		initialCriticalAlertUIMuted = settings.mute_critical_alerts
		publicAppRateLimitPerMinute = settings.public_app_execution_limit_per_minute ?? undefined
		initialPublicAppRateLimitPerMinute = settings.public_app_execution_limit_per_minute ?? undefined
		if (emptyString($enterpriseLicense)) {
			errorHandlerSelected = 'custom'
		} else {
			errorHandlerSelected = getHandlerType(errorHandlerScriptPath)
		}
		errorHandlerExtraArgs = errorHandler?.extra_args ?? {}
		const successHandler = settings.success_handler as
			| { path?: string; extra_args?: any }
			| undefined
		successHandlerScriptPath = (successHandler?.path ?? '').split('/').slice(1).join('/')
		workspaceDefaultAppPath = settings.default_app
		initialWorkspaceDefaultAppPath = settings.default_app

		s3ResourceSettings = convertBackendSettingsToFrontendSettings(
			settings.large_file_storage,
			!!$enterpriseLicense
		)
		s3ResourceSavedSettings = clone(s3ResourceSettings)
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

		// Store initial deploy settings state for unsaved changes detection
		initialWorkspaceToDeployTo = workspaceToDeployTo
		initialDeployUiSettings = clone(deployUiSettings)

		// Store initial webhook state for unsaved changes detection
		initialWebhook = webhook

		// Store initial encryption key state for unsaved changes detection
		initialEditedWorkspaceEncryptionKey = editedWorkspaceEncryptionKey

		// Store initial error handler state for unsaved changes detection
		initialErrorHandlerSelected = errorHandlerSelected
		initialErrorHandlerScriptPath = errorHandlerScriptPath
		initialErrorHandlerItemKind = errorHandlerItemKind
		initialErrorHandlerExtraArgs = clone(errorHandlerExtraArgs)
		initialErrorHandlerMutedOnCancel = errorHandlerMutedOnCancel
		initialErrorHandlerMutedOnUserPath = errorHandlerMutedOnUserPath

		// Store initial success handler state for unsaved changes detection
		initialSuccessHandlerScriptPath = successHandlerScriptPath

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
			slackAppType = config.slack_oauth_client_id ? 'workspace' : 'instance'
			slackOAuthClientId = config.slack_oauth_client_id || ''
			slackOAuthClientSecret = config.slack_oauth_client_secret || ''
			slackOAuthConfigLoaded = !!config.slack_oauth_client_id
		} catch (e) {
			console.error('Failed to load Slack OAuth config:', e)
		}
	}

	async function loadGlobalOAuthSettings(): Promise<void> {
		try {
			oauths = (await SettingService.getGlobal({ key: 'oauths' })) ?? {}
		} catch (e) {
			console.error('Failed to load global OAuth config:', e)
			oauths = {}
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
			slackAppType = 'instance'
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

	let deployUiSettings: {
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
	} = $state({
		include_path: [],
		include_type: {
			scripts: true,
			flows: true,
			apps: true,
			resources: true,
			variables: true,
			secrets: true,
			triggers: true
		}
	})

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				loadSettings()
				loadSlackOAuthConfig()
				loadGlobalOAuthSettings()
			})
		}
	})

	async function editErrorHandler() {
		if (errorHandlerScriptPath) {
			await WorkspaceService.editErrorHandler({
				workspace: $workspaceStore!,
				requestBody: {
					path: `${errorHandlerItemKind}/${errorHandlerScriptPath}`,
					extra_args: errorHandlerExtraArgs,
					muted_on_cancel: errorHandlerMutedOnCancel,
					muted_on_user_path: errorHandlerMutedOnUserPath
				}
			})
			sendUserToast(`workspace error handler set to ${errorHandlerScriptPath}`)
		} else {
			await WorkspaceService.editErrorHandler({
				workspace: $workspaceStore!,
				requestBody: {
					path: undefined,
					extra_args: undefined,
					muted_on_cancel: undefined,
					muted_on_user_path: undefined
				}
			})
			sendUserToast(`workspace error handler removed`)
		}

		// Update initial values for dirty detection
		initialErrorHandlerSelected = errorHandlerSelected
		initialErrorHandlerScriptPath = errorHandlerScriptPath
		initialErrorHandlerItemKind = errorHandlerItemKind
		initialErrorHandlerExtraArgs = clone(errorHandlerExtraArgs)
		initialErrorHandlerMutedOnCancel = errorHandlerMutedOnCancel
		initialErrorHandlerMutedOnUserPath = errorHandlerMutedOnUserPath
	}

	async function editSuccessHandler() {
		if (successHandlerScriptPath) {
			await WorkspaceService.editSuccessHandler({
				workspace: $workspaceStore!,
				requestBody: {
					path: `script/${successHandlerScriptPath}`
				}
			})
			sendUserToast(`workspace success handler set to ${successHandlerScriptPath}`)
		} else {
			await WorkspaceService.editSuccessHandler({
				workspace: $workspaceStore!,
				requestBody: {
					path: undefined
				}
			})
			sendUserToast(`workspace success handler removed`)
		}

		// Update initial value for dirty detection
		initialSuccessHandlerScriptPath = successHandlerScriptPath
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

		// Update initial value for dirty detection
		initialCriticalAlertUIMuted = criticalAlertUIMuted

		// reload page after change of setting
		setTimeout(() => {
			window.location.reload()
		}, 3000)
	}

	async function editPublicAppRateLimit() {
		await SettingService.setPublicAppRateLimit({
			workspace: $workspaceStore!,
			requestBody: {
				public_app_execution_limit_per_minute: publicAppRateLimitPerMinute
			}
		})
		initialPublicAppRateLimitPerMinute = publicAppRateLimitPerMinute
		sendUserToast(
			publicAppRateLimitPerMinute
				? `Public app rate limit set to ${publicAppRateLimitPerMinute} per minute per server`
				: `Public app rate limit disabled`
		)
	}

	// Function to check if there are unsaved changes in AI settings
	function getAiSettingsInitialAndModifiedValues() {
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
		const savedValue = {
			s3ResourceSettings: s3ResourceSavedSettings,
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
		s3ResourceSettings = clone(s3ResourceSavedSettings)
		ducklakeSettings = clone(ducklakeSavedSettings)
	}

	// Function to check if there are unsaved changes in deploy settings
	function getDeploySettingsInitialAndModifiedValues() {
		// Normalize empty strings to undefined for consistent comparison
		const normalizeWorkspaceValue = (value: string | undefined) =>
			value === '' ? undefined : value

		const savedValue = {
			workspaceToDeployTo: normalizeWorkspaceValue(initialWorkspaceToDeployTo),
			deployUiSettings: initialDeployUiSettings
		}

		const modifiedValue = {
			workspaceToDeployTo: normalizeWorkspaceValue(workspaceToDeployTo),
			deployUiSettings: deployUiSettings
		}

		return { savedValue, modifiedValue }
	}

	// Function to discard unsaved deploy settings changes
	function discardDeploySettingsChanges() {
		workspaceToDeployTo = initialWorkspaceToDeployTo
		deployUiSettings = clone(initialDeployUiSettings)
	}

	// Function to check if there are unsaved changes in webhook settings
	function getWebhookSettingsInitialAndModifiedValues() {
		// Normalize empty strings to undefined for consistent comparison
		const normalizeWebhookValue = (value: string | undefined) =>
			value && value.trim() !== '' ? value : undefined

		const savedValue = {
			webhook: normalizeWebhookValue(initialWebhook)
		}

		const modifiedValue = {
			webhook: normalizeWebhookValue(webhook)
		}

		return { savedValue, modifiedValue }
	}

	// Function to discard unsaved webhook settings changes
	function discardWebhookSettingsChanges() {
		webhook = initialWebhook || ''
	}

	// Function to check if there are unsaved changes in encryption key settings
	function getEncryptionKeySettingsInitialAndModifiedValues() {
		const savedValue = {
			editedWorkspaceEncryptionKey: initialEditedWorkspaceEncryptionKey
		}

		const modifiedValue = {
			editedWorkspaceEncryptionKey: editedWorkspaceEncryptionKey
		}

		return { savedValue, modifiedValue }
	}

	// Function to discard unsaved encryption key settings changes
	function discardEncryptionKeySettingsChanges() {
		editedWorkspaceEncryptionKey = initialEditedWorkspaceEncryptionKey
	}

	// Function to check if there are unsaved changes in default app settings
	function getDefaultAppSettingsInitialAndModifiedValues() {
		return {
			savedValue: {
				defaultAppPath: initialWorkspaceDefaultAppPath,
				publicAppRateLimitPerMinute: initialPublicAppRateLimitPerMinute
			},
			modifiedValue: {
				defaultAppPath: workspaceDefaultAppPath,
				publicAppRateLimitPerMinute: publicAppRateLimitPerMinute
			}
		}
	}

	// Function to discard unsaved default app settings changes
	function discardDefaultAppSettingsChanges() {
		workspaceDefaultAppPath = initialWorkspaceDefaultAppPath
		publicAppRateLimitPerMinute = initialPublicAppRateLimitPerMinute
	}

	// Strip keys from extraArgs that are auto-managed by child components:
	// - 'slack': computed by ErrorOrRecoveryHandler's $effect based on handler type
	// - 'channel_name': display metadata stripped by SchemaForm's removeExtraKey()
	function normalizeHandlerExtraArgs(args: Record<string, any>): Record<string, any> {
		const { slack: _, channel_name: __, ...rest } = args
		return rest
	}

	// Function to check if there are unsaved changes in error handler settings
	function getErrorHandlerSettingsInitialAndModifiedValues() {
		const savedValue = {
			errorHandlerSelected: initialErrorHandlerSelected,
			errorHandlerScriptPath: initialErrorHandlerScriptPath,
			errorHandlerItemKind: initialErrorHandlerItemKind,
			errorHandlerExtraArgs: normalizeHandlerExtraArgs(initialErrorHandlerExtraArgs),
			errorHandlerMutedOnCancel: initialErrorHandlerMutedOnCancel,
			errorHandlerMutedOnUserPath: initialErrorHandlerMutedOnUserPath
		}

		const modifiedValue = {
			errorHandlerSelected: errorHandlerSelected,
			errorHandlerScriptPath: errorHandlerScriptPath,
			errorHandlerItemKind: errorHandlerItemKind,
			errorHandlerExtraArgs: normalizeHandlerExtraArgs(errorHandlerExtraArgs),
			errorHandlerMutedOnCancel: errorHandlerMutedOnCancel,
			errorHandlerMutedOnUserPath: errorHandlerMutedOnUserPath
		}

		return { savedValue, modifiedValue }
	}

	// Function to discard unsaved error handler settings changes
	function discardErrorHandlerSettingsChanges() {
		errorHandlerSelected = initialErrorHandlerSelected
		errorHandlerScriptPath = initialErrorHandlerScriptPath
		errorHandlerItemKind = initialErrorHandlerItemKind
		errorHandlerExtraArgs = clone(initialErrorHandlerExtraArgs)
		errorHandlerMutedOnCancel = initialErrorHandlerMutedOnCancel
		errorHandlerMutedOnUserPath = initialErrorHandlerMutedOnUserPath
	}

	// Combined function to check for unsaved changes across all tabs
	function getAllUnsavedChanges() {
		switch (tab) {
			case 'windmill_data_tables':
				return dataTableSettingsComponent?.unsavedChanges() ?? { savedValue: {}, modifiedValue: {} }
			case 'ai':
				return getAiSettingsInitialAndModifiedValues()
			case 'windmill_lfs':
				return getStorageSettingsInitialAndModifiedValues()
			case 'deploy_to':
				return getDeploySettingsInitialAndModifiedValues()
			case 'webhook':
				return getWebhookSettingsInitialAndModifiedValues()
			case 'encryption':
				return getEncryptionKeySettingsInitialAndModifiedValues()
			case 'error_handler': {
				const errorValues = getErrorHandlerSettingsInitialAndModifiedValues()
				return {
					savedValue: {
						...(errorValues.savedValue ?? {}),
						successHandlerScriptPath: initialSuccessHandlerScriptPath
					},
					modifiedValue: {
						...(errorValues.modifiedValue ?? {}),
						successHandlerScriptPath: successHandlerScriptPath
					}
				}
			}
			case 'critical_alerts':
				return {
					savedValue: { criticalAlertUIMuted: initialCriticalAlertUIMuted },
					modifiedValue: { criticalAlertUIMuted: criticalAlertUIMuted }
				}
			case 'default_app':
				return getDefaultAppSettingsInitialAndModifiedValues()
			default:
				return { savedValue: {}, modifiedValue: {} }
		}
	}

	// Combined function to discard changes based on current tab
	function discardAllChanges() {
		switch (tab) {
			case 'ai':
				discardAiSettingsChanges()
				break
			case 'windmill_lfs':
				discardStorageSettingsChanges()
				break
			case 'deploy_to':
				discardDeploySettingsChanges()
				break
			case 'webhook':
				discardWebhookSettingsChanges()
				break
			case 'encryption':
				discardEncryptionKeySettingsChanges()
				break
			case 'error_handler':
				discardErrorHandlerSettingsChanges()
				successHandlerScriptPath = initialSuccessHandlerScriptPath
				break
			case 'critical_alerts':
				criticalAlertUIMuted = initialCriticalAlertUIMuted
				break
			case 'windmill_data_tables':
				dataTableSettingsComponent?.discard()
				break
			case 'default_app':
				discardDefaultAppSettingsChanges()
				break
		}
	}

	// Navigation groups for sidebar
	const navigationGroups = $derived([
		{
			items: [
				{
					id: 'general',
					label: 'General',
					aiId: 'workspace-settings-general',
					aiDescription: 'General workspace settings'
				},
				{
					id: 'users',
					label: 'Users',
					aiId: 'workspace-settings-users',
					aiDescription: 'Users workspace settings'
				},
				{
					id: 'ai',
					label: 'Windmill AI',
					aiId: 'workspace-settings-ai',
					aiDescription: 'Windmill AI workspace settings'
				},
				{
					id: 'premium',
					label: 'Premium plans',
					aiId: 'workspace-settings-premium',
					aiDescription: 'Premium plans workspace settings',
					showIf: isCloudHosted()
				}
			]
		},
		{
			title: 'Git & deployment',
			items: [
				{
					id: 'git_sync',
					label: 'Git sync',
					aiId: 'workspace-settings-git-sync',
					aiDescription: 'Git sync workspace settings',
					isEE: true
				},
				{
					id: 'deploy_to',
					label: 'Deployment UI',
					aiId: 'workspace-settings-deploy-to',
					aiDescription: 'Deployment UI workspace settings',
					isEE: true
				},
				{
					id: 'rulesets',
					label: 'Rulesets',
					aiId: 'workspace-settings-rulesets',
					aiDescription: 'Protection Rulesets workspace settings',
					isEE: true
				}
			]
		},
		{
			title: 'Integrations',
			items: [
				{
					id: 'slack',
					label: 'Slack / Teams',
					aiId: 'workspace-settings-slack',
					aiDescription: 'Slack / Teams workspace settings',
					showIf: WORKSPACE_SHOW_SLACK_CMD
				},
				{
					id: 'webhook',
					label: 'Webhook',
					aiId: 'workspace-settings-webhook',
					aiDescription: 'Webhook workspace settings',
					showIf: WORKSPACE_SHOW_WEBHOOK_CLI_SYNC
				},
				{
					id: 'native_triggers',
					label: 'Native triggers',
					aiId: 'workspace-settings-integrations',
					aiDescription: 'Workspace integrations for native triggers'
				}
			]
		},
		{
			title: 'Hooks',
			items: [
				{
					id: 'error_handler',
					label: 'Error / success handler',
					aiId: 'workspace-settings-error-handler',
					aiDescription: 'Error and success handler workspace settings',
					isEE: true
				},
				{
					id: 'critical_alerts',
					label: 'Critical alerts',
					aiId: 'workspace-settings-critical-alerts',
					aiDescription: 'Critical alerts workspace settings',
					isEE: true
				}
			]
		},
		{
			title: 'Data & storage',
			items: [
				{
					id: 'windmill_data_tables',
					label: 'Data tables',
					aiId: 'workspace-settings-windmill-data-tables',
					aiDescription: 'Data tables workspace settings'
				},
				{
					id: 'windmill_lfs',
					label: 'Object storage (S3)',
					aiId: 'workspace-settings-windmill-lfs',
					aiDescription: 'Object Storage (S3) workspace settings'
				}
			]
		},
		{
			title: 'Advanced',
			items: [
				{
					id: 'default_app',
					label: 'Apps',
					aiId: 'workspace-settings-apps',
					aiDescription: 'Apps workspace settings',
					isEE: true
				},
				{
					id: 'dependencies',
					label: 'Dependencies',
					aiId: 'workspace-settings-dependencies',
					aiDescription: 'Workspace dependencies settings'
				},
				{
					id: 'encryption',
					label: 'Encryption',
					aiId: 'workspace-settings-encryption',
					aiDescription: 'Encryption workspace settings'
				}
			]
		}
	])
</script>

<CenteredPage wrapperClasses="pb-0 h-screen" handleOverflow={false} class="flex flex-col h-full">
	{#if $userStore?.is_admin || $superadmin}
		<PageHeader title="Workspace settings: {$workspaceStore}"
			>{#if $superadmin}
				<Button variant="default" size="sm" on:click={() => goto('#superadmin-settings')}>
					Instance settings
				</Button>
			{/if}</PageHeader
		>

		<div class="flex grow min-h-0 pb-4">
			<!-- Sidebar Navigation -->
			<div class="w-60 shrink-0 h-full overflow-auto pb-4 pr-4 -ml-2">
				<SidebarNavigation
					groups={navigationGroups}
					selectedId={tab}
					onNavigate={(id) => {
						const params = new URLSearchParams($page.url.searchParams)
						params.set('tab', id)
						goto(`?${params.toString()}`)
					}}
				/>
			</div>

			<!-- Main Content -->
			<div class="flex-1 min-w-0 h-full">
				<div class="h-full overflow-auto">
					<div class="h-fit px-6" style="scrollbar-gutter: stable both-edges;">
						{#if !loadedSettings}
							<Skeleton layout={[1, [40]]} />
						{:else if tab == 'users'}
							<WorkspaceUserSettings />
						{:else if tab == 'deploy_to'}
							<SettingsPageHeader
								title="Link this workspace to another staging / prod workspace"
								description="Connecting this workspace with another staging/production workspace enables web-based deployment to that workspace."
								link="https://www.windmill.dev/docs/core_concepts/staging_prod"
							/>
							{#if $enterpriseLicense}
								<DeployToSetting
									bind:workspaceToDeployTo
									bind:deployUiSettings
									hasUnsavedChanges={hasDeploySettingsChanges}
									onSave={() => {
										// Update initial state after successful save
										initialWorkspaceToDeployTo = workspaceToDeployTo
										initialDeployUiSettings = clone(deployUiSettings)
									}}
									onDiscard={discardDeploySettingsChanges}
									onWorkspaceToDeployToSave={(newWorkspaceToDeployTo) => {
										// Update initial state after workspace to deploy to is saved
										initialWorkspaceToDeployTo = newWorkspaceToDeployTo
									}}
								/>
							{:else}
								<div class="my-2"
									><Alert type="warning" title="Enterprise license required"
										>Deploy to staging/prod from the web UI is only available with an enterprise
										license</Alert
									></div
								>
							{/if}
						{:else if tab == 'rulesets'}
							<SettingsPageHeader
								title="Workspace Protection Rulesets"
								description="Create and manage protection rules that define restrictions and specify which groups and users can bypass those restrictions. Users not in any bypass list will be subject to the configured rules."
							/>
							<WorkspaceRulesets />
						{:else if tab == 'premium'}
							<PremiumInfo {customer_id} {plan} />
						{:else if tab == 'slack'}
							<SettingsPageHeader
								title="Workspace connections to Slack and Teams"
								description="With workspace connections, you can trigger scripts or flows with a '/windmill' command with your Slack or Teams bot or set the workspace error handler to send notifications to your Slack or Teams channel."
								link="https://www.windmill.dev/docs/core_concepts/error_handling#workspace-error-handler"
							/>
							<div class="space-y-6">
								<Tabs
									selected={slack_tabs}
									on:selected={(e) => {
										const params = new URLSearchParams($page.url.searchParams)
										if (e.detail === 'teams_commands') {
											params.set('tab', 'teams')
										} else {
											params.set('tab', 'slack')
										}
										goto(`?${params.toString()}`)
									}}
								>
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
											if (slackOAuthConfigLoaded) {
												deleteSlackOAuthConfig()
											} else {
												await OauthService.disconnectSlack({ workspace: $workspaceStore ?? '' })
												loadSettings()
												sendUserToast('Disconnected Slack')
											}
										}}
										onSelect={editSlackCommand}
										connectHref="{base}/api/oauth/connect_slack"
										createScriptHref="{base}/scripts/add?hub=hub%2F28071%2Fslack%2Fexample_of_responding_to_a_slack_command_slack"
										createFlowHref="{base}/flows/add?hub=28"
										documentationLink="https://www.windmill.dev/docs/integrations/slack"
										onLoadSettings={loadSettings}
										display_name={slack_team_name}
										hideConnectButton={useCustomSlackApp && !slackOAuthConfigLoaded}
										isOAuthEnabled={isSlackOAuthEnabled}
										workspaceSpecificConnection={slackOAuthConfigLoaded}
									>
										{#snippet workspaceConfig()}
											<!-- Workspace OAuth Configuration Section -->
											{#if !slack_team_name}
												<div class="flex flex-col gap-1">
													<!-- Show toggle buttons for app type selection -->
													<ToggleButtonGroup bind:selected={slackAppType}>
														{#snippet children({ item })}
															<ToggleButton
																{item}
																value="instance"
																label="Instance specific Slack app"
															/>
															<ToggleButton
																{item}
																value="workspace"
																label="Workspace specific Slack app"
															/>
														{/snippet}
													</ToggleButtonGroup>
													<div class="text-2xs text-hint"
														>Use the Slack app configured at the instance level if you want to use
														the same Slack app for all workspaces. Configure your Slack app here if
														you want to use a specific Slack app for this workspace.</div
													>
												</div>
											{/if}
											{#if slackOAuthConfigLoaded}
												<!-- Show saved config with delete button -->
												<div class="flex flex-col gap-1">
													<div class="text-xs text-primary font-normal">Client ID</div>
													<TextInput
														inputProps={{
															type: 'text',
															readonly: true
														}}
														value={slackOAuthClientId}
													/>
													<div class="text-2xs text-hint"
														>Client ID for the Slack app configured at the workspace level</div
													>
												</div>
											{:else if slackAppType === 'workspace'}
												<div class="flex flex-col gap-6">
													<label class="flex flex-col gap-1">
														<span class="text-primary font-semibold text-xs">Client ID</span>
														<TextInput
															inputProps={{
																type: 'text',
																placeholder: '1234567890.1234567890'
															}}
															bind:value={slackOAuthClientId}
														/>
													</label>

													<label class="flex flex-col gap-1">
														<span class="text-primary font-semibold text-xs">Client secret</span>
														<TextInput
															inputProps={{
																type: 'password',
																placeholder: 'Enter client secret'
															}}
															bind:value={slackOAuthClientSecret}
														/>
													</label>

													<CollapseLink text="Instructions">
														<div class="text-xs text-secondary">
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
											{:else if !isSlackOAuthEnabled}
												<Alert type="warning" title="Slack OAuth not configured">
													Slack OAuth is not configured at the instance level. Please ask your
													administrator to configure Slack OAuth settings in the instance settings
													before you can use Slack features.
												</Alert>
											{/if}
										{/snippet}
									</ConnectionSection>
								{:else if slack_tabs === 'teams_commands'}
									{#if !$enterpriseLicense}
										<div class="pt-4"></div>
										<Alert type="warning" title="Workspace Teams commands is an EE feature">
											Workspace Teams commands is a Windmill EE feature. It enables using your
											current Slack / Teams connection to run a custom script and send
											notifications.
										</Alert>
										<div class="pb-2"></div>
									{:else}
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
											isOAuthEnabled={isTeamsOAuthEnabled}
										>
											{#snippet workspaceConfig()}
												{#if !isTeamsOAuthEnabled}
													<Alert type="warning" title="Teams OAuth not configured">
														Teams OAuth is not configured at the instance level. Please ask your
														administrator to configure Teams OAuth settings in the instance settings
														before you can use Teams features.
													</Alert>
												{/if}
											{/snippet}
										</ConnectionSection>
									{/if}
								{/if}
							</div>
						{:else if tab == 'general'}
							<SettingsPageHeader
								title="General"
								description="Configure general workspace settings."
								link="https://www.windmill.dev/docs/core_concepts/workspace_settings"
							/>

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
								<p class="text-2xs text-secondary">
									Only instance superadmins can delete a workspace.
								</p>
							{/if}
							{#if $workspaceStore === 'admins' || $workspaceStore === 'starter'}
								<p class="text-2xs text-secondary">
									This workspace cannot be deleted as it has a special function. Consult the
									documentation for more information.
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
							<SettingsPageHeader
								title="Workspace webhook"
								description="Connect your Windmill workspace to an external service to sync or get notified about any change."
								link="https://www.windmill.dev/docs/core_concepts/webhooks#workspace-webhook"
							/>

							<SettingCard
								label="URL to send requests to"
								description="This URL will be POSTed to with a JSON body depending on the type of event. The type is indicated by the type field. The other fields are dependent on the type."
							>
								<TextInput
									bind:value={webhook}
									inputProps={{
										placeholder: 'https://your-endpoint.com/webhook'
									}}
									error={webhookValidationError}
									class="max-w-lg"
								/>
								{#if webhookValidationError}
									<div class="text-xs text-red-600 dark:text-red-400">{webhookValidationError}</div>
								{/if}
							</SettingCard>
							<SettingsFooter
								hasUnsavedChanges={hasWebhookChanges}
								onSave={editWebhook}
								onDiscard={discardWebhookSettingsChanges}
								saveLabel="Save webhook"
								disabled={!!webhookValidationError}
								class="mt-8"
							/>
						{:else if tab == 'error_handler'}
							<SettingsPageHeader
								title="Workspace error / success handler"
								description="Configure handlers that automatically execute when scripts or flows in the workspace fail or succeed."
								link="https://www.windmill.dev/docs/core_concepts/error_handling#workspace-error-handler"
							/>
							{#if !$enterpriseLicense}
								<Alert type="warning" title="Workspace error/success handler is an EE feature">
									Workspace error and success handlers are Windmill EE features.
								</Alert>
							{/if}

							<Section label="Error handler">
								<div class="flex flex-col gap-6">
									<ErrorOrRecoveryHandler
										noMargin
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
																<b>email</b>: The email of the user who ran the script or flow that
																errored.
															</li>
															<li><b>error</b>: The error details.</li>
															<li><b>job_id</b>: The job id.</li>
															<li><b>is_flow</b>: Whether the error comes from a flow.</li>
															<li
																><b>workspace_id</b>: The workspace id of the failed script or flow.</li
															>
														</ul>
														<br />
														The error handler will be executed by the automatically created group g/error_handler.
														If your error handler requires variables or resources, you need to add them
														to the group.
													</div>
												</div>
											</Tooltip>
										{/snippet}
									</ErrorOrRecoveryHandler>

									<SettingCard class="gap-2">
										<Toggle
											disabled={!$enterpriseLicense ||
												((errorHandlerSelected === 'slack' || errorHandlerSelected === 'teams') &&
													!emptyString(errorHandlerScriptPath) &&
													emptyString(errorHandlerExtraArgs['channel']))}
											bind:checked={errorHandlerMutedOnCancel}
											options={{ right: 'Do not run error handler for canceled jobs' }}
										/>
										<Toggle
											disabled={!$enterpriseLicense ||
												((errorHandlerSelected === 'slack' || errorHandlerSelected === 'teams') &&
													!emptyString(errorHandlerScriptPath) &&
													emptyString(errorHandlerExtraArgs['channel']))}
											bind:checked={errorHandlerMutedOnUserPath}
											options={{ right: 'Do not run error handler for u/ scripts and flows' }}
										/>
									</SettingCard>
								</div>

								<SettingsFooter
									class="mt-6"
									inline
									hasUnsavedChanges={hasErrorHandlerChanges}
									onSave={editErrorHandler}
									onDiscard={discardErrorHandlerSettingsChanges}
									saveLabel="Save error handler"
									disabled={!$enterpriseLicense ||
										((errorHandlerSelected === 'slack' || errorHandlerSelected === 'teams') &&
											!emptyString(errorHandlerScriptPath) &&
											emptyString(errorHandlerExtraArgs['channel']))}
								/>
							</Section>

							<div class="pt-8 border-b mb-8"></div>

							<Section label="Success handler">
								<div class="flex flex-col gap-6">
									<div class="flex flex-col gap-4">
										<div class="flex flex-row gap-2 items-center">
											<ScriptPicker
												disabled={!$enterpriseLicense}
												initialPath={successHandlerScriptPath}
												allowRefresh
												itemKind="script"
												on:select={(ev) => {
													successHandlerScriptPath = ev?.detail?.path
												}}
												clearable
											/>
											<Button
												variant="default"
												href={`${base}/scripts/add?lang=bun#` +
													encodeState({
														path: 'f/success_handler',
														summary: 'Workspace Success Handler',
														description:
															'Called when any job in the workspace completes successfully',
														content: `//native

// Workspace Success Handler
// This script is called whenever a job completes successfully in this workspace.

export async function main(
  path: string,
  email: string,
  result: any,
  job_id: string,
  is_flow: boolean,
  workspace_id: string,
  started_at: string
) {
  console.log(\`Job \${job_id} completed successfully\`)
  console.log(\`Path: \${path}, Is Flow: \${is_flow}\`)
  console.log(\`Result:\`, result)

  // Add your success handling logic here
  // Examples:
  // - Send a notification
  // - Update an external system
  // - Log to a database
  // - Trigger another workflow

  return { handled: true }
}
`,
														language: 'bun',
														kind: 'script'
													})}
												target="_blank"
											>
												Create from template
											</Button>
										</div>
									</div>
								</div>

								<SettingsFooter
									class="mt-6"
									inline
									hasUnsavedChanges={hasSuccessHandlerChanges}
									onSave={editSuccessHandler}
									onDiscard={() => {
										successHandlerScriptPath = initialSuccessHandlerScriptPath
									}}
									saveLabel="Save success handler"
									disabled={!$enterpriseLicense}
								/>
							</Section>
						{:else if tab == 'critical_alerts'}
							<SettingsPageHeader
								title="Workspace critical alerts"
								description="Critical alerts within the scope of a workspace are sent to the workspace admins through a UI notification."
								link="https://www.windmill.dev/docs/core_concepts/critical_alerts"
							/>
							<div class="flex flex-col gap-6 py-4">
								{#if !$enterpriseLicense}
									<Alert type="info" title="Workspace critical alerts is an EE feature">
										Workspace critical alerts is a Windmill Enterprise Edition feature that sends
										notifications to workspace admins when critical events occur.
									</Alert>
								{/if}
								<Toggle
									disabled={!$enterpriseLicense}
									bind:checked={criticalAlertUIMuted}
									options={{ right: 'Mute critical alerts UI for this workspace' }}
								/>

								<div class="flex gap-2">
									<Button
										disabled={!$enterpriseLicense}
										on:click={() => isCriticalAlertsUIOpen.set(true)}
										btnClasses="w-fit"
									>
										Show critical alerts
									</Button>
								</div>
							</div>

							<SettingsFooter
								hasUnsavedChanges={hasCriticalAlertMuteChanges}
								onSave={editCriticalAlertMuteSetting}
								onDiscard={() => {
									criticalAlertUIMuted = initialCriticalAlertUIMuted
								}}
								saveLabel="Save mute setting"
								disabled={!$enterpriseLicense}
							/>
						{:else if tab == 'ai'}
							<AISettings
								bind:aiProviders
								bind:codeCompletionModel
								bind:defaultModel
								bind:customPrompts
								bind:maxTokensPerModel
								bind:usingOpenaiClientCredentialsOauth
								hasUnsavedChanges={hasAiSettingsChanges}
								onDiscard={discardAiSettingsChanges}
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
								{s3ResourceSavedSettings}
								onSave={() => {
									s3ResourceSavedSettings = clone(s3ResourceSettings)
								}}
								onDiscard={() => {
									s3ResourceSettings = clone(s3ResourceSavedSettings)
								}}
							/>
							<DucklakeSettings
								bind:ducklakeSettings
								bind:ducklakeSavedSettings
								onSave={() => {
									ducklakeSavedSettings = clone(ducklakeSettings)
								}}
								onDiscard={() => {
									ducklakeSettings = clone(ducklakeSavedSettings)
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
							<SettingsPageHeader
								title="Workspace default app"
								description="If configured, users who are operators in this workspace will be redirected to this app automatically when logging into this workspace. Make sure the default app is shared with all the operators of this workspace before turning this feature on."
								link="https://www.windmill.dev/docs/apps/default_app"
							/>
							{#if !$enterpriseLicense}
								<Alert type="warning" title="Windmill EE only feature">
									Default app can only be set on Windmill Enterprise Edition.
								</Alert>
							{:else}
								<Alert type="info" title="Default app must be accessible to all operators">
									Make sure the default app is shared with all the operators of this workspace
									before turning this feature on.
								</Alert>
							{/if}
							<SettingCard label="App" class="mt-6">
								<ScriptPicker bind:scriptPath={workspaceDefaultAppPath} itemKind="app" clearable />
							</SettingCard>

							<SettingCard
								label="Rate limiting"
								description="Limit the number of public (anonymous) app executions per minute per server. Set to 0 or leave empty to disable. This is a per-server limit, not a global limit."
								class="mt-6"
							>
								<TextInput
									inputProps={{ type: 'number', placeholder: '0 (disabled)' }}
									bind:value={publicAppRateLimitPerMinute}
									class="w-48"
								/>
								<span class="text-hint text-2xs">executions per minute per server</span>
							</SettingCard>

							<SettingsFooter
								class="mt-8"
								hasUnsavedChanges={hasDefaultAppChanges}
								onSave={saveDefaultAppSettings}
								onDiscard={discardDefaultAppSettingsChanges}
								saveLabel="Save app settings"
								disabled={!$enterpriseLicense}
							/>
						{:else if tab == 'native_triggers'}
							{#if $workspaceStore}
								{#await import('$lib/components/workspaceSettings/WorkspaceIntegrations.svelte') then { default: WorkspaceIntegrations }}
									<WorkspaceIntegrations />
								{/await}
							{:else}
								<div class="flex items-center justify-center p-8">
									<div class="text-sm text-secondary">Loading workspace...</div>
								</div>
							{/if}
						{:else if tab == 'encryption'}
							<SettingsPageHeader
								title="Workspace secret encryption"
								description="When updating the encryption key of a workspace, all secrets will be re-encrypted with the new key and the previous key will be replaced by the new one. If you're manually updating the key to match another workspace key from another Windmill instance, make sure not to use the 'SECRET_SALT' environment variable or, if you're using it, make sure it the salt matches across both instances."
								link="https://www.windmill.dev/docs/core_concepts/workspace_secret_encryption"
							/>
							<SettingCard label="Workspace encryption key" class="mt-6">
								<div class="flex gap-2">
									<TextInput
										inputProps={{
											id: 'workspace-encryption-key',
											placeholder: '*'.repeat(64)
										}}
										bind:value={editedWorkspaceEncryptionKey}
										error={encryptionKeyValidationError}
									/>
									<Button
										variant="default"
										unifiedSize="md"
										on:click={() => {
											loadWorkspaceEncryptionKey()
										}}>Load current key</Button
									>
								</div>
								{#if encryptionKeyValidationError}
									<div class="text-xs text-red-600">
										{encryptionKeyValidationError}
									</div>
								{/if}
							</SettingCard>

							<SettingsFooter
								class="mt-8"
								hasUnsavedChanges={hasEncryptionKeyChanges}
								onSave={setWorkspaceEncryptionKey}
								onDiscard={discardEncryptionKeySettingsChanges}
								saveLabel="Save & Re-encrypt workspace"
								disabled={!!encryptionKeyValidationError || workspaceReencryptionInProgress}
							/>
						{/if}
					</div>
				</div>
			</div>
		</div>
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
