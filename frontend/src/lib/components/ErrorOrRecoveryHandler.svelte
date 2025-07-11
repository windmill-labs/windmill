<script lang="ts">
	import { Alert, Button, Tab, Tabs, Badge } from '$lib/components/common'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ChannelSelector from '$lib/components/ChannelSelector.svelte'

	import type { Schema, SupportedLanguage } from '$lib/common'
	import { base } from '$lib/base'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import MsTeamsIcon from '$lib/components/icons/MSTeamsIcon.svelte'
	import { emptySchema, emptyString, sendUserToast, tryEvery } from '$lib/utils'
	import {
		FlowService,
		JobService,
		type Script,
		ScriptService,
		WorkspaceService,
		type Flow
	} from '$lib/gen'
	import type { ListAvailableTeamsChannelsResponse } from '$lib/gen/types.gen'
	import { inferArgs } from '$lib/infer'
	import { hubBaseUrlStore } from '$lib/stores'

	import { CheckCircle2, Loader2, RotateCw, XCircle, RefreshCcw } from 'lucide-svelte'
	import { hubPaths } from '$lib/hub'

	const slackRecoveryHandler = hubPaths.slackRecoveryHandler
	const slackHandlerScriptPath = hubPaths.slackErrorHandler
	const slackSuccessHandler = hubPaths.slackSuccessHandler

	const teamsRecoveryHandler = hubPaths.teamsRecoveryHandler
	const teamsHandlerScriptPath = hubPaths.teamsErrorHandler
	const teamsSuccessHandler = hubPaths.teamsSuccessHandler

	interface Props {
		errorOrRecovery: 'error' | 'recovery' | 'success'
		isEditable: boolean
		toggleText?: string
		showScriptHelpText?: boolean
		handlerSelected: 'custom' | 'slack' | 'teams'
		handlerPath: string | undefined
		handlerExtraArgs: Record<string, any>
		customScriptTemplate: string
		customHandlerKind?: 'flow' | 'script'
		customTabTooltip?: import('svelte').Snippet
	}

	let {
		errorOrRecovery,
		isEditable,
		toggleText = 'Enable',
		showScriptHelpText = false,
		handlerSelected = $bindable(),
		handlerPath = $bindable(),
		handlerExtraArgs = $bindable(),
		customScriptTemplate,
		customHandlerKind = $bindable('script'),
		customTabTooltip
	}: Props = $props()

	let customHandlerSchema: Schema | undefined = $state()
	let slackHandlerSchema: Schema | undefined = $state()
	let isFetching: boolean = $state(false)

	let teams_channels: ListAvailableTeamsChannelsResponse = $state([])
	let teams_team_name: string | undefined = $state(undefined)

	let workspaceConnectedToSlack: boolean | undefined = $state(undefined)
	let workspaceConnectedToTeams: boolean | undefined = $state(undefined)

	let connectionTestJob: { uuid: string; is_success: boolean; in_progress: boolean } | undefined =
		$state()

	async function loadSlackResources() {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		if (!emptyString(settings.slack_name) && !emptyString(settings.slack_team_id)) {
			workspaceConnectedToSlack = true
		} else {
			workspaceConnectedToSlack = false
		}
	}

	async function loadTeamsResources() {
		isFetching = true
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		if (!emptyString(settings.teams_team_name) && !emptyString(settings.teams_team_id)) {
			workspaceConnectedToTeams = true
		} else {
			workspaceConnectedToTeams = false
		}
		if (workspaceConnectedToTeams) {
			teams_team_name = settings.teams_team_name
			teams_channels = await WorkspaceService.listAvailableTeamsChannels({
				workspace: $workspaceStore!
			})
		}
		isFetching = false
	}

	async function sendMessage(channel: string, platform: 'teams' | 'slack'): Promise<void> {
		const testJobFunction =
			platform === 'slack'
				? WorkspaceService.runSlackMessageTestJob
				: WorkspaceService.runTeamsMessageTestJob

		let submitted_job = await testJobFunction({
			workspace: $workspaceStore!,
			requestBody: {
				hub_script_path: handlerPath,
				channel: channel,
				test_msg: `This is a notification to test the connection between ${platform} and Windmill workspace '${$workspaceStore!}'`
			}
		})

		connectionTestJob = {
			uuid: submitted_job.job_uuid!,
			in_progress: true,
			is_success: false
		}
		tryEvery({
			tryCode: async () => {
				const testResult = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
					id: connectionTestJob!.uuid
				})
				connectionTestJob!.in_progress = false
				connectionTestJob!.is_success = testResult.success
			},
			timeoutCode: async () => {
				try {
					await JobService.cancelQueuedJob({
						workspace: $workspaceStore!,
						id: connectionTestJob!.uuid,
						requestBody: {
							reason: 'Slack message not sent after 5s'
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

	async function sendSlackMessage(channel: string): Promise<void> {
		await sendMessage(channel, 'slack')
	}

	async function sendTeamsMessage(channel: string): Promise<void> {
		await sendMessage(channel, 'teams')
	}

	async function loadHandlerScriptArgs(p: string, defaultArgs: string[] = []) {
		try {
			let schema: Schema | undefined = emptySchema()
			if (p.startsWith('hub/')) {
				const hubScript = await ScriptService.getHubScriptByPath({
					path: p
				})

				if ((hubScript.schema as any)?.properties) {
					schema = hubScript.schema as any
				} else {
					await inferArgs(hubScript.language as SupportedLanguage, hubScript.content ?? '', schema)
				}
			} else {
				let scriptOrFlow: Script | Flow =
					customHandlerKind === 'script'
						? await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: p })
						: await FlowService.getFlowByPath({ workspace: $workspaceStore!, path: p })
				schema = scriptOrFlow.schema as Schema
			}
			if (schema && schema.properties) {
				for (let key in schema.properties) {
					if (defaultArgs.includes(key)) {
						delete schema.properties[key]
					}
				}
				return schema
			}
		} catch (err) {
			sendUserToast(`Could not query handler schema: ${err}`, true)
		}
	}

	function isSlackHandler(scriptPath: string | undefined) {
		if (!scriptPath) {
			return false
		}
		if (errorOrRecovery == 'error') {
			return (
				scriptPath.startsWith('hub/') &&
				scriptPath.endsWith('/workspace-or-schedule-error-handler-slack')
			)
		} else if (errorOrRecovery == 'recovery') {
			return (
				scriptPath.startsWith('hub/') && scriptPath.endsWith('/schedule-recovery-handler-slack')
			)
		} else {
			return scriptPath.startsWith('hub/') && scriptPath.endsWith('/schedule-success-handler-slack')
		}
	}

	function isTeamsHandler(scriptPath: string | undefined) {
		if (!scriptPath) {
			return false
		}
		if (errorOrRecovery == 'error') {
			return (
				scriptPath.startsWith('hub/') &&
				scriptPath.endsWith('/workspace-or-schedule-error-handler-teams')
			)
		} else if (errorOrRecovery == 'recovery') {
			return (
				scriptPath.startsWith('hub/') && scriptPath.endsWith('/schedule-recovery-handler-teams')
			)
		} else {
			return scriptPath.startsWith('hub/') && scriptPath.endsWith('/schedule-success-handler-teams')
		}
	}

	$effect(() => {
		if ($workspaceStore) {
			loadSlackResources()
			loadTeamsResources()
		}
	})

	$effect(() => {
		if (handlerSelected === 'slack' && isSlackHandler(handlerPath)) {
			handlerExtraArgs['slack'] = '$res:f/slack_bot/bot_token'
		} else {
			handlerExtraArgs['slack'] = undefined
		}
	})

	let lastHandlerSelected: 'slack' | 'teams' | 'custom' | undefined = $state(undefined)
	let channelCache = $state({
		slack: undefined as string | undefined,
		teams: undefined as string | undefined
	})
	$effect(() => {
		if (lastHandlerSelected !== handlerSelected && lastHandlerSelected !== undefined) {
			if (lastHandlerSelected === 'teams' || lastHandlerSelected === 'slack') {
				channelCache[lastHandlerSelected] = handlerExtraArgs['channel']
			}

			if (handlerSelected === 'custom') {
				handlerExtraArgs['channel'] = ''
				handlerPath = undefined
			} else {
				handlerExtraArgs['channel'] = channelCache[handlerSelected] ?? ''
			}
		}

		lastHandlerSelected = handlerSelected
	})

	$effect(() => {
		handlerPath &&
			!isSlackHandler(handlerPath) &&
			!isTeamsHandler(handlerPath) &&
			loadHandlerScriptArgs(handlerPath, [
				'path',
				'workspace_id',
				'job_id',
				'is_flow',
				'schedule_path',
				'error',
				'error_started_at',
				'failed_times',
				'started_at',
				'success_times',
				'success_result',
				'success_started_at',
				'email',
				'trigger_path'
			]).then((schema) => (customHandlerSchema = schema))
	})

	$effect(() => {
		handlerPath &&
			isSlackHandler(handlerPath) &&
			loadHandlerScriptArgs(handlerPath, [
				'path',
				'workspace_id',
				'job_id',
				'is_flow',
				'schedule_path',
				'error',
				'error_started_at',
				'failed_times',
				'started_at',
				'success_times',
				'success_result',
				'success_started_at',
				'email',
				'trigger_path',
				'slack'
			]).then((schema) => (slackHandlerSchema = schema))
	})
</script>

<div>
	<Tabs bind:selected={handlerSelected} class="mt-2 mb-4">
		<Tab value="slack" disabled={!isEditable}>Slack</Tab>
		<Tab value="teams" disabled={!isEditable}>Teams</Tab>
		<Tab value="custom" disabled={!isEditable}>
			Custom
			{@render customTabTooltip?.()}
		</Tab>
	</Tabs>
</div>

{#if handlerSelected === 'custom'}
	<div class="flex flex-row mb-2">
		<ScriptPicker
			disabled={!isEditable || !$enterpriseLicense}
			kinds={['script', 'failure']}
			allowFlow={true}
			bind:scriptPath={handlerPath}
			bind:itemKind={customHandlerKind}
			allowRefresh={isEditable}
			clearable
		/>

		{#if !handlerPath}
			<Button
				btnClasses="ml-4 mt-2"
				color="dark"
				size="xs"
				href={customScriptTemplate}
				disabled={!isEditable}
				target="_blank">Create from template</Button
			>
		{/if}
	</div>
	{#if showScriptHelpText}
		<div class="text-xs">
			Example of error handler scripts can be found on <a
				target="_blank"
				href="{$hubBaseUrlStore}/failures"
			>
				Windmill Hub</a
			>
		</div>
	{/if}
	{#if handlerPath}
		<p class="font-semibold text-sm mt-4 mb-2">Extra arguments</p>
		{#await import('$lib/components/SchemaForm.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				disabled={!isEditable}
				schema={customHandlerSchema}
				bind:args={handlerExtraArgs}
				shouldHideNoInputs
				className="text-xs"
			/>
		{/await}
		{#if customHandlerSchema && customHandlerSchema.properties && Object.keys(customHandlerSchema.properties).length === 0}
			<div class="text-xs texg-gray-700">This error handler takes no extra arguments</div>
		{/if}
	{/if}
{:else if handlerSelected === 'slack'}
	<span class="w-full flex mb-3">
		<Toggle
			disabled={!$enterpriseLicense || !isEditable}
			checked={isSlackHandler(handlerPath)}
			options={{ right: toggleText }}
			on:change={async (e) => {
				if (e.detail && errorOrRecovery === 'error') {
					handlerPath = slackHandlerScriptPath
				} else if (e.detail && errorOrRecovery === 'recovery') {
					handlerPath = slackRecoveryHandler
				} else if (e.detail && errorOrRecovery === 'success') {
					handlerPath = slackSuccessHandler
				} else {
					handlerPath = undefined
				}
			}}
		/>
	</span>
	{#if workspaceConnectedToSlack}
		{#await import('$lib/components/SchemaForm.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				disabled={!$enterpriseLicense || !isSlackHandler(handlerPath)}
				schema={slackHandlerSchema}
				hiddenArgs={['slack']}
				schemaFieldTooltip={{
					channel: 'Slack channel name without the "#" - example: "windmill-alerts"'
				}}
				bind:args={handlerExtraArgs}
				shouldHideNoInputs
				className="text-xs"
			/>
		{/await}
	{:else if workspaceConnectedToSlack == undefined}
		<Loader2 class="animate-spin" size={10} />
	{/if}
	{#if $enterpriseLicense && isSlackHandler(handlerPath)}
		{#if workspaceConnectedToSlack == false}
			<Alert type="error" title="Workspace not connected to Slack">
				<div class="flex flex-row gap-x-1 w-full items-center">
					<p class="text-clip grow min-w-0">
						The workspace needs to be connected to Slack to use this feature. You can <a
							target="_blank"
							href="{base}/workspace_settings?tab=slack">configure it here</a
						>.
					</p>
					<Button
						variant="border"
						color="light"
						on:click={loadSlackResources}
						startIcon={{ icon: RotateCw }}
					/>
				</div>
			</Alert>
		{:else}
			<Button
				disabled={emptyString(handlerExtraArgs['channel'])}
				btnClasses="w-32 text-center"
				color="dark"
				on:click={() => sendSlackMessage(handlerExtraArgs['channel'])}
				size="xs">Send test message</Button
			>
			{#if connectionTestJob !== undefined}
				<p class="text-normal text-2xs mt-1 gap-2">
					{#if connectionTestJob.in_progress}
						<RotateCw size={14} />
					{:else if connectionTestJob.is_success}
						<CheckCircle2 size={14} class="text-green-600" />
					{:else}
						<XCircle size={14} class="text-red-700" />
					{/if}
					Message sent via Windmill job
					<a
						target="_blank"
						href={`${base}/run/${connectionTestJob.uuid}?workspace=${$workspaceStore}`}
					>
						{connectionTestJob.uuid}
					</a>
				</p>
			{/if}
		{/if}
	{/if}
{:else if handlerSelected === 'teams'}
	<span class="w-full flex mb-3">
		<Toggle
			disabled={!$enterpriseLicense || !isEditable}
			checked={isTeamsHandler(handlerPath)}
			options={{ right: toggleText }}
			on:change={async (e) => {
				if (e.detail && errorOrRecovery === 'error') {
					handlerPath = teamsHandlerScriptPath
				} else if (e.detail && errorOrRecovery === 'recovery') {
					handlerPath = teamsRecoveryHandler
				} else if (e.detail && errorOrRecovery === 'success') {
					handlerPath = teamsSuccessHandler
				} else {
					handlerPath = undefined
				}
			}}
		/>
	</span>
	{#if workspaceConnectedToTeams}
		<div class="w-2/3 flex flex-col gap-2">
			<div class="flex flex-row items-center gap-2">
				<div class="pt-1 flex-shrink-0">
					<MsTeamsIcon height="24px" width="24px" />
				</div>
				<p class="text-sm">Teams Channel</p>
			</div>

			<div class="flex flex-row gap-2 items-start">
				<ChannelSelector
					containerClass="flex-grow"
					minWidth="200px"
					placeholder="Select Teams channel"
					channels={teams_channels}
					bind:selectedChannel={
						() =>
							handlerExtraArgs['channel']
								? teams_channels.find((ch) => ch.channel_id === handlerExtraArgs['channel'])
								: undefined,
						(channel) => (handlerExtraArgs['channel'] = channel?.channel_id)
					}
				/>
				<div class="flex-shrink-0">
					<button
						onclick={loadTeamsResources}
						class="flex items-center gap-1 p-1.5 rounded hover:bg-surface-hover focus:bg-surface-hover"
					>
						<RefreshCcw size={16} class={isFetching ? 'animate-spin' : ''} />
					</button>
				</div>
			</div>
		</div>
		<div class="flex flex-row gap-2 pb-4">
			<p class="text-sm">
				This workspace is connected to Team: <Badge color="blue" size="xs" class="mt-2"
					>{teams_team_name}</Badge
				>
			</p>
			<Tooltip>
				Each workspace can only be connected to one Microsoft Teams team. You can configure it under <a
					target="_blank"
					href="{base}/workspace_settings?tab=teams">workspace settings</a
				>.
			</Tooltip>
		</div>
	{:else if workspaceConnectedToTeams == undefined}
		<Loader2 class="animate-spin" size={10} />
	{/if}
	{#if $enterpriseLicense && isTeamsHandler(handlerPath)}
		{#if workspaceConnectedToTeams == false}
			<Alert type="error" title="Workspace not connected to Teams">
				<div class="flex flex-row gap-x-1 w-full items-center">
					<p class="text-clip grow min-w-0">
						The workspace needs to be connected to Teams to use this feature. You can configure it
						under <a target="_blank" href="{base}/workspace_settings?tab=teams"
							>workspace settings</a
						>.
					</p>
					<Button
						variant="border"
						color="light"
						on:click={loadTeamsResources}
						startIcon={{ icon: RotateCw }}
					/>
				</div>
			</Alert>
		{:else}
			<Button
				disabled={emptyString(handlerExtraArgs['channel'])}
				btnClasses="w-32 text-center mt-2"
				color="dark"
				on:click={() => sendTeamsMessage(handlerExtraArgs['channel'] ?? '')}
				size="xs">Send test message</Button
			>
			{#if connectionTestJob !== undefined}
				<p class="text-normal text-2xs mt-1 gap-2">
					{#if connectionTestJob.in_progress}
						<RotateCw size={14} class="animate-spin" />
					{:else if connectionTestJob.is_success}
						<CheckCircle2 size={14} class="text-green-600" />
					{:else}
						<XCircle size={14} class="text-red-700" />
					{/if}
					Message sent via Windmill job
					<a
						target="_blank"
						href={`${base}/run/${connectionTestJob.uuid}?workspace=${$workspaceStore}`}
					>
						{connectionTestJob.uuid}
					</a>
				</p>
			{/if}
		{/if}
	{/if}
{/if}
