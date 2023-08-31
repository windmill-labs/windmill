<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { isCloudHosted } from '$lib/cloud'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Badge, Button, Tab, Tabs } from '$lib/components/common'

	import DeployToSetting from '$lib/components/DeployToSetting.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import WorkspaceUserSettings from '$lib/components/settings/WorkspaceUserSettings.svelte'
	import { WORKSPACE_SHOW_SLACK_CMD, WORKSPACE_SHOW_WEBHOOK_CLI_SYNC } from '$lib/consts'
	import { OauthService, Script, WorkspaceService } from '$lib/gen'
	import {
		enterpriseLicense,
		existsOpenaiResourcePath,
		superadmin,
		userStore,
		usersWorkspaceStore,
		workspaceStore
	} from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { setQueryWithoutLoad } from '$lib/utils'
	import { faSlack } from '@fortawesome/free-brands-svg-icons'
	import { faBarsStaggered, faScroll } from '@fortawesome/free-solid-svg-icons'
	import { Slack } from 'lucide-svelte'

	import PremiumInfo from '$lib/components/settings/PremiumInfo.svelte'

	let initialPath: string
	let scriptPath: string
	let team_name: string | undefined
	let itemKind: 'flow' | 'script' = 'flow'
	let plan: string | undefined = undefined
	let customer_id: string | undefined = undefined
	let webhook: string | undefined = undefined
	let workspaceToDeployTo: string | undefined = undefined
	let errorHandlerInitialPath: string
	let errorHandlerScriptPath: string
	let errorHandlerItemKind: 'script' = 'script'
	let openaiResourceInitialPath: string | undefined = undefined
	let tab =
		($page.url.searchParams.get('tab') as
			| 'users'
			| 'slack'
			| 'premium'
			| 'export_delete'
			| 'webhook'
			| 'deploy_to'
			| 'error_handler') ?? 'users'

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

	async function editOpenaiResourcePath(openaiResourcePath: string): Promise<void> {
		// in JS, an empty string is also falsy
		openaiResourceInitialPath = openaiResourcePath
		if (openaiResourcePath) {
			await WorkspaceService.editOpenaiResourcePath({
				workspace: $workspaceStore!,
				requestBody: { openai_resource_path: openaiResourcePath }
			})
			existsOpenaiResourcePath.set(true)
			sendUserToast('OpenAI resource set')
		} else {
			await WorkspaceService.editOpenaiResourcePath({
				workspace: $workspaceStore!,
				requestBody: { openai_resource_path: undefined }
			})
			existsOpenaiResourcePath.set(false)
			sendUserToast(`OpenAI resource removed`)
		}
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
		errorHandlerScriptPath = (settings.error_handler ?? '').split('/').slice(1).join('/')
		errorHandlerInitialPath = errorHandlerScriptPath
	}

	$: {
		if ($workspaceStore) {
			loadSettings()
		}
	}

	async function editErrorHandler() {
		errorHandlerInitialPath = errorHandlerScriptPath
		if (errorHandlerScriptPath) {
			await WorkspaceService.editErrorHandler({
				workspace: $workspaceStore!,
				requestBody: { error_handler: `${errorHandlerItemKind}/${errorHandlerScriptPath}` }
			})
			sendUserToast(`workspace error handler set to ${errorHandlerScriptPath}`)
		} else {
			await WorkspaceService.editErrorHandler({
				workspace: $workspaceStore!,
				requestBody: { error_handler: undefined }
			})
			sendUserToast(`workspace error handler removed`)
		}
	}
</script>

