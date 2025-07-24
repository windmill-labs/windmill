<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import { isCloudHosted } from '$lib/cloud'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Button, Skeleton, Tab, Tabs } from '$lib/components/common'
	import { deepEqual } from 'fast-equals'

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
		RotateCcw,
		CheckCircle2,
		Trash,
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
	import InitGitRepoPopover from '$lib/components/InitGitRepoPopover.svelte'
	import PullGitRepoPopover from '$lib/components/PullGitRepoPopover.svelte'
	import GitSyncFilterSettings from '$lib/components/workspaceSettings/GitSyncFilterSettings.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { untrack } from 'svelte'

	// Shared defaults for new Git-Sync repositories
	const DEFAULT_INCLUDE_PATH = ['f/**'] as const;
	const DEFAULT_EXCLUDE_PATH: string[] = [];
	const DEFAULT_EXTRA_INCLUDE_PATH: string[] = [];

	type ObjectType =
		| 'script'
		| 'flow'
		| 'app'
		| 'folder'
		| 'resource'
		| 'variable'
		| 'secret'
		| 'resourcetype'
		| 'schedule'
		| 'user'
		| 'group'
		| 'trigger'
		| 'settings'
		| 'key'

	type GitSyncSettings = {
		repositories: GitSyncRepository[]
	}

	type GitRepositorySettings = {
		include_path: string[]
		exclude_path: string[]
		extra_include_path: string[]
		include_type: ObjectType[]
	}

	// Import the generated backend type
	import type { GitRepositorySettings as BackendGitRepositorySettings } from '$lib/gen'
	import Label from '$lib/components/Label.svelte'

	// Frontend repository format extends backend with guaranteed settings and additional UI state
	type GitSyncRepository = BackendGitRepositorySettings & {
		settings: GitRepositorySettings  // Required in frontend after transformation
		legacyImported?: boolean
	}

	// Workspace-level legacy filter arrays (populated if we imported legacy settings)
	let legacyWorkspaceIncludePath = $state<string[]>([])
	let legacyWorkspaceIncludeType = $state<ObjectType[]>([])

	let slackInitialPath: string = $state('')
	let slackScriptPath: string = $state('')
	let teamsInitialPath: string = $state('')
	let teamsScriptPath: string = $state('')
	let slack_team_name: string | undefined = $state()
	let teams_team_id: string | undefined = $state()
	let teams_team_name: string | undefined = $state()
	let itemKind: 'flow' | 'script' = $state('flow')
	let plan: string | undefined = $state(undefined)
	let customer_id: string | undefined = $state(undefined)
	let webhook: string | undefined = $state(undefined)
	let workspaceToDeployTo: string | undefined = $state(undefined)
	let errorHandlerSelected: 'custom' | 'slack' | 'teams' = $state('slack')
	let errorHandlerScriptPath: string | undefined = $state(undefined)
	let errorHandlerItemKind: 'flow' | 'script' = $state('script')
	let errorHandlerExtraArgs: Record<string, any> = $state({})
	let errorHandlerMutedOnCancel: boolean | undefined = $state(undefined)
	let criticalAlertUIMuted: boolean | undefined = $state(undefined)
	let initialCriticalAlertUIMuted: boolean | undefined = $state(undefined)
	let triggerFailureEmailRecipients: string[] = $state([])
	let initialTriggerFailureEmailRecipients: string[] = $state([])

	let aiProviders: Exclude<AIConfig['providers'], undefined> = $state({})
	let codeCompletionModel: string | undefined = $state(undefined)
	let defaultModel: string | undefined = $state(undefined)

	let s3ResourceSettings: S3ResourceSettings = $state({
		resourceType: 's3',
		resourcePath: undefined,
		publicResource: undefined,
		secondaryStorage: undefined
	})

	let gitSyncSettings = $state<GitSyncSettings>({
		repositories: []
	})

	let gitSyncTestJobs = $state<
		{
			jobId: string | undefined
			status: 'running' | 'success' | 'failure' | undefined
		}[]
	>([])

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
			| 'error_handler') ?? 'users'
	)
	let usingOpenaiClientCredentialsOauth = $state(false)

	let yamlText = $state('')
	let initialGitSyncSettings = $state<GitSyncSettings | undefined>(undefined)
	let loadedSettings = $state(false)

	// Reactive trigger to ensure UI updates when repository data changes
	let repoReactivityTrigger = $state(0)



	const latestGitSyncHubScript = hubPaths.gitSync

	// Each repository may have been populated from workspace-level legacy settings. Track on the repo itself.
	const anyLegacyImported = $derived(gitSyncSettings.repositories.some((r) => r.legacyImported))

	// Track changes in repositories
	const repoChanges = $derived(
		(() => {
			// Force reactivity check by accessing the trigger
			repoReactivityTrigger;

			return gitSyncSettings.repositories.map((repo, idx) => {
				const repoValid = isRepoValid(idx)

				// If there were no initial repos, treat each repo as changed only when valid
				if (!initialGitSyncSettings || !initialGitSyncSettings.repositories || initialGitSyncSettings.repositories.length === 0) {
					return repoValid
				}

				const initial = initialGitSyncSettings.repositories.find(
					initialRepo => initialRepo.git_repo_resource_path === repo.git_repo_resource_path
				)

				// If no matching initial repo found, this is a new repo - changed only when valid
				if (!initial) return repoValid

				// Handle array ordering for consistent comparison
				const settings1 = {
					include_path: [...(initial.settings?.include_path ?? [])].sort(),
					exclude_path: [...(initial.settings?.exclude_path ?? [])].sort(),
					extra_include_path: [...(initial.settings?.extra_include_path ?? [])].sort(),
					include_type: [...(initial.settings?.include_type ?? [])].sort()
				};

				const settings2 = {
					include_path: [...(repo.settings?.include_path ?? [])].sort(),
					exclude_path: [...(repo.settings?.exclude_path ?? [])].sort(),
					extra_include_path: [...(repo.settings?.extra_include_path ?? [])].sort(),
					include_type: [...(repo.settings?.include_type ?? [])].sort()
				};

				// Compare all properties in a consistent way
				const isChanged = !deepEqual(
					{
						settings: settings1,
						use_individual_branch: initial.use_individual_branch,
						group_by_folder: initial.group_by_folder,
						script_path: initial.script_path,
						git_repo_resource_path: initial.git_repo_resource_path,
						exclude_types_override: [...(initial.exclude_types_override ?? [])].sort()
					},
					{
						settings: settings2,
						use_individual_branch: repo.use_individual_branch,
						group_by_folder: repo.group_by_folder,
						script_path: repo.script_path,
						git_repo_resource_path: repo.git_repo_resource_path,
						exclude_types_override: [...(repo.exclude_types_override ?? [])].sort()
					}
				)

				return isChanged && repoValid
			})
		})()
	)

	const hasAnyChanges = $derived(
		repoChanges.some(Boolean) ||
		anyLegacyImported ||
		// Check if the set of valid repos has changed (added/removed repos)
		(() => {
			if (!initialGitSyncSettings?.repositories) return gitSyncSettings.repositories.filter((_,i)=>isRepoValid(i)).length > 0

			const initialValidPaths = new Set(
				initialGitSyncSettings.repositories
					.filter(r => !emptyString(r.git_repo_resource_path))
					.map(r => r.git_repo_resource_path)
			)
			const currentValidPaths = new Set(
				gitSyncSettings.repositories
					.filter((_,i) => isRepoValid(i))
					.map(r => r.git_repo_resource_path)
			)

			// Check if sets are different (repos added or removed)
			return initialValidPaths.size !== currentValidPaths.size ||
				   [...initialValidPaths].some(path => !currentValidPaths.has(path)) ||
				   [...currentValidPaths].some(path => !initialValidPaths.has(path))
		})()
	)

	// Helper that tells if a repo card is valid (resource selected and not duplicated)
	function isRepoValid(idx: number): boolean {
		const repo = gitSyncSettings.repositories[idx]
		if (!repo) return false
		if (emptyString(repo.git_repo_resource_path)) return false
		return !isRepoDuplicate(idx)
	}

	// Helper: true if repo shares its resource with an earlier repo
	function isRepoDuplicate(idx: number): boolean {
		const repo = gitSyncSettings.repositories[idx]
		if (!repo || emptyString(repo.git_repo_resource_path)) return false
		const firstIdx = gitSyncSettings.repositories.findIndex(r => r.git_repo_resource_path === repo.git_repo_resource_path)
		return firstIdx !== idx
	}

	function serializeRepo(repo: GitSyncRepository) {
		const serialized: any = {
			script_path: repo.script_path,
			git_repo_resource_path: `$res:${repo.git_repo_resource_path.replace('$res:', '')}`,
			use_individual_branch: repo.use_individual_branch,
			group_by_folder: repo.group_by_folder,
			settings: repo.settings
		}

		// exclude_types_override should never be included for migrated repos (only legacy repos have it)
		// Migration removes excluded types from include_type and drops exclude_types_override

		return serialized
	}

	async function saveRepoSettings(idx: number): Promise<void> {
		const currentRepo = gitSyncSettings.repositories[idx]
		if (!currentRepo || !isRepoValid(idx)) {
			sendUserToast('Cannot save invalid repository (missing or duplicate resource)', true)
			return
		}

		// If we started with empty settings, we need to save all valid repositories
		if (!initialGitSyncSettings || !initialGitSyncSettings.repositories || initialGitSyncSettings.repositories.length === 0) {
			// For new repositories starting from empty, save all current repositories with valid resources
			const validRepositories = gitSyncSettings.repositories
				.filter((_,i)=>isRepoValid(i))
				.map(repo => serializeRepo(repo))

			await WorkspaceService.editWorkspaceGitSyncConfig({
				workspace: $workspaceStore!,
				requestBody: {
					git_sync_settings: { repositories: validRepositories }
				}
			})

			// Mark all repos migrated and reset legacy arrays
			gitSyncSettings.repositories.forEach(r => r.legacyImported = false)
			legacyWorkspaceIncludePath = []
			legacyWorkspaceIncludeType = []
			// Update initial settings to reflect what we just saved
			initialGitSyncSettings = JSON.parse(JSON.stringify(gitSyncSettings))
		} else {
			// Build repositories array: include all repos but serialize differently based on migration status
			let repositories: any[] = []

			// Process all repos that should be in the final payload
			for (const repo of gitSyncSettings.repositories) {
				if (repo === currentRepo) {
					// This is the repo we're saving - migrate if legacy, otherwise serialize normally
					if (currentRepo.legacyImported) {
						// Migrate legacy repo: remove excluded types from include_type and drop exclude_types_override
						const migratedRepo = {
							...currentRepo,
							settings: {
								...currentRepo.settings,
								include_type: currentRepo.settings.include_type.filter(
									type => !currentRepo.exclude_types_override?.includes(type)
								)
							},
							exclude_types_override: [], // Clear this for migrated repo
							legacyImported: false
						}
						repositories.push(serializeRepo(migratedRepo))
						// Update the current repo in the UI to reflect migration
						Object.assign(currentRepo, migratedRepo)
					} else {
						repositories.push(serializeRepo(currentRepo))
					}
				} else if (repo.legacyImported) {
					// This is a legacy repo - serialize without settings field at all
					const legacyRepoData: any = {
						script_path: repo.script_path,
						git_repo_resource_path: `$res:${repo.git_repo_resource_path.replace('$res:', '')}`,
						use_individual_branch: repo.use_individual_branch,
						group_by_folder: repo.group_by_folder
					}
					// Include exclude_types_override if it has values
					if (repo.exclude_types_override && repo.exclude_types_override.length > 0) {
						legacyRepoData.exclude_types_override = repo.exclude_types_override
					}
					repositories.push(legacyRepoData)
				} else {
					// This is an already-migrated repo
					repositories.push(serializeRepo(repo))
				}
			}

			// Mark current repo as migrated
			currentRepo.legacyImported = false;

			// Check if there are still legacy repos
			const remainingLegacy = gitSyncSettings.repositories.some(r => r.legacyImported)

			const gitSyncPayload: any = {
				git_sync_settings: {
					repositories,
					...(remainingLegacy && {
						include_path: legacyWorkspaceIncludePath,
						include_type: legacyWorkspaceIncludeType
					})
				}
			}

			console.log('Sending payload:', JSON.stringify(gitSyncPayload, null, 2))

			await WorkspaceService.editWorkspaceGitSyncConfig({
				workspace: $workspaceStore!,
				requestBody: gitSyncPayload
			})

			// Update initial settings to reflect current state
			initialGitSyncSettings = JSON.parse(JSON.stringify(gitSyncSettings))

			// if no more legacy repos, clear workspace-level legacy arrays
			if (!remainingLegacy) {
				legacyWorkspaceIncludePath = []
				legacyWorkspaceIncludeType = []
			}
		}

		sendUserToast('Repository settings updated')
	}

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

	async function editWindmillGitSyncSettings(): Promise<void> {
		// Filter out repositories with empty resource paths before processing
		const validRepos = gitSyncSettings.repositories.filter((_,i)=>isRepoValid(i))

		let alreadySeenResource: string[] = []
		let repositories = validRepos.map((repo) => {
			alreadySeenResource.push(repo.git_repo_resource_path)

			return serializeRepo(repo)
		})

		if (alreadySeenResource.some((res, index) => alreadySeenResource.indexOf(res) !== index)) {
			sendUserToast('Same Git resource used more than once', true)
			return
		}

		if (repositories.length > 0) {
			await WorkspaceService.editWorkspaceGitSyncConfig({
				workspace: $workspaceStore!,
				requestBody: {
					git_sync_settings: {
						repositories
					}
				}
			})
			// Update initial settings to reflect what we just saved
			initialGitSyncSettings = {
				repositories: gitSyncSettings.repositories.filter((_,i)=>isRepoValid(i))
			}
			sendUserToast('Workspace Git sync settings updated')
			gitSyncSettings.repositories.forEach(r => r.legacyImported = false);
			legacyWorkspaceIncludePath = [];
			legacyWorkspaceIncludeType = [];
		} else {
			await WorkspaceService.editWorkspaceGitSyncConfig({
				workspace: $workspaceStore!,
				requestBody: {
					git_sync_settings: { repositories: [] }
				}
			})
			initialGitSyncSettings = { repositories: [] }
			sendUserToast('Workspace Git sync settings updated (no repositories)')
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

		errorHandlerItemKind = settings.error_handler
			? (settings.error_handler.split('/')[0] as 'flow' | 'script')
			: 'script'
		errorHandlerScriptPath = (settings.error_handler ?? '').split('/').slice(1).join('/')
		errorHandlerMutedOnCancel = settings.error_handler_muted_on_cancel
		criticalAlertUIMuted = settings.mute_critical_alerts
		initialCriticalAlertUIMuted = settings.mute_critical_alerts
		triggerFailureEmailRecipients = settings.trigger_failure_email_recipients
			? settings.trigger_failure_email_recipients
			: []
		initialTriggerFailureEmailRecipients = [...triggerFailureEmailRecipients]
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
			// Derive workspace-level legacy defaults (outside repositories)
			const workspaceLegacyIncludePath: string[] = (settings.git_sync as any)?.include_path ?? [];
			const workspaceLegacyIncludeTypeRaw: ObjectType[] = (settings.git_sync as any)?.include_type ?? [];
			// Note: exclude_types_override is only at repository level, not workspace level
			const workspaceLegacyIncludeType: ObjectType[] = [...workspaceLegacyIncludeTypeRaw];

			legacyWorkspaceIncludePath = [...workspaceLegacyIncludePath];
			legacyWorkspaceIncludeType = [...workspaceLegacyIncludeType];

			gitSyncSettings.repositories = (settings.git_sync.repositories ?? []).map((repo: BackendGitRepositorySettings) => {
				gitSyncTestJobs.push({
					jobId: undefined,
					status: undefined
				})

				// Now we have proper nested settings structure from the backend
				const defaultTypes: ObjectType[] = workspaceLegacyIncludeType.length > 0
					? [...workspaceLegacyIncludeType]
					: (['script', 'flow', 'app', 'folder'] as ObjectType[]);

				// Check if this is a legacy repo (no nested settings object)
				const isRepoLegacy = !repo.settings;
				const repoExcludeTypesOverride = repo.exclude_types_override ?? [];

				const repoSettings: GitRepositorySettings = {
					include_path: repo.settings?.include_path ?? [...workspaceLegacyIncludePath],
					exclude_path: repo.settings?.exclude_path ?? [],
					extra_include_path: repo.settings?.extra_include_path ?? [],
					include_type: (repo.settings?.include_type ?? [...defaultTypes]) as ObjectType[]
				};

				return {
					...repo,
					git_repo_resource_path: repo.git_repo_resource_path.replace('$res:', ''),
					collapsed: repo.collapsed ?? false,
					settings: repoSettings,
					exclude_types_override: repoExcludeTypesOverride,
					legacyImported: isRepoLegacy && (legacyWorkspaceIncludePath.length > 0 || legacyWorkspaceIncludeType.length > 0)
				} satisfies GitSyncRepository
			})
			// Store initial settings
			initialGitSyncSettings = JSON.parse(JSON.stringify(gitSyncSettings))
		} else {
			gitSyncSettings.repositories = []
			gitSyncTestJobs = []
			initialGitSyncSettings = undefined
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
		$workspaceStore && untrack(() => loadSettings())
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

	async function editTriggerFailureEmailSettings() {
		try {
			await WorkspaceService.editTriggerFailureEmailNotifications({
				workspace: $workspaceStore!,
				requestBody: {
					trigger_failure_email_recipients: triggerFailureEmailRecipients
				}
			})
			sendUserToast(`Trigger failure email recipients updated`)
			initialTriggerFailureEmailRecipients = [...triggerFailureEmailRecipients]
		} catch (err) {
			sendUserToast(`Failed to update trigger failure email recipients: ${err}`, true)
		}
	}


	async function runGitSyncTestJob(settingsIdx: number) {
		let gitSyncRepository = gitSyncSettings.repositories[settingsIdx]
		if (emptyString(gitSyncRepository.script_path)) {
			return
		}
		let jobId = await JobService.runScriptByPath({
			workspace: $workspaceStore!,
			path: hubPaths.gitSyncTest,
			requestBody: {
				repo_url_resource_path: gitSyncRepository.git_repo_resource_path.replace('$res:', '')
			},
			skipPreprocessor: true
		})
		gitSyncTestJobs[settingsIdx] = {
			jobId: jobId,
			status: 'running'
		}
		gitSyncTestJobs = [...gitSyncTestJobs]
		tryEvery({
			tryCode: async () => {
				const testResult = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
					id: jobId
				})
				gitSyncTestJobs[settingsIdx].status = testResult.success ? 'success' : 'failure'
				gitSyncTestJobs = [...gitSyncTestJobs]
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
				} finally {
					gitSyncTestJobs[settingsIdx].status = 'failure'
					gitSyncTestJobs = [...gitSyncTestJobs]
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
					<div class="text-primary text-lg font-semibold">Trigger Failure Email Notifications</div>
					<Description>
						Configure email addresses to receive notifications when trigger jobs fail. This feature requires SMTP to be configured.
					</Description>
				</div>
			</div>
			<div class="flex flex-col gap-2 my-4">
				<Label>Email Recipients</Label>
				<MultiSelect
					items={[] as { label: string; value: string }[]}
					bind:value={triggerFailureEmailRecipients}
					placeholder="Enter email addresses..."
					onCreateItem={(email) => {
						const emailRegex = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
						if (!emailRegex.test(email)) {
							sendUserToast('Invalid email format', true);
							return;
						}
						if (triggerFailureEmailRecipients.includes(email)) {
							sendUserToast('Email already added', true);
							return;
						}
						triggerFailureEmailRecipients = [...triggerFailureEmailRecipients, email];
					}}
					class="w-full"
				/>
				<div class="flex gap-2 items-center mt-2">
					<Button
						disabled={deepEqual(triggerFailureEmailRecipients, initialTriggerFailureEmailRecipients)}
						size="sm"
						on:click={editTriggerFailureEmailSettings}
					>
						Save Email Settings
					</Button>
					{#if triggerFailureEmailRecipients.length > 0}
						<span class="text-sm text-tertiary">
							{triggerFailureEmailRecipients.length} email{triggerFailureEmailRecipients.length === 1 ? '' : 's'} configured
						</span>
					{/if}
				</div>
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
				<Alert type="info" title="Only new updates trigger git sync">
					Only new changes matching the filters will trigger a git sync. You still need to initialize
					the repo to the desired state first.
				</Alert>
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
						color="red"
						startIcon={{ icon: Save }}
						disabled={!$enterpriseLicense || !hasAnyChanges || gitSyncSettings.repositories.some((_,i)=>!isRepoValid(i))}
						on:click={() => {
							editWindmillGitSyncSettings()
							console.log('Saving git sync settings', gitSyncSettings)
						}}>Save all git sync settings {!$enterpriseLicense ? '(ee only)' : ''}</Button
					>

					<Button
						color="dark"
						target="_blank"
						endIcon={{ icon: ExternalLink }}
						href={`/runs?job_kinds=deploymentcallbacks&workspace=${$workspaceStore}`}
						>See sync jobs</Button
					>
				</div>
				<div class="pt-2"></div>



				{#if Array.isArray(gitSyncSettings.repositories)}
					{#each gitSyncSettings.repositories as repo, idx}
						<div class="rounded-lg shadow-sm border p-0 w-full mb-4">
							<!-- Card Header -->
							<div class="flex items-center justify-between min-h-10 px-4 py-1 border-b">
								<div class="flex items-center gap-2">
									<span class="font-semibold">Repository #{idx + 1}</span>
									<span class="text-xs text-tertiary pt-1 pl-8">
										{repo.git_repo_resource_path}
									</span>
								</div>
								<div class="flex items-center gap-2">
									{#if (repoChanges[idx] || repo.legacyImported) && isRepoValid(idx)}
										<Button
											color="red"
											size="xs"
											on:click={() => saveRepoSettings(idx)}
											startIcon={{ icon: Save }}
										>
											Save changes
										</Button>
										<Button
											color="light"
											size="xs"
											on:click={() => {
												// Revert to initial repository settings
												if (initialGitSyncSettings?.repositories[idx]) {
													gitSyncSettings.repositories[idx] = JSON.parse(JSON.stringify(initialGitSyncSettings.repositories[idx]));
													sendUserToast('Reverted repository settings');
												}
											}}
											startIcon={{ icon: RotateCcw }}
										>
											Revert
										</Button>
									{/if}
									<button
										class="text-secondary hover:text-primary focus:outline-none"
										onclick={() => (repo.collapsed = !repo.collapsed)}
										aria-label="Toggle collapse"
									>
										{#if repo.collapsed}
											<svg
												xmlns="http://www.w3.org/2000/svg"
												class="h-5 w-5"
												fill="none"
												viewBox="0 0 24 24"
												stroke="currentColor"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2"
													d="M19 9l-7 7-7-7"
												/>
											</svg>
										{:else}
											<svg
												xmlns="http://www.w3.org/2000/svg"
												class="h-5 w-5"
												fill="none"
												viewBox="0 0 24 24"
												stroke="currentColor"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2"
													d="M5 15l7-7 7 7"
												/>
											</svg>
										{/if}
									</button>
									<button
										transition:fade|local={{ duration: 100 }}
										class="rounded-full p-2 bg-surface-secondary duration-200 hover:bg-surface-hover"
										aria-label="Remove repository"
										onclick={() => {
											gitSyncSettings.repositories = gitSyncSettings.repositories.filter((_, i) => i !== idx)
										}}
									>
										<Trash size={14} />
									</button>
								</div>
							</div>
							{#if !repo.collapsed}
								<div class="px-4 py-2">
									<div class="flex mt-5 mb-1 gap-1">
										{#key repo}
											<div class="pt-1 font-semibold">Resource: </div>
											<ResourcePicker
												bind:value={repo.git_repo_resource_path}
												resourceType={'git_repository'}
											/>
											{#if !emptyString(repo.git_repo_resource_path)}
												<Button
													disabled={emptyString(repo.script_path)}
													color="dark"
													on:click={() => runGitSyncTestJob(idx)}
													size="xs">Test connection</Button
												>
											{/if}
										{/key}
									</div>

									{#if !emptyString(repo.git_repo_resource_path)}
										<div class="flex mb-5 text-normal text-2xs gap-1">
											{#if isRepoDuplicate(idx)}
												<span class="text-red-700">Using the same resource twice is not allowed.</span
												>
											{/if}
											{#if gitSyncTestJobs[idx].status !== undefined}
												{#if gitSyncTestJobs[idx].status === 'running'}
													<RotateCw size={14} class="animate-spin" />
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

										{#if repo.legacyImported}
											<Alert type="warning" title="Legacy git sync settings imported">
												This repository was initialized from workspace-level legacy Git-Sync settings. Review the filters and press <b>Save</b> to migrate.
											</Alert>
										{/if}
										<div class="flex flex-col mt-5 mb-1 gap-4">
											{#if gitSyncSettings && repo}
												{#if repo.script_path != latestGitSyncHubScript}
													<Alert type="warning" title="Script version mismatch">
														The git sync version for this repository is not latest. Current: <a
															target="_blank"
															href="https://hub.windmill.dev/scripts/windmill/6943/sync-script-to-git-repo-windmill/9014/versions"
															>{repo.script_path}</a
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
																	repo.script_path = latestGitSyncHubScript
																}}
																>Update git sync script (require save git settings to be applied)</Button
															>
														</div>
													</Alert>
												{/if}
												<GitSyncFilterSettings
													git_repo_resource_path={repo.git_repo_resource_path}
													bind:include_path={repo.settings.include_path}
													bind:include_type={repo.settings.include_type}
													bind:exclude_types_override={repo.exclude_types_override}
													isLegacyRepo={repo.legacyImported}
													bind:excludes={repo.settings.exclude_path}
													bind:extraIncludes={repo.settings.extra_include_path}
													bind:yamlText
													onSettingsChange={(settings) => {
														yamlText = settings.yaml
														// Force reactivity update
														repoReactivityTrigger = repoReactivityTrigger + 1
													}}
												/>
												<div class="w-1/3 flex gap-2">
													<InitGitRepoPopover
														gitRepoResourcePath={repo.git_repo_resource_path}
														uiState={{
															include_path: repo.settings.include_path,
															exclude_path: repo.settings.exclude_path || [],
															extra_include_path: repo.settings.extra_include_path || [],
															include_type: repo.settings.include_type
														}}
													/>
													<PullGitRepoPopover
														gitRepoResourcePath={repo.git_repo_resource_path}
														uiState={{
															include_path: repo.settings.include_path,
															exclude_path: repo.settings.exclude_path || [],
															extra_include_path: repo.settings.extra_include_path || [],
															include_type: repo.settings.include_type
														}}
														onFilterUpdate={(filters: { include_path: string[], exclude_path: string[], extra_include_path: string[], include_type: string[] }) => {
															// Direct prop update - much simpler!
															repo.settings.include_path = filters.include_path
															repo.settings.exclude_path = filters.exclude_path
															repo.settings.extra_include_path = filters.extra_include_path
															repo.settings.include_type = filters.include_type as ObjectType[]
														}}
													/>
												</div>
												<Toggle
													disabled={emptyString(repo.git_repo_resource_path)}
													bind:checked={repo.use_individual_branch}
													options={{
														right: 'Create one branch per deployed object',
														rightTooltip:
															"If set, Windmill will create a unique branch per object being pushed based on its path, prefixed with 'wm_deploy/'."
													}}
												/>

												<Toggle
													disabled={emptyString(repo.git_repo_resource_path) ||
														!repo.use_individual_branch}
													bind:checked={repo.group_by_folder}
													options={{
														right: 'Group deployed objects by folder',
														rightTooltip:
															'Instead of creating a branch per object, Windmill will create a branch per folder containing objects being deployed.'
													}}
												/>
											{/if}
										</div>
									{:else}
										<div class="text-tertiary text-sm mt-3 mb-2">
											Select a git repository resource to configure sync settings.
										</div>
									{/if}
								</div>
							{/if}
						</div>
					{/each}
				{/if}

				<div class="flex mt-5 mb-5 gap-1">
					<Button
						color="none"
						variant="border"
						btnClasses="mt-1"
						on:click={() => {
							gitSyncSettings.repositories = [
								...gitSyncSettings.repositories,
								{
									script_path: latestGitSyncHubScript,
									git_repo_resource_path: '',
									use_individual_branch: false,
									group_by_folder: false,
									collapsed: false,
									settings: {
										include_path: [...DEFAULT_INCLUDE_PATH],
										exclude_path: [...DEFAULT_EXCLUDE_PATH],
										extra_include_path: [...DEFAULT_EXTRA_INCLUDE_PATH],
										include_type: ['script', 'flow', 'app', 'folder'] as ObjectType[]
									},
									exclude_types_override: [],
									legacyImported: false
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
