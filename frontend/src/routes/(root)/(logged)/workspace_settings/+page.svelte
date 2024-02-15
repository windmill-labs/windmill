<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { isCloudHosted } from '$lib/cloud'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Badge, Button, Tab, Tabs } from '$lib/components/common'

	import DeployToSetting from '$lib/components/DeployToSetting.svelte'
	import ErrorOrRecoveryHandler from '$lib/components/ErrorOrRecoveryHandler.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import WorkspaceUserSettings from '$lib/components/settings/WorkspaceUserSettings.svelte'
	import { WORKSPACE_SHOW_SLACK_CMD, WORKSPACE_SHOW_WEBHOOK_CLI_SYNC } from '$lib/consts'
	import {
		LargeFileStorage,
		OauthService,
		Script,
		WorkspaceService,
		JobService,
		ResourceService
	} from '$lib/gen'
	import {
		enterpriseLicense,
		copilotInfo,
		superadmin,
		userStore,
		usersWorkspaceStore,
		workspaceStore
	} from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { setQueryWithoutLoad, emptyString, tryEvery } from '$lib/utils'
	import {
		Scroll,
		Slack,
		XCircle,
		RotateCw,
		CheckCircle2,
		X,
		Plus,
		Loader2,
		Save
	} from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import PremiumInfo from '$lib/components/settings/PremiumInfo.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import TestOpenaiKey from '$lib/components/copilot/TestOpenaiKey.svelte'
	import Portal from 'svelte-portal'
	import { fade } from 'svelte/transition'

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

	let s3FileViewer: S3FilePicker

	let initialPath: string
	let scriptPath: string
	let team_name: string | undefined
	let itemKind: 'flow' | 'script' = 'flow'
	let plan: string | undefined = undefined
	let customer_id: string | undefined = undefined
	let webhook: string | undefined = undefined
	let workspaceToDeployTo: string | undefined = undefined
	let errorHandlerSelected: 'custom' | 'slack' = 'slack'
	let errorHandlerInitialScriptPath: string
	let errorHandlerScriptPath: string
	let errorHandlerItemKind: 'flow' | 'script' = 'script'
	let errorHandlerExtraArgs: Record<string, any> = {}
	let errorHandlerMutedOnCancel: boolean | undefined = undefined
	let openaiResourceInitialPath: string | undefined = undefined
	let s3ResourceSettings: {
		resourceType: 's3' | 'azure_blob'
		resourcePath: string | undefined
		publicResource: boolean | undefined
	}
	let gitSyncSettings: {
		include_path: string[]
		repositories: {
			exclude_types_override: GitSyncTypeMap
			script_path: string
			git_repo_resource_path: string
			use_individual_branch: boolean
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
	let codeCompletionEnabled: boolean = false
	let tab =
		($page.url.searchParams.get('tab') as
			| 'users'
			| 'slack'
			| 'premium'
			| 'export_delete'
			| 'webhook'
			| 'deploy_to'
			| 'error_handler') ?? 'users'
	let usingOpenaiClientCredentialsOauth = false

	// function getDropDownItems(username: string): DropdownItem[] {
	// 	return [
	// 		{
	// 			displayName: 'Manage user',
	// 			href: `/admin/user/manage/${username}`
	// 		},
	// 		{
	// 			displayName: 'Delete',
	// 			action: () => deleteUser(username)
	// 		}
	// 	];
	// }

	// async function deleteUser(username: string): Promise<void> {
	// 	try {
	// 		await UserService.deleteUser({ workspace: $workspaceStore!, username });
	// 		users = await UserService.listUsers({ workspace: $workspaceStore! });
	// 		fuse?.setCollection(users);
	// 		sendUserToast(`User ${username} has been removed`);
	// 	} catch (err) {
	// 		console.error(err);
	// 		sendUserToast(`Cannot delete user: ${err}`, true);
	// 	}
	// }

	async function editSlackCommand(): Promise<void> {
		initialPath = scriptPath
		if (scriptPath) {
			await WorkspaceService.editSlackCommand({
				workspace: $workspaceStore!,
				requestBody: { slack_command_script: `${itemKind}/${scriptPath}` }
			})
			sendUserToast(`slack command script set to ${scriptPath}`)
		} else {
			await WorkspaceService.editSlackCommand({
				workspace: $workspaceStore!,
				requestBody: { slack_command_script: undefined }
			})
			sendUserToast(`slack command script removed`)
		}
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

	async function editCopilotConfig(openaiResourcePath: string): Promise<void> {
		// in JS, an empty string is also falsy
		openaiResourceInitialPath = openaiResourcePath
		if (openaiResourcePath) {
			await WorkspaceService.editCopilotConfig({
				workspace: $workspaceStore!,
				requestBody: {
					openai_resource_path: openaiResourcePath,
					code_completion_enabled: codeCompletionEnabled
				}
			})
			copilotInfo.set({
				exists_openai_resource_path: true,
				code_completion_enabled: codeCompletionEnabled
			})
		} else {
			await WorkspaceService.editCopilotConfig({
				workspace: $workspaceStore!,
				requestBody: {
					openai_resource_path: undefined,
					code_completion_enabled: codeCompletionEnabled
				}
			})
			copilotInfo.set({
				exists_openai_resource_path: true,
				code_completion_enabled: codeCompletionEnabled
			})
		}
		sendUserToast(`Copilot settings updated`)
	}

	async function editWindmillLFSSettings(): Promise<void> {
		if (!emptyString(s3ResourceSettings.resourcePath)) {
			let resourcePathWithPrefix = `$res:${s3ResourceSettings.resourcePath}`
			let params = {
				public_resource: s3ResourceSettings.publicResource
			}
			if (s3ResourceSettings.resourceType === 'azure_blob') {
				params['type'] = LargeFileStorage.type.AZURE_BLOB_STORAGE
				params['azure_blob_resource_path'] = resourcePathWithPrefix
			} else {
				params['type'] = LargeFileStorage.type.S3STORAGE
				params['s3_resource_path'] = resourcePathWithPrefix
			}
			await WorkspaceService.editLargeFileStorageConfig({
				workspace: $workspaceStore!,
				requestBody: {
					large_file_storage: params
				}
			})
			sendUserToast(`Large file storage settings updated`)
		} else {
			await WorkspaceService.editLargeFileStorageConfig({
				workspace: $workspaceStore!,
				requestBody: {
					large_file_storage: undefined
				}
			})
			sendUserToast(`Large file storage settings reset`)
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
				use_individual_branch: elmt.use_individual_branch
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
		setTimeout(() => {
			workspaceReencryptionInProgress = false
		}, 1000 - (timeEnd - timeStart))
	}

	async function loadSettings(): Promise<void> {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		team_name = settings.slack_name

		if (settings.slack_command_script) {
			itemKind = settings.slack_command_script.split('/')[0] as 'flow' | 'script'
		}
		scriptPath = (settings.slack_command_script ?? '').split('/').slice(1).join('/')
		initialPath = scriptPath
		plan = settings.plan
		customer_id = settings.customer_id
		workspaceToDeployTo = settings.deploy_to
		webhook = settings.webhook
		openaiResourceInitialPath = settings.openai_resource_path
		errorHandlerItemKind = settings.error_handler?.split('/')[0] as 'flow' | 'script'
		errorHandlerScriptPath = (settings.error_handler ?? '').split('/').slice(1).join('/')
		errorHandlerInitialScriptPath = errorHandlerScriptPath
		errorHandlerMutedOnCancel = settings.error_handler_muted_on_cancel
		if (emptyString($enterpriseLicense)) {
			errorHandlerSelected = 'custom'
		} else {
			errorHandlerSelected =
				emptyString(errorHandlerScriptPath) ||
				(errorHandlerScriptPath.startsWith('hub/') &&
					errorHandlerScriptPath.endsWith('/workspace-or-schedule-error-handler-slack'))
					? 'slack'
					: 'custom'
		}
		errorHandlerExtraArgs = settings.error_handler_extra_args ?? {}
		codeCompletionEnabled = settings.code_completion_enabled
		workspaceDefaultAppPath = settings.default_app

		if (settings.large_file_storage?.type === LargeFileStorage.type.S3STORAGE) {
			s3ResourceSettings = {
				resourceType: 's3',
				resourcePath: settings.large_file_storage?.s3_resource_path?.replace('$res:', ''),
				publicResource: settings.large_file_storage?.public_resource
			}
		} else if (settings.large_file_storage?.type === LargeFileStorage.type.AZURE_BLOB_STORAGE) {
			s3ResourceSettings = {
				resourceType: 'azure_blob',
				resourcePath: settings.large_file_storage?.azure_blob_resource_path?.replace('$res:', ''),
				publicResource: settings.large_file_storage?.public_resource
			}
		} else {
			s3ResourceSettings = {
				resourceType: 's3',
				resourcePath: undefined,
				publicResource: undefined
			}
		}
		if (settings.git_sync !== undefined && settings.git_sync !== null) {
			gitSyncTestJobs = []
			gitSyncSettings = {
				include_path:
					settings.git_sync.include_path?.length ?? 0 > 0
						? settings.git_sync.include_path ?? []
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
						exclude_types_override: {
							scripts: (settings.exclude_types_override?.indexOf('script') ?? -1) >= 0,
							flows: (settings.exclude_types_override?.indexOf('flow') ?? -1) >= 0,
							apps: (settings.exclude_types_override?.indexOf('app') ?? -1) >= 0,
							resourceTypes: (settings.exclude_types_override?.indexOf('resourcetype') ?? -1) >= 0,
							resources: (settings.exclude_types_override?.indexOf('resource') ?? -1) >= 0,
							variables: (settings.exclude_types_override?.indexOf('variable') ?? -1) >= 0,
							secrets: (settings.exclude_types_override?.indexOf('secret') ?? -1) >= 0,
							schedules: (settings.exclude_types_override?.indexOf('schedule') ?? -1) >= 0,
							folders: (settings.exclude_types_override?.indexOf('folder') ?? -1) >= 0
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
					folders: (settings.git_sync.include_type?.indexOf('folder') ?? -1) >= 0
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
					schedules: false
				}
			}
			gitSyncTestJobs = []
		}

		// check openai_client_credentials_oauth
		usingOpenaiClientCredentialsOauth = await ResourceService.existsResourceType({
			workspace: $workspaceStore!,
			path: 'openai_client_credentials_oauth'
		})
	}

	$: {
		if ($workspaceStore) {
			loadSettings()
		}
	}

	async function editErrorHandler() {
		if (errorHandlerScriptPath) {
			if (errorHandlerScriptPath !== undefined && isSlackHandler(errorHandlerScriptPath)) {
				errorHandlerExtraArgs['slack'] = '$res:f/slack_bot/bot_token'
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
			path: 'hub/7925/git-repo-test-read-write-windmill',
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
</script>

<Portal>
	<S3FilePicker bind:this={s3FileViewer} readOnlyMode={false} fromWorkspaceSettings={true} />
</Portal>

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
					setQueryWithoutLoad($page.url, [{ key: 'tab', value: tab }], 0)
				}}
			>
				<Tab size="xs" value="users">
					<div class="flex gap-2 items-center my-1"> Users</div>
				</Tab>
				<Tab size="xs" value="git_sync">
					<div class="flex gap-2 items-center my-1">Git Sync</div>
				</Tab>
				<Tab size="xs" value="deploy_to">
					<div class="flex gap-2 items-center my-1">Deployment UI</div>
				</Tab>
				{#if WORKSPACE_SHOW_SLACK_CMD}
					<Tab size="xs" value="slack">
						<div class="flex gap-2 items-center my-1"> Slack </div>
					</Tab>
				{/if}
				{#if isCloudHosted()}
					<Tab size="xs" value="premium">
						<div class="flex gap-2 items-center my-1"> Premium Plans </div>
					</Tab>
				{/if}
				{#if WORKSPACE_SHOW_WEBHOOK_CLI_SYNC}
					<Tab size="xs" value="webhook">
						<div class="flex gap-2 items-center my-1">Webhook</div>
					</Tab>
				{/if}
				<Tab size="xs" value="error_handler">
					<div class="flex gap-2 items-center my-1">Error Handler</div>
				</Tab>
				<Tab size="xs" value="openai">
					<div class="flex gap-2 items-center my-1">Windmill AI</div>
				</Tab>
				<Tab size="xs" value="windmill_lfs">
					<div class="flex gap-2 items-center my-1"> S3 Storage </div>
				</Tab>
				<Tab size="xs" value="default_app">
					<div class="flex gap-2 items-center my-1"> Default App </div>
				</Tab>
				<Tab size="xs" value="encryption">
					<div class="flex gap-2 items-center my-1"> Encryption </div>
				</Tab>
				<Tab size="xs" value="export_delete">
					<div class="flex gap-2 items-center my-1"> Delete Workspace </div>
				</Tab>
			</Tabs>
		</div>
		{#if tab == 'users'}
			<WorkspaceUserSettings />
		{:else if tab == 'deploy_to'}
			<div class="my-2 pt-4"
				><Alert type="info" title="Link this workspace to another Staging/Prod workspace"
					>Linking this workspace to another staging/prod workspace unlock the Web-based flow to
					deploy to another workspace.</Alert
				></div
			>
			{#if $enterpriseLicense}
				<DeployToSetting bind:workspaceToDeployTo />
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
					<div class=" text-primary text-md font-semibold"> Connect workspace to Slack </div>
					<div class="text-tertiary text-xs">
						Connect your Windmill workspace to your Slack workspace to trigger a script or a flow
						with a '/windmill' command or to configure Slack error handlers.
					</div>
				</div>

				{#if team_name}
					<div class="flex flex-col gap-2 max-w-sm">
						<Button
							size="sm"
							endIcon={{ icon: Slack }}
							btnClasses="mt-2"
							variant="border"
							on:click={async () => {
								await OauthService.disconnectSlack({
									workspace: $workspaceStore ?? ''
								})
								loadSettings()
								sendUserToast('Disconnected Slack')
							}}
						>
							Disconnect Slack
						</Button>
						<Button
							size="sm"
							endIcon={{ icon: Scroll }}
							href="/scripts/add?hub=hub%2F314%2Fslack%2Fexample_of_responding_to_a_slack_command_slack"
						>
							Create a script to handle slack commands
						</Button>
						<Button size="sm" endIcon={{ icon: BarsStaggered }} href="/flows/add?hub=28">
							Create a flow to handle slack commands
						</Button>
					</div>
				{:else}
					<div class="flex flex-row gap-2">
						<Button
							size="xs"
							color="dark"
							href="/api/oauth/connect_slack"
							startIcon={{ icon: Slack }}
						>
							Connect to Slack
						</Button>
						<Badge color="red">Not connnected</Badge>
					</div>
				{/if}
			</div>
			<div class="bg-surface-disabled p-4 rounded-md flex flex-col gap-1">
				<div class="text-primary font-md font-semibold">
					Script or flow to run on /windmill command
				</div>
				<div class="relative">
					{#if !team_name}
						<div class="absolute top-0 right-0 bottom-0 left-0 bg-surface-disabled/50 z-40" />
					{/if}
					<ScriptPicker
						kinds={[Script.kind.SCRIPT]}
						allowFlow
						bind:itemKind
						bind:scriptPath
						{initialPath}
						on:select={editSlackCommand}
					/>
				</div>

				<div class="prose text-2xs text-tertiary">
					Pick a script or flow meant to be triggered when the `/windmill` command is invoked. Upon
					connection, templates for a <a href="https://hub.windmill.dev/scripts/slack/1405/"
						>script</a
					>
					and <a href="https://hub.windmill.dev/flows/28/">flow</a> are available.

					<br /><br />

					The script or flow chosen is passed the parameters `response_url: string` and `text:
					string` respectively the url to reply directly to the trigger and the text of the command.

					<br /><br />

					The script or flow is permissioned as group "slack" that will be automatically created
					after connection to Slack.

					<br /><br />

					See more on <a href="https://www.windmill.dev/docs/integrations/slack">documentation</a>.
				</div>
			</div>
		{:else if tab == 'export_delete'}
			<PageHeader title="Export workspace" primary={false} />
			<div class="flex justify-start">
				<Button
					size="sm"
					href="/api/w/{$workspaceStore ?? ''}/workspaces/tarball?archive_type=zip"
					target="_blank"
				>
					Export workspace as zip file
				</Button>
			</div>

			<div class="mt-20" />
			<PageHeader title="Delete workspace" primary={false} />
			<p class="italic text-xs">
				The workspace will be archived for a short period of time and then permanently deleted
			</p>
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
			<PageHeader title="Webhook on changes" primary={false} />

			<div class="mt-2"
				><Alert type="info" title="Send events to an external service"
					>Connect your windmill workspace to an external service to sync or get notified about any
					changes.</Alert
				></div
			>

			<h3 class="mt-5 text-secondary"
				>URL to send requests to<Tooltip>
					This URL will be POSTed to with a JSON body depending on the type of event. The type is
					indicated by the <pre>type</pre> field. The other fields are dependent on the type.
				</Tooltip>
			</h3>

			<div class="flex gap-2">
				<input class="justify-start" type="text" bind:value={webhook} />
				<Button color="blue" btnClasses="justify-end" on:click={editWebhook}>Set Webhook</Button>
			</div>
		{:else if tab == 'error_handler'}
			{#if !$enterpriseLicense}
				<div class="pt-4" />
				<Alert type="info" title="Workspace error handler is an EE feature">
					Workspace error handler is a Windmill EE feature. It enables using your current Slack
					connection or a custom script to send notifications anytime any job would fail.
				</Alert>
			{/if}

			<PageHeader title="Script to run as error handler" primary={false} />

			<ErrorOrRecoveryHandler
				isEditable={true}
				errorOrRecovery="error"
				showScriptHelpText={true}
				customInitialScriptPath={errorHandlerInitialScriptPath}
				bind:handlerSelected={errorHandlerSelected}
				bind:handlerPath={errorHandlerScriptPath}
				customScriptTemplate="/scripts/add?hub=hub%2F2420%2Fwindmill%2Fworkspace_error_handler_template"
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
						(errorHandlerSelected === 'slack' &&
							!emptyString(errorHandlerScriptPath) &&
							emptyString(errorHandlerExtraArgs['channel']))}
					bind:checked={errorHandlerMutedOnCancel}
					options={{ right: 'Do not run error handler for canceled jobs' }}
				/>
				<Button
					disabled={!$enterpriseLicense ||
						(errorHandlerSelected === 'slack' &&
							!emptyString(errorHandlerScriptPath) &&
							emptyString(errorHandlerExtraArgs['channel']))}
					size="sm"
					on:click={editErrorHandler}
				>
					Save
				</Button>
			</div>
		{:else if tab == 'openai'}
			<PageHeader title="Windmill AI" primary={false} />
			<div class="mt-2">
				<Alert type="info" title="Select an OpenAI resource to unlock Windmill AI features!">
					Windmill AI uses OpenAI's GPT-3.5-turbo for code completion and GPT-4 Turbo for all other
					AI features.
				</Alert>
			</div>
			<div class="mt-5 flex gap-1">
				{#key [openaiResourceInitialPath, usingOpenaiClientCredentialsOauth]}
					<ResourcePicker
						resourceType={usingOpenaiClientCredentialsOauth
							? 'openai_client_credentials_oauth'
							: 'openai'}
						initialValue={openaiResourceInitialPath}
						on:change={(ev) => {
							editCopilotConfig(ev.detail)
						}}
					/>
				{/key}
				<TestOpenaiKey disabled={!openaiResourceInitialPath} />
			</div>
			<div class="mt-3">
				<Toggle
					class="mr-2"
					bind:checked={codeCompletionEnabled}
					options={{ right: 'Enable code completion' }}
					on:change={() => {
						editCopilotConfig(openaiResourceInitialPath || '')
					}}
				/>
			</div>
		{:else if tab == 'windmill_lfs'}
			<PageHeader title="S3 Storage" primary={false} />
			{#if !$enterpriseLicense}
				<Alert type="info" title="S3 storage is limited to 20 files in Windmill CE">
					Windmill S3 bucket browser will not work for buckets containing more than 20 files and
					uploads are limited to files {'<'} 50MB. Consider upgrading to Windmill EE to use this feature
					with large buckets.
				</Alert>
			{/if}
			{#if s3ResourceSettings}
				<div class="mt-5 flex gap-1">
					{#key s3ResourceSettings.resourcePath}
						<ResourcePicker
							resourceType="s3,azure_blob"
							bind:value={s3ResourceSettings.resourcePath}
							bind:valueType={s3ResourceSettings.resourceType}
						/>
					{/key}
					<Button
						size="sm"
						variant="contained"
						color="dark"
						disabled={emptyString(s3ResourceSettings.resourcePath)}
						on:click={async () => {
							if ($workspaceStore) {
								s3FileViewer?.open?.(undefined)
							}
						}}>Browse content (save first)</Button
					>
				</div>
				<div class="flex flex-col mt-5 mb-1 gap-1">
					<Toggle
						disabled={emptyString(s3ResourceSettings.resourcePath)}
						bind:checked={s3ResourceSettings.publicResource}
						options={{
							right: 'S3 resource details can be accessed by all users of this workspace',
							rightTooltip:
								'If set, all users of this workspace will have access the to entire content of the S3 bucket, as well as the resource details. this effectively by-pass the permissions set on the resource and makes it public to everyone.'
						}}
					/>
					{#if s3ResourceSettings.publicResource === true}
						<Alert type="warning" title="S3 bucket content and resource details are shared">
							S3 resource public access is ON, which means that the entire content of the S3 bucket
							will be accessible to all the users of this workspace regardless of whether they have
							access the resource or not. Similarly, certain Windmill SDK endpoints can be used in
							scripts to access the resource details, including public and private keys.
						</Alert>
					{/if}
				</div>
				<div class="flex mt-5 mb-5 gap-1">
					<Button
						color="blue"
						disabled={emptyString(s3ResourceSettings.resourcePath)}
						on:click={() => {
							editWindmillLFSSettings()
							console.log('Saving S3 settings', s3ResourceSettings)
						}}>Save S3 settings</Button
					>
				</div>
			{/if}
		{:else if tab == 'git_sync'}
			<PageHeader
				title="Git sync"
				primary={false}
				tooltip="Connect the Windmill workspace to a Git repository to automatically commit and push scripts, flows and apps to the repository on each deploy."
				documentationLink="https://www.windmill.dev/docs/advanced/git_sync"
			/>
			<div class="flex flex-col gap-1">
				<div class="text-tertiary text-xs">
					Connect the Windmill workspace to a Git repository to automatically commit and push
					scripts, flows and apps to the repository on each deploy.
				</div>
			</div>
			{#if !$enterpriseLicense}
				<Alert type="warning" title="Syncing workspace to Git is an EE feature">
					Automatically saving scripts to a Git repository on each deploy is a Windmill EE feature.
				</Alert>
				<div class="mb-1" />
			{/if}
			{#if gitSyncSettings != undefined}
				{#if $enterpriseLicense}
					<div class="flex mt-5 mb-5 gap-1">
						<Button
							color="blue"
							disabled={gitSyncSettings?.repositories?.some((elmt) =>
								emptyString(elmt.git_repo_resource_path)
							)}
							on:click={() => {
								editWindmillGitSyncSettings()
								console.log('Saving git sync settings', gitSyncSettings)
							}}>Save Git sync settings</Button
						>
					</div>
				{/if}

				<div class="flex flex-wrap gap-20">
					<div class="max-w-md w-full">
						{#if Array.isArray(gitSyncSettings?.include_path)}
							<h4 class="flex gap-2 mb-4"
								>Filter on path<Tooltip>
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
					</div>

					<div class="max-w-md w-full">
						<h4 class="flex gap-2 mb-4"
							>Filter on type<Tooltip>
								On top of the filter path above, you can include only certain type of object to be
								synced with the Git repository.
								<br />By default everything is synced.
							</Tooltip></h4
						>
						<div class="flex flex-col gap-1 mt-1">
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

						<div class="flex mt-5 mb-1 gap-1">
							{#if gitSyncSettings}
								<Toggle
									disabled={emptyString(gitSyncRepository.git_repo_resource_path)}
									bind:checked={gitSyncRepository.use_individual_branch}
									options={{
										right: 'Create one branch per deployed object',
										rightTooltip:
											"If set, Windmill will create a unique branch per object being pushed based on its path, prefixed with 'wm_deploy/'."
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
									script_path: 'hub/7958/sync-script-to-git-repo-windmill',
									git_repo_resource_path: '',
									use_individual_branch: false,
									exclude_types_override: {
										scripts: false,
										flows: false,
										apps: false,
										folders: false,
										resourceTypes: false,
										resources: false,
										variables: false,
										secrets: false,
										schedules: false
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

						. For the git repo to be representative of the entire workspace, it is recommended to
						set it up using the Windmill CLI before turning this option on.

						<br /><br />

						Not familiar with Windmill CLI?
						<a href="https://www.windmill.dev/docs/advanced/cli">Check out the docs</a>

						<br /><br />

						Run the following commands from the git repo folder to push the initial workspace
						content to the remote:

						<br />

						<pre class="overflow-auto max-h-screen"
							><code
								>wmill workspace add  {$workspaceStore} {$workspaceStore} {`${$page.url.protocol}//${$page.url.hostname}/`}
echo 'includes: ["f/**"]' > wmill.yaml
wmill sync pull --raw --skip-variables --skip-secrets --skip-resources
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
			<PageHeader
				title="Workspace default app"
				tooltip="Users who are operators in this workspace will be redirected to this app automatically when login into this workspace."
				primary={false}
			/>
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
			<PageHeader title="Workspace secret encryption" primary={false} />
			<Alert type="info" title="Windmill EE only feature">
				When updating the encryption key of a workspace, all secrets will be re-encrypted with the
				new key and the previous key will be replaced by the new one.
				<br />
				If you're manually updating the key to match another workspace key from another Windmill instance,
				make sure not to use the 'SECRET_SALT' environment variable or, if you're using it, make sure
				it the salt matches across both instances.
			</Alert>
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
