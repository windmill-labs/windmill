<script lang="ts" module>
	export const errorHandlerArgs = [
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
	]

	export const slackErrorHandlerHubPathEnding = '/workspace-or-schedule-error-handler-slack'
</script>

<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import SlackConnectionStatus from '$lib/components/common/slack/SlackConnectionStatus.svelte'
	import TeamsConnectionStatus from '$lib/components/common/teams/TeamsConnectionStatus.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ChannelSelector from '$lib/components/ChannelSelector.svelte'

	import type { Schema, SupportedLanguage } from '$lib/common'
	import { base } from '$lib/base'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import MsTeamsIcon from '$lib/components/icons/MSTeamsIcon.svelte'
	import { classNames, emptySchema, emptyString, sendUserToast, tryEvery } from '$lib/utils'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import {
		FlowService,
		JobService,
		type Script,
		ScriptService,
		WorkspaceService,
		type Flow
	} from '$lib/gen'
	import type { ErrorHandler } from '$lib/gen/types.gen'
	import { inferArgs } from '$lib/infer'
	import { hubBaseUrlStore } from '$lib/stores'

	import {
		CheckCircle2,
		CircleCheck,
		CircleX,
		ExternalLink,
		Loader2,
		RotateCw,
		XCircle
	} from 'lucide-svelte'
	import { hubPaths } from '$lib/hub'
	import { isCloudHosted } from '$lib/cloud'
	import SmtpConfigurationStatus from './common/smtp/SmtpConfigurationStatus.svelte'
	import { SettingService } from '$lib/gen'
	import { isSmtpSettingsValid } from './instanceSettings/SmtpSettings.svelte'

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
		handlerSelected: ErrorHandler
		handlerPath: string | undefined
		handlerExtraArgs: Record<string, any>
		customScriptTemplate: string
		customHandlerKind?: 'flow' | 'script'
		customTabTooltip?: import('svelte').Snippet
		noMargin?: boolean
	}

	let {
		errorOrRecovery,
		isEditable,
		toggleText = 'Enable',
		showScriptHelpText = false,
		handlerSelected = $bindable('custom'),
		handlerPath = $bindable(),
		handlerExtraArgs = $bindable(),
		customScriptTemplate,
		customHandlerKind = $bindable('script'),
		customTabTooltip,
		noMargin = false
	}: Props = $props()

	let customHandlerSchema: Schema | undefined = $state()
	let slackHandlerSchema: Schema | undefined = $state()
	let teams_team_name: string | undefined = $state(undefined)
	let teams_team_guid: string | undefined = $state(undefined)
	let slack_team_name: string | undefined = $state(undefined)

	let workspaceConnectedToSlack: boolean | undefined = $state(undefined)
	let workspaceConnectedToTeams: boolean | undefined = $state(undefined)
	let hasSmtpConfig: boolean = $state(false)

	let connectionTestJob: { uuid: string; is_success: boolean; in_progress: boolean } | undefined =
		$state()
	const EMAIL_RECIPIENTS_KEY = 'email_recipients'
	const CHANNEL_KEY = 'channel'

	async function loadSlackResources() {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		if (!emptyString(settings.slack_name) && !emptyString(settings.slack_team_id)) {
			workspaceConnectedToSlack = true
			slack_team_name = settings.slack_name
		} else {
			workspaceConnectedToSlack = false
			slack_team_name = undefined
		}
	}

	async function loadTeamsResources() {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		if (!emptyString(settings.teams_team_name) && !emptyString(settings.teams_team_id)) {
			workspaceConnectedToTeams = true
		} else {
			workspaceConnectedToTeams = false
		}
		if (workspaceConnectedToTeams) {
			teams_team_name = settings.teams_team_name
			teams_team_guid = settings.teams_team_guid
		}
	}

	async function loadSmtpConfiguration() {
		try {
			const smtpSettings = (await SettingService.getGlobal({ key: 'smtp_settings' })) as Record<
				string,
				any
			> | null
			hasSmtpConfig = smtpSettings ? isSmtpSettingsValid(smtpSettings) : false
		} catch (error) {
			hasSmtpConfig = false
		}
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
							reason: 'Slack message not sent after 10s'
						}
					})
				} catch (err) {
					console.error(err)
				}
			},
			interval: 500,
			timeout: 10000
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
			return scriptPath.startsWith('hub/') && scriptPath.endsWith(slackErrorHandlerHubPathEnding)
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

	function isEmailHandler(scriptPath: string | undefined) {
		if (!scriptPath) {
			return false
		}

		return scriptPath.startsWith('hub/') && scriptPath.endsWith('/workspace-or-error-handler-email')
	}

	$effect(() => {
		if ($workspaceStore) {
			loadSlackResources()
			loadTeamsResources()
		}
		loadSmtpConfiguration()
	})

	$effect(() => {
		if (handlerSelected === 'slack' && isSlackHandler(handlerPath)) {
			handlerExtraArgs['slack'] = '$res:f/slack_bot/bot_token'
		} else {
			handlerExtraArgs['slack'] = undefined
		}
	})

	let lastHandlerSelected: ErrorHandler | undefined = $state(undefined)
	let handlerCache = $state({
		slack: undefined as string | undefined,
		teams: undefined as string | undefined,
		email: undefined as string[] | undefined
	})
	let handlerPathCache: Partial<Record<ErrorHandler, string | undefined>> = $state({})
	$effect(() => {
		if (lastHandlerSelected !== handlerSelected && lastHandlerSelected !== undefined) {
			if (lastHandlerSelected != 'custom') {
				const key = lastHandlerSelected === 'email' ? EMAIL_RECIPIENTS_KEY : CHANNEL_KEY
				handlerCache[lastHandlerSelected] = handlerExtraArgs[key]
			}
			handlerPathCache[lastHandlerSelected] = handlerPath

			if (handlerSelected === 'custom') {
				delete handlerExtraArgs[CHANNEL_KEY]
				delete handlerExtraArgs[EMAIL_RECIPIENTS_KEY]
				handlerPath = handlerPathCache['custom']
			} else if (handlerSelected === 'email') {
				handlerExtraArgs[EMAIL_RECIPIENTS_KEY] = handlerCache[handlerSelected] ?? []
				delete handlerExtraArgs[CHANNEL_KEY]
			} else {
				handlerExtraArgs[CHANNEL_KEY] = handlerCache[handlerSelected] ?? ''
				delete handlerExtraArgs[EMAIL_RECIPIENTS_KEY]
				handlerPath = handlerPathCache[handlerSelected]
			}
		}

		lastHandlerSelected = handlerSelected
	})

	$effect(() => {
		handlerPath &&
			!isSlackHandler(handlerPath) &&
			!isTeamsHandler(handlerPath) &&
			!isEmailHandler(handlerPath) &&
			loadHandlerScriptArgs(handlerPath, errorHandlerArgs).then(
				(schema) => (customHandlerSchema = schema)
			)
	})

	$effect(() => {
		handlerPath &&
			isSlackHandler(handlerPath) &&
			loadHandlerScriptArgs(handlerPath, [...errorHandlerArgs, 'slack']).then(
				(schema) => (slackHandlerSchema = schema)
			)
	})

	$effect(() => {
		if (handlerSelected === 'email') {
			handlerPath = hubPaths.emailErrorHandler
		}
	})