<CenteredPage>
	{#if $userStore?.is_admin || $superadmin}
		<PageHeader title="Workspace settings: {$workspaceStore}" />

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
				<Tab size="xs" value="deploy_to">
					<div class="flex gap-2 items-center my-1"> Dev/Staging/Prod</div>
				</Tab>
				{#if WORKSPACE_SHOW_SLACK_CMD}
					<Tab size="xs" value="slack">
						<div class="flex gap-2 items-center my-1"> Slack Command </div>
					</Tab>
				{/if}
				{#if isCloudHosted()}
					<Tab size="xs" value="premium">
						<div class="flex gap-2 items-center my-1"> Premium Plans </div>
					</Tab>
				{/if}
				<Tab size="xs" value="export_delete">
					<div class="flex gap-2 items-center my-1"> Delete Workspace </div>
				</Tab>
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
			</Tabs>
		</div>
		{#if tab == 'users'}
			<WorkspaceUserSettings />
		{:else if tab == 'deploy_to'}
			<div class="my-2"
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
					<div class=" text-primary text-md font-semibold"> Send commands from slack </div>
					<div class="text-tertiary text-xs">
						Connect your windmill workspace to your slack workspace to trigger a script or a flow
						with a '/windmill' command.
					</div>
				</div>

				{#if team_name}
					<div class="flex flex-col gap-2 max-w-sm">
						<Button
							size="sm"
							endIcon={{ icon: faSlack }}
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
							endIcon={{ icon: faScroll }}
							href="/scripts/add?hub=hub%2F314%2Fslack%2Fexample_of_responding_to_a_slack_command_slack"
						>
							Create a script to handle slack commands
						</Button>
						<Button size="sm" endIcon={{ icon: faBarsStaggered }} href="/flows/add?hub=28">
							Create a flow to handle slack commands
						</Button>
					</div>
				{:else}
					<div class="flex flex-row gap-2">
						<Button size="xs" color="dark" href="/api/oauth/connect_slack">
							<div class="flex flex-row gap-1 items-center">
								<Slack size={14} />
								Connect to Slack
							</div>
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
						kind={Script.kind.SCRIPT}
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
			<PageHeader title="Script to run as error handler" primary={false} />
			<ScriptPicker
				kind={undefined}
				bind:itemKind={errorHandlerItemKind}
				bind:scriptPath={errorHandlerScriptPath}
				initialPath={errorHandlerInitialPath}
				on:select={editErrorHandler}
				allowRefresh
			/>
			<div class="flex flex-col gap-20 items-start mt-3">
				<div class="w-2/3">
					<div class="text-tertiary text-xs">
						The following args will be passed to the error handler:
						<ul class="mt-1 ml-2">
							<li><b>path</b>: The path of the script or flow that errored.</li>
							<li><b>email</b>: The email of the user who ran the script or flow that errored.</li>
							<li><b>error</b>: The error details.</li>
							<li><b>job_id</b>: The job id.</li>
							<li><b>is_flow</b>: Whether the error comes from a flow.</li>
							<li><b>workspace_id</b>: The workspace id of the failed script or flow.</li>
						</ul>
						<br />
						The error handler will be executed by the automatically created group g/error_handler. If
						your error handler requires variables or resources, you need to add them to the group.
					</div>
				</div>
				<div class="w-1/3 flex items-start">
					<div class="mt-2">
						<!-- Adjusted margin class -->
						<Button
							href="/scripts/add?hub=hub%2F1088%2Fwindmill%2FGlobal_%2F_workspace_error_handler_template"
							target="_blank">Use template</Button
						>
					</div>
				</div>
			</div>
		{:else if tab == 'openai'}
			<PageHeader title="Windmill AI" primary={false} />
			<div class="mt-2">
				<Alert type="info" title="Select an OpenAI resource to unlock Windmill AI features!">
					Windmill AI currently only supports OpenAI's GPT-4.
				</Alert>
			</div>
			<div class="mt-5">
				{#key openaiResourceInitialPath}
					<ResourcePicker
						resourceType="openai"
						initialValue={openaiResourceInitialPath}
						on:change={(ev) => {
							editOpenaiResourcePath(ev.detail)
						}}
					/>
				{/key}
			</div>
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
