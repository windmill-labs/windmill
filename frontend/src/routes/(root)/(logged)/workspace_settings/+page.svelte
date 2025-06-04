<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import { isCloudHosted } from '$lib/cloud'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Button, Skeleton, Tab, Tabs } from '$lib/components/common'

	import DeployToSetting from '$lib/components/DeployToSetting.svelte'
	import ErrorOrRecoveryHandler from '$lib/components/ErrorOrRecoveryHandler.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import WorkspaceUserSettings from '$lib/components/settings/WorkspaceUserSettings.svelte'
	import { WORKSPACE_SHOW_SLACK_CMD, WORKSPACE_SHOW_WEBHOOK_CLI_SYNC } from '$lib/consts'
	import {
		OauthService,
		WorkspaceService,
		JobService,
		ResourceService,
		SettingService,
		type AIConfig
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
	import { emptyString, tryEvery } from '$lib/utils'
	import {
		XCircle,
		RotateCw,
		CheckCircle2,
		X,
		Plus,
		Loader2,
		Save,
		ExternalLink
	} from 'lucide-svelte'

	import PremiumInfo from '$lib/components/settings/PremiumInfo.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	import { fade } from 'svelte/transition'
	import ChangeWorkspaceName from '$lib/components/settings/ChangeWorkspaceName.svelte'
	import ChangeWorkspaceId from '$lib/components/settings/ChangeWorkspaceId.svelte'
	import ChangeWorkspaceColor from '$lib/components/settings/ChangeWorkspaceColor.svelte'
	import {
		convertBackendSettingsToFrontendSettings,
		type S3ResourceSettings
	} from '$lib/workspace_settings'
	import { base } from '$lib/base'
	import { hubPaths } from '$lib/hub'
	import Description from '$lib/components/Description.svelte'
	import ConnectionSection from '$lib/components/ConnectionSection.svelte'
	import AISettings from '$lib/components/workspaceSettings/AISettings.svelte'
	import StorageSettings from '$lib/components/workspaceSettings/StorageSettings.svelte'

	type GitSyncTypeMap = {
		scripts: boolean
		flows: boolean
		apps: boolean
		folders: boolean
		resourceTypes: boolean
		resources: boolean
		variables: boolean
		secrets: boolean
		schedules: boolean
		users: boolean
		groups: boolean
		triggers: boolean
	}
	type GitSyncType =
		| 'script'
		| 'flow'
		| 'app'
		| 'folder'
		| 'resourcetype'
		| 'resource'
		| 'variable'
		| 'secret'
		| 'schedule'
		| 'user'
		| 'group'
		| 'trigger'
	let slackInitialPath: string
	let slackScriptPath: string
	let teamsInitialPath: string
	let teamsScriptPath: string
	let slack_team_name: string | undefined
	let teams_team_id: string | undefined
	let teams_team_name: string | undefined
	let itemKind: 'flow' | 'script' = 'flow'
	let plan: string | undefined = undefined
	let customer_id: string | undefined = undefined
	let webhook: string | undefined = undefined
	let workspaceToDeployTo: string | undefined = undefined
	let errorHandlerSelected: 'custom' | 'slack' | 'teams' = 'slack'
	let errorHandlerInitialScriptPath: string
	let errorHandlerScriptPath: string
	let errorHandlerItemKind: 'flow' | 'script' = 'script'
	let errorHandlerExtraArgs: Record<string, any> = {}
	let errorHandlerMutedOnCancel: boolean | undefined = undefined
	let criticalAlertUIMuted: boolean | undefined = undefined
	let initialCriticalAlertUIMuted: boolean | undefined = undefined

	let aiProviders: Exclude<AIConfig['providers'], undefined> = {}
	let codeCompletionModel: string | undefined = undefined
	let defaultModel: string | undefined = undefined

	let s3ResourceSettings: S3ResourceSettings = {
		resourceType: 's3',
		resourcePath: undefined,
		publicResource: undefined,
		secondaryStorage: undefined
	}
	let gitSyncSettings: {
		include_path: string[]
		repositories: {
			exclude_types_override: GitSyncTypeMap
			script_path: string
			git_repo_resource_path: string
			use_individual_branch: boolean
			group_by_folder: boolean
		}[]
		include_type: GitSyncTypeMap
	}
	let gitSyncTestJobs: {
		jobId: string | undefined
		status: 'running' | 'success' | 'failure' | undefined
	}[]
	let workspaceDefaultAppPath: string | undefined = undefined
	let workspaceEncryptionKey: string | undefined = undefined
	let editedWorkspaceEncryptionKey: string | undefined = undefined
	let workspaceReencryptionInProgress: boolean = false
	let encryptionKeyRegex = /^[a-zA-Z0-9]{64}$/
	let slack_tabs: 'slack_commands' | 'teams_commands' = 'slack_commands'
	let tab =
		($page.url.searchParams.get('tab') as
			| 'users'
			| 'slack'
			| 'premium'
			| 'general'
			| 'webhook'
			| 'deploy_to'
			| 'error_handler') ?? 'users'
	let usingOpenaiClientCredentialsOauth = false

	const latestGitSyncHubScript = hubPaths.gitSync

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
			console.log('editWorkspaceCommand', scriptPath)
			console.log('itemKind', itemKind)
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

	async function editWindmillGitSyncSettings(): Promise<void> {
		let alreadySeenResource: string[] = []
		let repositories = gitSyncSettings.repositories.map((elmt) => {
			alreadySeenResource.push(elmt.git_repo_resource_path)
			let exclude_types_override = gitSyncTypeMapToArray(elmt.exclude_types_override, true)
			return {
				exclude_types_override: exclude_types_override,
				script_path: elmt.script_path,
				git_repo_resource_path: `$res:${elmt.git_repo_resource_path.replace('$res:', '')}`,
				use_individual_branch: elmt.use_individual_branch,
				group_by_folder: elmt.group_by_folder
			}
		})

		let include_path = gitSyncSettings.include_path.filter((elmt) => {
			return !emptyString(elmt)
		})

		let include_type = gitSyncTypeMapToArray(gitSyncSettings.include_type, true)

		if (alreadySeenResource.some((res, index) => alreadySeenResource.indexOf(res) !== index)) {
			sendUserToast('Same Git resource used more than once', true)
			return
		}
		if (repositories.length > 0 || include_path.length > 1 || include_path[0] !== 'f/**') {
			await WorkspaceService.editWorkspaceGitSyncConfig({
				workspace: $workspaceStore!,
				requestBody: {
					git_sync_settings: {
						repositories: repositories,
						include_path: include_path,
						include_type: include_type
					}
				}
			})
			sendUserToast('Workspace Git sync settings updated')
		} else {
			await WorkspaceService.editWorkspaceGitSyncConfig({
				workspace: $workspaceStore!,
				requestBody: {
					git_sync_settings: undefined
				}
			})
			sendUserToast('Workspace Git sync settings reset')
		}
	}

	function gitSyncTypeMapToArray(typesMap: GitSyncTypeMap, expectedValue: boolean): GitSyncType[] {
		let result: GitSyncType[] = []
		if (typesMap.scripts == expectedValue) {
			result.push('script')
		}
		if (typesMap.flows == expectedValue) {
			result.push('flow')
		}
		if (typesMap.apps == expectedValue) {
			result.push('app')
		}
		if (typesMap.folders == expectedValue) {
			result.push('folder')
		}
		if (typesMap.resourceTypes == expectedValue) {
			result.push('resourcetype')
		}
		if (typesMap.resources == expectedValue) {
			result.push('resource')
		}
		if (typesMap.variables == expectedValue) {
			result.push('variable')
		}
		if (typesMap.secrets == expectedValue) {
			result.push('secret')
		}
		if (typesMap.schedules == expectedValue) {
			result.push('schedule')
		}
		if (typesMap.users == expectedValue) {
			result.push('user')
		}
		if (typesMap.groups == expectedValue) {
			result.push('group')
		}
		if (typesMap.triggers == expectedValue) {
			result.push('trigger')
		}
		return result
	}

	function resetGitSyncRepositoryExclude(type: string) {
		gitSyncSettings.repositories.forEach((elmt) => {
			elmt.exclude_types_override[type] = false
		})
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

	let loadedSettings = false
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

		errorHandlerItemKind = settings.error_handler?.split('/')[0] as 'flow' | 'script'
		errorHandlerScriptPath = (settings.error_handler ?? '').split('/').slice(1).join('/')
		errorHandlerInitialScriptPath = errorHandlerScriptPath
		errorHandlerMutedOnCancel = settings.error_handler_muted_on_cancel
		criticalAlertUIMuted = settings.mute_critical_alerts
		initialCriticalAlertUIMuted = settings.mute_critical_alerts
		if (emptyString($enterpriseLicense)) {
			errorHandlerSelected = 'custom'
		} else {
			errorHandlerSelected = emptyString(errorHandlerScriptPath)
				? 'custom'
				: errorHandlerScriptPath.startsWith('hub/') &&
					  errorHandlerScriptPath.endsWith('/workspace-or-schedule-error-handler-slack')
					? 'slack'
					: errorHandlerScriptPath.endsWith('/workspace-or-schedule-error-handler-teams')
						? 'teams'
						: 'custom'
		}
		errorHandlerExtraArgs = settings.error_handler_extra_args ?? {}
		workspaceDefaultAppPath = settings.default_app

		s3ResourceSettings = convertBackendSettingsToFrontendSettings(settings.large_file_storage)

		if (settings.git_sync !== undefined && settings.git_sync !== null) {
			gitSyncTestJobs = []
			gitSyncSettings = {
				include_path:
					(settings.git_sync.include_path?.length ?? 0 > 0)
						? (settings.git_sync.include_path ?? [])
						: ['f/**'],
				repositories: (settings.git_sync.repositories ?? []).map((settings) => {
					gitSyncTestJobs.push({
						jobId: undefined,
						status: undefined
					})
					return {
						git_repo_resource_path: settings.git_repo_resource_path.replace('$res:', ''),
						script_path: settings.script_path,
						use_individual_branch: settings.use_individual_branch ?? false,
						group_by_folder: settings.group_by_folder ?? false,
						exclude_types_override: {
							scripts: (settings.exclude_types_override?.indexOf('script') ?? -1) >= 0,
							flows: (settings.exclude_types_override?.indexOf('flow') ?? -1) >= 0,
							apps: (settings.exclude_types_override?.indexOf('app') ?? -1) >= 0,
							resourceTypes: (settings.exclude_types_override?.indexOf('resourcetype') ?? -1) >= 0,
							resources: (settings.exclude_types_override?.indexOf('resource') ?? -1) >= 0,
							variables: (settings.exclude_types_override?.indexOf('variable') ?? -1) >= 0,
							secrets: (settings.exclude_types_override?.indexOf('secret') ?? -1) >= 0,
							schedules: (settings.exclude_types_override?.indexOf('schedule') ?? -1) >= 0,
							folders: (settings.exclude_types_override?.indexOf('folder') ?? -1) >= 0,
							users: (settings.exclude_types_override?.indexOf('user') ?? -1) >= 0,
							groups: (settings.exclude_types_override?.indexOf('group') ?? -1) >= 0,
							triggers: (settings.exclude_types_override?.indexOf('trigger') ?? -1) >= 0
						}
					}
				}),
				include_type: {
					scripts: (settings.git_sync.include_type?.indexOf('script') ?? -1) >= 0,
					flows: (settings.git_sync.include_type?.indexOf('flow') ?? -1) >= 0,
					apps: (settings.git_sync.include_type?.indexOf('app') ?? -1) >= 0,
					resourceTypes: (settings.git_sync.include_type?.indexOf('resourcetype') ?? -1) >= 0,
					resources: (settings.git_sync.include_type?.indexOf('resource') ?? -1) >= 0,
					variables: (settings.git_sync.include_type?.indexOf('variable') ?? -1) >= 0,
					secrets: (settings.git_sync.include_type?.indexOf('secret') ?? -1) >= 0,
					schedules: (settings.git_sync.include_type?.indexOf('schedule') ?? -1) >= 0,
					folders: (settings.git_sync.include_type?.indexOf('folder') ?? -1) >= 0,
					users: (settings.git_sync.include_type?.indexOf('user') ?? -1) >= 0,
					groups: (settings.git_sync.include_type?.indexOf('group') ?? -1) >= 0,
					triggers: (settings.git_sync.include_type?.indexOf('trigger') ?? -1) >= 0
				}
			}
		} else {
			gitSyncSettings = {
				include_path: ['f/**'],
				repositories: [],
				include_type: {
					scripts: true,
					flows: true,
					apps: true,
					folders: true,
					resourceTypes: false,
					resources: false,
					variables: false,
					secrets: false,
					schedules: false,
					users: false,
					groups: false,
					triggers: false
				}
			}
			gitSyncTestJobs = []
		}
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
	}

	$: $workspaceStore && loadSettings()

	async function editErrorHandler() {
		if (errorHandlerScriptPath) {
			if (errorHandlerScriptPath !== undefined && isSlackHandler(errorHandlerScriptPath)) {
				errorHandlerExtraArgs['slack'] = '$res:f/slack_bot/bot_token'
			} else {
				errorHandlerExtraArgs['slack'] = undefined
			}
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

	function isSlackHandler(scriptPath: string) {
		return (
			scriptPath.startsWith('hub/') &&
			scriptPath.endsWith('/workspace-or-schedule-error-handler-slack')
		)
	}

	async function runGitSyncTestJob(settingsIdx: number) {
		let gitSyncRepository = gitSyncSettings.repositories[settingsIdx]
		if (emptyString(gitSyncRepository.script_path)) {
			return
		}
		let jobId = await JobService.runScriptByPath({
			workspace: $workspaceStore!,
			path: hubPaths.gitSyncTest,
			skipPreprocessor: true,
			requestBody: {
				repo_url_resource_path: gitSyncRepository.git_repo_resource_path.replace('$res:', '')
			}
		})
		gitSyncTestJobs[settingsIdx] = {
			jobId: jobId,
			status: 'running'
		}
		tryEvery({
			tryCode: async () => {
				const testResult = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
					id: jobId
				})
				gitSyncTestJobs[settingsIdx].status = testResult.success ? 'success' : 'failure'
			},
			timeoutCode: async () => {
				try {
					await JobService.cancelQueuedJob({
						workspace: $workspaceStore!,
						id: jobId,
						requestBody: {
							reason: 'Git sync test job timed out after 5s'
						}
					})
				} catch (err) {
					console.error(err)
				}
			},
			interval: 500,
			timeout: 5000
		})
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

	function updateFromSearchTab(searchTab: string | null) {
		if (searchTab && searchTab !== tab) {
			tab = searchTab as typeof tab
		}
	}

	$: updateFromSearchTab($page.url.searchParams.get('tab'))
</script>

<CenteredPage>
	{#if $userStore?.is_admin || $superadmin}
		<PageHeader title="Workspace settings: {$workspaceStore}"
			>{#if $superadmin}
				<Button
					variant="border"
					color="dark"
					size="sm"
					on:click={() => goto('#superadmin-settings')}
				>
					Instance settings
				</Button>
			{/if}</PageHeader
		>

		<div class="overflow-x-auto scrollbar-hidden">
			<Tabs
				bind:selected={tab}
				on:selected={() => {
					// setQueryWithoutLoad($page.url, [{ key: 'tab', value: tab }], 0)
					$page.url.searchParams.set('tab', tab)
					goto(`?${$page.url.searchParams.toString()}`)
				}}
			>
				<Tab
					size="xs"
					value="users"
					aiId="workspace-settings-users"
					aiDescription="Users workspace settings"
				>
					<div class="flex gap-2 items-center my-1"> Users</div>
				</Tab>
				<Tab
					size="xs"
					value="git_sync"
					aiId="workspace-settings-git-sync"
					aiDescription="Git sync workspace settings"
				>
					<div class="flex gap-2 items-center my-1">Git Sync</div>
				</Tab>
				<Tab
					size="xs"
					value="deploy_to"
					aiId="workspace-settings-deploy-to"
					aiDescription="Deployment UI workspace settings"
				>
					<div class="flex gap-2 items-center my-1">Deployment UI</div>
				</Tab>
				{#if WORKSPACE_SHOW_SLACK_CMD}
					<Tab
						size="xs"
						value="slack"
						aiId="workspace-settings-slack"
						aiDescription="Slack / Teams workspace settings"
					>
						<div class="flex gap-2 items-center my-1"> Slack / Teams</div>
					</Tab>
				{/if}
				{#if isCloudHosted()}
					<Tab
						size="xs"
						value="premium"
						aiId="workspace-settings-premium"
						aiDescription="Premium plans workspace settings"
					>
						<div class="flex gap-2 items-center my-1"> Premium Plans </div>
					</Tab>
				{/if}
				{#if WORKSPACE_SHOW_WEBHOOK_CLI_SYNC}
					<Tab
						size="xs"
						value="webhook"
						aiId="workspace-settings-webhook"
						aiDescription="Webhook workspace settings"
					>
						<div class="flex gap-2 items-center my-1">Webhook</div>
					</Tab>
				{/if}
				<Tab
					size="xs"
					value="error_handler"
					aiId="workspace-settings-error-handler"
					aiDescription="Error handler workspace settings"
				>
					<div class="flex gap-2 items-center my-1">Error Handler</div>
				</Tab>
				<Tab
					size="xs"
					value="ai"
					aiId="workspace-settings-ai"
					aiDescription="Windmill AI workspace settings"
				>
					<div class="flex gap-2 items-center my-1">Windmill AI</div>
				</Tab>
				<Tab
					size="xs"
					value="windmill_lfs"
					aiId="workspace-settings-windmill-lfs"
					aiDescription="Object Storage (S3) workspace settings"
				>
					<div class="flex gap-2 items-center my-1"> Object Storage (S3)</div>
				</Tab>
				<Tab
					size="xs"
					value="default_app"
					aiId="workspace-settings-default-app"
					aiDescription="Default app workspace settings"
				>
					<div class="flex gap-2 items-center my-1"> Default App </div>
				</Tab>
				<Tab
					size="xs"
					value="encryption"
					aiId="workspace-settings-encryption"
					aiDescription="Encryption workspace settings"
				>
					<div class="flex gap-2 items-center my-1"> Encryption </div>
				</Tab>
				<Tab
					size="xs"
					value="general"
					aiId="workspace-settings-general"
					aiDescription="General workspace settings"
				>
					<div class="flex gap-2 items-center my-1"> General </div>
				</Tab>
			</Tabs>
		</div>
		{#if !loadedSettings}
			<Skeleton layout={[1, [40]]} />
		{:else if tab == 'users'}
			<WorkspaceUserSettings />
		{:else if tab == 'deploy_to'}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-primary text-lg font-semibold">
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
					><Alert type="error" title="Enterprise license required"
						>Deploy to staging/prod from the web UI is only available with an enterprise license</Alert
					></div
				>
			{/if}
		{:else if tab == 'premium'}
			<PremiumInfo {customer_id} {plan} />
		{:else if tab == 'slack'}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-primary text-lg font-semibold"
						>Workspace connections to Slack and Teams</div
					>
					<Description link="https://www.windmill.dev/docs/integrations/slack">
						With workspace connections, you can trigger scripts or flows with a '/windmill' command
						with your Slack or Teams bot.
					</Description>
				</div>

				<Tabs bind:selected={slack_tabs}>
					<Tab size="xs" value="slack_commands">
						<div class="flex gap-2 items-center my-1"> Slack</div>
					</Tab>
					<Tab size="xs" value="teams_commands">
						<div class="flex gap-2 items-center my-1"> Teams</div>
					</Tab>
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
						createScriptHref="{base}/scripts/add?hub=hub%2F314%2Fslack%2Fexample_of_responding_to_a_slack_command_slack"
						createFlowHref="{base}/flows/add?hub=28"
						documentationLink="https://www.windmill.dev/docs/integrations/slack"
						onLoadSettings={loadSettings}
						display_name={slack_team_name}
					/>
				{:else if slack_tabs === 'teams_commands'}
					{#if !$enterpriseLicense}
						<div class="pt-4"></div>
						<Alert type="info" title="Workspace Teams commands is an EE feature">
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
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class=" text-primary text-lg font-semibold">General</div>
					<Description link="https://www.windmill.dev/docs/core_concepts/workspace_settings">
						Configure general workspace settings.
					</Description>
				</div>
			</div>

			<div class="flex flex-col gap-10">
				<ChangeWorkspaceName />
				<ChangeWorkspaceId />
				<ChangeWorkspaceColor />
			</div>

			<PageHeader title="Export workspace" primary={false} />
			<div class="flex justify-start">
				<Button
					size="sm"
					href="{base}/api/w/{$workspaceStore ?? ''}/workspaces/tarball?archive_type=zip"
					target="_blank"
				>
					Export workspace as zip file
				</Button>
			</div>

			<div class="mt-20"></div>
			<PageHeader title="Delete workspace" primary={false} />
			{#if !$superadmin}
				<p class="italic text-xs"> Only instance superadmins can delete a workspace. </p>
			{/if}
			{#if $workspaceStore === 'admins' || $workspaceStore === 'starter'}
				<p class="italic text-xs">
					This workspace cannot be deleted as it has a special function. Consult the documentation
					for more information.
				</p>
			{/if}
			<div class="flex gap-2">
				<Button
					color="red"
					disabled={$workspaceStore === 'admins' || $workspaceStore === 'starter'}
					size="sm"
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
					<div class=" text-primary text-lg font-semibold"> Workspace Webhook</div>
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
					<div class=" text-primary text-base font-semibold"> URL to send requests to</div>
					<div class="text-tertiary text-xs">
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
				<Alert type="info" title="Workspace error handler is an EE feature">
					Workspace error handler is a Windmill EE feature. It enables using your current Slack
					connection or a custom script to send notifications anytime any job would fail.
				</Alert>
				<div class="pb-2"></div>
			{/if}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-primary text-lg font-semibold"> Workspace Error Handler</div>
					<Description
						link="https://www.windmill.dev/docs/core_concepts/error_handling#workspace-error-handler"
					>
						Define a script or flow to be executed automatically in case of error in the workspace.
					</Description>
				</div>
			</div>
			<div class="flex flex-col gap-4 my-4">
				<div class="flex flex-col gap-1">
					<div class="text-primary text-base font-semibold">
						Script or flow to run as error handler</div
					>
				</div>
			</div>
			<ErrorOrRecoveryHandler
				isEditable={true}
				errorOrRecovery="error"
				showScriptHelpText={true}
				customInitialScriptPath={errorHandlerInitialScriptPath}
				bind:handlerSelected={errorHandlerSelected}
				bind:handlerPath={errorHandlerScriptPath}
				customScriptTemplate="/scripts/add?hub=hub%2F9083%2Fwindmill%2Fworkspace_error_handler_template"
				bind:customHandlerKind={errorHandlerItemKind}
				bind:handlerExtraArgs={errorHandlerExtraArgs}
			>
				<svelte:fragment slot="custom-tab-tooltip">
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
				</svelte:fragment>
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
					<div class="text-primary text-lg font-semibold"> Workspace Critical Alerts</div>
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
				bind:usingOpenaiClientCredentialsOauth
			/>
		{:else if tab == 'windmill_lfs'}
			<StorageSettings bind:s3ResourceSettings />
		{:else if tab == 'git_sync'}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-primary text-lg font-semibold">Git Sync</div>
					<Description link="https://www.windmill.dev/docs/advanced/git_sync">
						Connect the Windmill workspace to a Git repository to automatically commit and push
						scripts, flows, and apps to the repository on each deploy.
					</Description>
				</div>
			</div>
			{#if !$enterpriseLicense}
				<div class="mb-2"></div>

				<Alert type="warning" title="Syncing workspace to Git is an EE feature">
					Automatically saving scripts to a Git repository on each deploy is a Windmill EE feature.
				</Alert>
				<div class="mb-2"></div>
			{/if}
			{#if gitSyncSettings != undefined}
				<div class="flex mt-5 mb-5 gap-8">
					<Button
						color="blue"
						disabled={!$enterpriseLicense ||
							gitSyncSettings?.repositories?.some((elmt) =>
								emptyString(elmt.git_repo_resource_path)
							)}
						on:click={() => {
							editWindmillGitSyncSettings()
							console.log('Saving git sync settings', gitSyncSettings)
						}}>Save git sync settings {!$enterpriseLicense ? '(ee only)' : ''}</Button
					>

					<Button
						color="dark"
						target="_blank"
						endIcon={{ icon: ExternalLink }}
						href={`/runs?job_kinds=deploymentcallbacks&workspace=${$workspaceStore}`}
						>See sync jobs</Button
					>
				</div>

				<div class="flex flex-wrap gap-20">
					<div class="max-w-md w-full">
						{#if Array.isArray(gitSyncSettings?.include_path)}
							<h4 class="flex gap-2 mb-4"
								>Path filters<Tooltip>
									Only scripts, flows and apps with their path matching one of those filters will be
									synced to the Git repositories below. The filters allow '*'' and '**' characters,
									with '*'' matching any character allowed in paths until the next slash (/) and
									'**' matching anything including slashes.
									<br />By default everything in folders will be synced.
								</Tooltip></h4
							>
							{#each gitSyncSettings.include_path ?? [] as gitSyncRegexpPath, idx}
								<div class="flex mt-1 items-center">
									<input type="text" bind:value={gitSyncRegexpPath} id="arg-input-array" />
									<button
										transition:fade|local={{ duration: 100 }}
										class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover ml-2"
										aria-label="Clear"
										on:click={() => {
											gitSyncSettings.include_path.splice(idx, 1)
											gitSyncSettings.include_path = [...gitSyncSettings.include_path]
										}}
									>
										<X size={14} />
									</button>
								</div>
							{/each}
						{/if}
						<div class="flex mt-2">
							<Button
								variant="border"
								color="light"
								size="xs"
								btnClasses="mt-1"
								on:click={() => {
									gitSyncSettings.include_path = [...gitSyncSettings.include_path, '']
								}}
								id="git-sync-add-path-filter"
								startIcon={{ icon: Plus }}
							>
								Add filter
							</Button>
						</div>
						<div class="pt-2"></div>
						<Alert type="info" title="Only new updates trigger git sync">
							Only new changes matching the filters will trigger a git sync. You still need to
							initalize the repo to the desired state first.
						</Alert>
					</div>

					<div class="max-w-md w-full">
						<h4 class="flex gap-2 mb-4"
							>Type filters<Tooltip>
								On top of the filter path above, you can include only certain type of object to be
								synced with the Git repository.
								<br />By default everything is synced.
							</Tooltip></h4
						>
						<div class="flex flex-col gap-2 mt-1">
							<Toggle
								bind:checked={gitSyncSettings.include_type.scripts}
								on:change={(_) => resetGitSyncRepositoryExclude('scripts')}
								options={{ right: 'Scripts' }}
							/>
							<Toggle
								bind:checked={gitSyncSettings.include_type.flows}
								on:change={(_) => resetGitSyncRepositoryExclude('flows')}
								options={{ right: 'Flows' }}
							/>
							<Toggle
								bind:checked={gitSyncSettings.include_type.apps}
								on:change={(_) => resetGitSyncRepositoryExclude('apps')}
								options={{ right: 'Apps' }}
							/>
							<Toggle
								bind:checked={gitSyncSettings.include_type.folders}
								on:change={(_) => resetGitSyncRepositoryExclude('folders')}
								options={{ right: 'Folders' }}
							/>
							<Toggle
								bind:checked={gitSyncSettings.include_type.resources}
								on:change={(_) => resetGitSyncRepositoryExclude('resources')}
								options={{ right: 'Resources' }}
							/>
							<div class="flex gap-3">
								<Toggle
									bind:checked={gitSyncSettings.include_type.variables}
									on:change={(ev) => {
										resetGitSyncRepositoryExclude('variables')
										resetGitSyncRepositoryExclude('secrets')
										if (!ev.detail) {
											gitSyncSettings.include_type.secrets = false
										}
									}}
									options={{ right: 'Variables ' }}
								/>
								<span>-</span>
								<Toggle
									disabled={!gitSyncSettings.include_type.variables}
									bind:checked={gitSyncSettings.include_type.secrets}
									on:change={(_) => resetGitSyncRepositoryExclude('secrets')}
									options={{ left: 'Include secrets' }}
								/>
							</div>
							<Toggle
								bind:checked={gitSyncSettings.include_type.schedules}
								on:change={(_) => resetGitSyncRepositoryExclude('schedules')}
								options={{ right: 'Schedules' }}
							/>
							<Toggle
								bind:checked={gitSyncSettings.include_type.resourceTypes}
								on:change={(_) => resetGitSyncRepositoryExclude('resourcetypes')}
								options={{ right: 'Resource Types' }}
							/>
							<Toggle
								bind:checked={gitSyncSettings.include_type.users}
								on:change={(_) => resetGitSyncRepositoryExclude('users')}
								options={{ right: 'Users' }}
							/>
							<Toggle
								bind:checked={gitSyncSettings.include_type.groups}
								on:change={(_) => resetGitSyncRepositoryExclude('groups')}
								options={{ right: 'Groups' }}
							/>
							<Toggle
								bind:checked={gitSyncSettings.include_type.triggers}
								on:change={(_) => resetGitSyncRepositoryExclude('triggers')}
								options={{ right: 'Triggers' }}
							/>
						</div>
					</div>
				</div>

				<h4 class="flex gap-2 mt-5 mb-5"
					>Repositories to sync<Tooltip>
						The changes will be deployed to all the repositories set below.
					</Tooltip></h4
				>
				{#if Array.isArray(gitSyncSettings.repositories)}
					{#each gitSyncSettings.repositories as gitSyncRepository, idx}
						<div class="flex mt-5 mb-1 gap-1 items-center text-xs">
							<h6>Repository #{idx + 1}</h6>
							<button
								transition:fade|local={{ duration: 100 }}
								class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover ml-2"
								aria-label="Clear"
								on:click={() => {
									gitSyncSettings.repositories.splice(idx, 1)
									gitSyncSettings.repositories = [...gitSyncSettings.repositories]
								}}
							>
								<X size={14} />
							</button>
						</div>
						<div class="flex mt-5 mb-1 gap-1">
							{#key gitSyncRepository}
								<ResourcePicker
									resourceType="git_repository"
									initialValue={gitSyncRepository.git_repo_resource_path}
									on:change={(ev) => {
										gitSyncRepository.git_repo_resource_path = ev.detail
									}}
								/>
								<Button
									disabled={emptyString(gitSyncRepository.script_path)}
									btnClasses="w-32 text-center"
									color="dark"
									on:click={() => runGitSyncTestJob(idx)}
									size="xs">Test connection</Button
								>
							{/key}
						</div>

						<div class="flex mb-5 text-normal text-2xs gap-1">
							{#if gitSyncSettings.repositories.filter((settings) => settings.git_repo_resource_path === gitSyncRepository.git_repo_resource_path).length > 1}
								<span class="text-red-700">Using the same resource twice is not allowed.</span>
							{/if}
							{#if gitSyncTestJobs[idx].status !== undefined}
								{#if gitSyncTestJobs[idx].status === 'running'}
									<RotateCw size={14} />
								{:else if gitSyncTestJobs[idx].status === 'success'}
									<CheckCircle2 size={14} class="text-green-600" />
								{:else}
									<XCircle size={14} class="text-red-700" />
								{/if}
								Git sync resource checked via Windmill job
								<a
									target="_blank"
									href={`/run/${gitSyncTestJobs[idx].jobId}?workspace=${$workspaceStore}`}
								>
									{gitSyncTestJobs[idx].jobId}
								</a>WARNING: Only read permissions are verified.
							{/if}
						</div>

						<div class="flex flex-col mt-5 mb-1 gap-4">
							{#if gitSyncSettings && gitSyncRepository}
								{#if gitSyncRepository.script_path != latestGitSyncHubScript}
									<Alert type="warning" title="Script version mismatch">
										The git sync version for this repository is not latest. Current: <a
											target="_blank"
											href="https://hub.windmill.dev/scripts/windmill/6943/sync-script-to-git-repo-windmill/9014/versions"
											>{gitSyncRepository.script_path}</a
										>, latest:
										<a
											target="_blank"
											href="https://hub.windmill.dev/scripts/windmill/6943/sync-script-to-git-repo-windmill/9014/versions"
											>{latestGitSyncHubScript}</a
										>
										<div class="flex mt-2">
											<Button
												size="xs"
												color="dark"
												on:click={() => {
													gitSyncRepository.script_path = latestGitSyncHubScript
												}}>Update git sync script (require save git settings to be applied)</Button
											>
										</div>
									</Alert>
								{/if}
								<Toggle
									disabled={emptyString(gitSyncRepository.git_repo_resource_path)}
									bind:checked={gitSyncRepository.use_individual_branch}
									options={{
										right: 'Create one branch per deployed object',
										rightTooltip:
											"If set, Windmill will create a unique branch per object being pushed based on its path, prefixed with 'wm_deploy/'."
									}}
								/>

								<Toggle
									disabled={emptyString(gitSyncRepository.git_repo_resource_path) ||
										!gitSyncRepository.use_individual_branch}
									bind:checked={gitSyncRepository.group_by_folder}
									options={{
										right: 'Group deployed objects by folder',
										rightTooltip:
											'Instead of creating a branch per object, Windmill will create a branch per folder containing objects being deployed.'
									}}
								/>
							{/if}
						</div>

						<div class="flex flex-col mt-5 mb-1 gap-1">
							{#if gitSyncSettings && Object.keys(gitSyncSettings.include_type).some((k) => gitSyncSettings.include_type[k] === true)}
								<h6>Exclude specific types for this repository only</h6>
								{#if gitSyncSettings.include_type.scripts}
									<Toggle
										color="red"
										bind:checked={gitSyncRepository.exclude_types_override.scripts}
										options={{ right: 'Exclude scripts' }}
									/>
								{/if}
								{#if gitSyncSettings.include_type.flows}
									<Toggle
										color="red"
										bind:checked={gitSyncRepository.exclude_types_override.flows}
										options={{ right: 'Exclude flows' }}
									/>
								{/if}
								{#if gitSyncSettings.include_type.apps}
									<Toggle
										color="red"
										bind:checked={gitSyncRepository.exclude_types_override.apps}
										options={{ right: 'Exclude apps' }}
									/>
								{/if}
								{#if gitSyncSettings.include_type.folders}
									<Toggle
										color="red"
										bind:checked={gitSyncRepository.exclude_types_override.folders}
										options={{ right: 'Exclude folders' }}
									/>
								{/if}
								{#if gitSyncSettings.include_type.resources}
									<Toggle
										color="red"
										bind:checked={gitSyncRepository.exclude_types_override.resources}
										options={{ right: 'Exclude resources' }}
									/>
								{/if}
								{#if gitSyncSettings.include_type.variables}
									<div class="flex gap-3">
										<Toggle
											color="red"
											bind:checked={gitSyncRepository.exclude_types_override.variables}
											on:change={(ev) => {
												if (ev.detail && gitSyncSettings.include_type.secrets) {
													gitSyncRepository.exclude_types_override.secrets = true
												} else if (ev.detail) {
													gitSyncRepository.exclude_types_override.secrets = false
												}
											}}
											options={{ right: 'Exclude variables ' }}
										/>
										{#if gitSyncSettings.include_type.secrets}
											<span>-</span>
											<Toggle
												color="red"
												disabled={gitSyncRepository.exclude_types_override.variables}
												bind:checked={gitSyncRepository.exclude_types_override.secrets}
												options={{ left: 'Exclude secrets' }}
											/>
										{/if}
									</div>
								{/if}
								{#if gitSyncSettings.include_type.schedules}
									<Toggle
										color="red"
										bind:checked={gitSyncRepository.exclude_types_override.schedules}
										options={{ right: 'Exclude schedules' }}
									/>
								{/if}
								{#if gitSyncSettings.include_type.resourceTypes}
									<Toggle
										color="red"
										bind:checked={gitSyncRepository.exclude_types_override.resourceTypes}
										options={{ right: 'Exclude resource types' }}
									/>
								{/if}
								{#if gitSyncSettings.include_type.triggers}
									<Toggle
										color="red"
										bind:checked={gitSyncRepository.exclude_types_override.triggers}
										options={{ right: 'Exclude triggers' }}
									/>
								{/if}
							{/if}
						</div>
					{/each}
				{/if}

				<div class="flex mt-5 mb-5 gap-1">
					<Button
						color="none"
						variant="border"
						size="xs"
						btnClasses="mt-1"
						on:click={() => {
							gitSyncSettings.repositories = [
								...gitSyncSettings.repositories,
								{
									script_path: latestGitSyncHubScript,
									git_repo_resource_path: '',
									use_individual_branch: false,
									group_by_folder: false,
									exclude_types_override: {
										scripts: false,
										flows: false,
										apps: false,
										folders: false,
										resourceTypes: false,
										resources: false,
										variables: false,
										secrets: false,
										schedules: false,
										users: false,
										groups: false,
										triggers: false
									}
								}
							]
							gitSyncTestJobs = [
								...gitSyncTestJobs,
								{
									jobId: undefined,
									status: undefined
								}
							]
						}}
						id="git-sync-add-connection"
						startIcon={{ icon: Plus }}
					>
						Add connection
					</Button>
				</div>

				<div class="bg-surface-disabled p-4 rounded-md flex flex-col gap-1">
					<div class="text-primary font-md font-semibold"> Git repository initial setup </div>

					<div class="prose max-w-none text-2xs text-tertiary">
						Every time a script is deployed, only the updated script will be pushed to the remote
						Git repository.

						<br />

						For the git repo to be representative of the entire workspace, it is recommended to set
						it up using the Windmill CLI before turning this option on.

						<br /><br />

						Not familiar with Windmill CLI?
						<a href="https://www.windmill.dev/docs/advanced/cli" class="text-primary"
							>Check out the docs</a
						>

						<br /><br />

						Run the following commands from the git repo folder to push the initial workspace
						content to the remote:

						<br />

						<pre class="overflow-auto max-h-screen"
							><code
								>npm install -g windmill-cli
wmill workspace add  {$workspaceStore} {$workspaceStore} {`${$page.url.protocol}//${$page.url.hostname}/`}
wmill init
# adjust wmill.yaml file configuraton as needed
wmill sync pull
git add -A
git commit -m 'Initial commit'
git push</code
							></pre
						>
					</div>
				</div>
			{:else}
				<Loader2 class="animate-spin mt-4" size={20} />
			{/if}
		{:else if tab == 'default_app'}
			<div class="flex flex-col gap-4 my-8">
				<div class="flex flex-col gap-1">
					<div class="text-primary text-lg font-semibold">Workspace Default App</div>
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
				<Alert type="info" title="Windmill EE only feature">
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
					<div class="text-primary text-lg font-semibold">Workspace Secret Encryption</div>
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
			<h6> Workspace encryption key </h6>
			<div class="flex gap-2 mt-1">
				<input
					class="justify-start"
					type="text"
					placeholder={'*'.repeat(64)}
					bind:value={editedWorkspaceEncryptionKey}
				/>
				<Button
					color="light"
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

<style>
</style>