</script>

<div class={classNames('space-y-2', noMargin ? '' : 'mt-2')}>
	<ToggleButtonGroup bind:selected={handlerSelected} disabled={!isEditable}>
		{#snippet children({ item })}
			<ToggleButton label="Slack" value="slack" {item} disabled={!isEditable} />
			<ToggleButton label="Teams" value="teams" {item} disabled={!isEditable} />
			<ToggleButton label="Email" value="email" {item} disabled={!isEditable} />
			<ToggleButton
				label="Custom"
				value="custom"
				{item}
				disabled={!isEditable}
				tooltip={customTabTooltip ? 'Custom error handler with script or flow' : undefined}
			/>
		{/snippet}
	</ToggleButtonGroup>

	<div class="flex flex-col gap-6 p-4 rounded-md shadow-sm bg-surface-tertiary">
		{#if handlerSelected === 'custom'}
			<div class="flex flex-col gap-1">
				<div class="flex flex-row">
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
							btnClasses="ml-4 whitespace-nowrap"
							variant="default"
							size="xs"
							href={customScriptTemplate}
							disabled={!isEditable}
							target="_blank"
						>
							Create from template
						</Button>
					{/if}
				</div>
				{#if showScriptHelpText}
					<div class="text-2xs text-secondary">
						Example of error handler scripts can be found on <a
							target="_blank"
							href="{$hubBaseUrlStore}/failures"
						>
							Windmill Hub</a
						>
					</div>
				{/if}
			</div>
			{#if handlerPath}
				<div>
					<p class="font-semibold text-xs mb-1">Extra arguments</p>
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
						<div class="text-xs text-secondary">This error handler takes no extra arguments</div>
					{/if}
				</div>
			{/if}
		{:else if handlerSelected === 'slack'}
			<!-- Slack Connection Status -->
			<SlackConnectionStatus
				isConnected={workspaceConnectedToSlack}
				slackTeamName={slack_team_name}
				mode="workspace"
				onRefresh={loadSlackResources}
			/>

			{#if workspaceConnectedToSlack}
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
			{/if}

			{#if workspaceConnectedToSlack && isSlackHandler(handlerPath)}
				<div class="flex flex-col gap-2">
					{#await import('$lib/components/SchemaForm.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default
							disabled={!$enterpriseLicense}
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

					{#if $enterpriseLicense && isSlackHandler(handlerPath)}
						<Button
							disabled={emptyString(handlerExtraArgs['channel'])}
							wrapperClasses="w-fit"
							variant="default"
							on:click={() => sendSlackMessage(handlerExtraArgs['channel'])}
							unifiedSize="md">Send test message</Button
						>
						{#if connectionTestJob !== undefined}
							<div class="flex items-center gap-2 p-4 rounded-md bg-surface-tertiary">
								<p class="text-normal text-2xs flex items-center gap-4">
									{#if connectionTestJob.in_progress}
										<RotateCw size={14} class="animate-spin" />
										Sending message...
									{:else if connectionTestJob.is_success}
										<CircleCheck size={14} class="text-green-600" />
										Message sent via Windmill job
									{:else}
										<CircleX size={14} class="text-red-700" />
										Message not sent
									{/if}

									<a
										target="_blank"
										href={`${base}/run/${connectionTestJob.uuid}?workspace=${$workspaceStore}`}
										class="inline-flex items-center gap-1"
									>
										{connectionTestJob.uuid}
										<ExternalLink size={12} class="inline-block" />
									</a>
								</p>
							</div>
						{/if}
					{/if}
				</div>
			{:else if workspaceConnectedToSlack == undefined}
				<Loader2 class="animate-spin" size={10} />
			{/if}
		{:else if handlerSelected === 'teams'}
			<!-- Teams Connection Status -->
			<TeamsConnectionStatus
				isConnected={workspaceConnectedToTeams}
				teamsTeamName={teams_team_name}
				mode="workspace"
				onRefresh={loadTeamsResources}
			/>

			{#if workspaceConnectedToTeams}
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
			{/if}

			{#if workspaceConnectedToTeams}
				<div class="flex flex-col gap-2">
					<div class="w-2/3 flex flex-col gap-2">
						<div class="flex flex-row items-center gap-2">
							<p class="text-xs text-emphasis font-semibold">Teams Channel</p>
							<div class="flex-shrink-0">
								<MsTeamsIcon size={14} />
							</div>
						</div>

						<div class="flex flex-row gap-2 items-start">
							<ChannelSelector
								containerClass="flex-grow"
								minWidth="200px"
								placeholder="Search Teams channels"
								teamId={teams_team_guid}
								selectedChannel={handlerExtraArgs['channel']
									? {
											channel_id: handlerExtraArgs['channel'],
											channel_name: handlerExtraArgs['channel_name']
										}
									: undefined}
								onSelectedChannelChange={(channel) => {
									handlerExtraArgs['channel'] = channel?.channel_id
									handlerExtraArgs['channel_name'] = channel?.channel_name
								}}
								onError={(e) => sendUserToast('Failed to load channels: ' + e.message, true)}
							/>
						</div>
					</div>
					{#if $enterpriseLicense && isTeamsHandler(handlerPath) && workspaceConnectedToTeams}
						<Button
							disabled={emptyString(handlerExtraArgs['channel'])}
							btnClasses="w-32 text-center whitespace-nowrap"
							variant="default"
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
				</div>
			{:else if workspaceConnectedToTeams == undefined}
				<Loader2 class="animate-spin" size={10} />
			{/if}
		{:else if handlerSelected === 'email'}
			{#if isCloudHosted()}
				<Alert type="info" title="Email notifications are not available in Cloud">
					Email notifications for trigger failures are only available in self-hosted Windmill
					instances.
				</Alert>
			{:else}
				<SmtpConfigurationStatus {hasSmtpConfig} />

				<div class="flex flex-col gap-2">
					<MultiSelect
						items={[] as { label: string; value: string }[]}
						bind:value={
							() => handlerExtraArgs[EMAIL_RECIPIENTS_KEY] ?? [],
							(recipients) => (handlerExtraArgs[EMAIL_RECIPIENTS_KEY] = recipients)
						}
						placeholder="Enter email addresses..."
						onCreateItem={(email) => {
							const emailRegex = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/
							if (!emailRegex.test(email)) {
								sendUserToast('Invalid email format', true)
								return
							}
							const currentArray = handlerExtraArgs[EMAIL_RECIPIENTS_KEY] ?? []
							handlerExtraArgs[EMAIL_RECIPIENTS_KEY] = [...currentArray, email]
						}}
						class="w-full"
					/>
					{#if handlerExtraArgs[EMAIL_RECIPIENTS_KEY]?.length > 0}
						<span class="text-xs text-secondary">
							{handlerExtraArgs[EMAIL_RECIPIENTS_KEY]?.length} email{handlerExtraArgs[
								EMAIL_RECIPIENTS_KEY
							]?.length === 1
								? ''
								: 's'} configured
						</span>
					{/if}
				</div>
			{/if}
		{/if}
	</div>
</div>
