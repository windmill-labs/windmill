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
	import { enterpriseLicense } from '$lib/stores'
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
	import { isCloudHosted } from '$lib/cloud'
	import SmtpConfigurationStatus from './common/smtp/SmtpConfigurationStatus.svelte'
	import { SettingService } from '$lib/gen'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { builtinHubPath, type HandlerDraft, type HandlerMode } from './handlerDraft.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		errorOrRecovery: HandlerMode
		isEditable: boolean
		toggleText?: string
		showScriptHelpText?: boolean
		/** Owned by the caller, which restores it on Discard; every tab edits its own slot. */
		draft: HandlerDraft
		customScriptTemplate: string
		customTabTooltip?: import('svelte').Snippet
		noMargin?: boolean
		/** Workspace for handler lookup / settings / test jobs. Defaults to the operating
		 * workspace (see `useOperatingWorkspace`). */
		workspace?: string
		/** Offer the instance critical alert channels as a destination. Workspace-level
		 * error handling only: schedules and triggers have no such setting. */
		showInstanceAlerts?: boolean
	}

	let {
		errorOrRecovery,
		isEditable,
		toggleText = 'Enable',
		showScriptHelpText = false,
		draft = $bindable(),
		customScriptTemplate,
		customTabTooltip,
		noMargin = false,
		workspace = undefined,
		showInstanceAlerts = false
	}: Props = $props()

	let effectiveWorkspace = $derived(workspace ?? $operatingWorkspace)
	// Carry the workspace onto the "create from template" route, so the handler script is
	// created where this handler is saved. `customScriptTemplate` already has a query string.
	let templateHref = $derived(
		effectiveWorkspace
			? `${customScriptTemplate}&workspace=${encodeURIComponent(effectiveWorkspace)}`
			: customScriptTemplate
	)

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

	async function loadSlackResources() {
		const settings = await WorkspaceService.getPublicSettings({ workspace: effectiveWorkspace! })
		if (!emptyString(settings.slack_name) && !emptyString(settings.slack_team_id)) {
			workspaceConnectedToSlack = true
			slack_team_name = settings.slack_name
		} else {
			workspaceConnectedToSlack = false
			slack_team_name = undefined
		}
	}

	async function loadTeamsResources() {
		const settings = await WorkspaceService.getPublicSettings({ workspace: effectiveWorkspace! })
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
			hasSmtpConfig = await SettingService.getSmtpConfigured()
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
			workspace: effectiveWorkspace!,
			requestBody: {
				hub_script_path: draft[platform].path,
				channel: channel,
				test_msg: `This is a notification to test the connection between ${platform} and Windmill workspace '${effectiveWorkspace!}'`
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
					workspace: effectiveWorkspace!,
					id: connectionTestJob!.uuid
				})
				connectionTestJob!.in_progress = false
				connectionTestJob!.is_success = testResult.success
			},
			timeoutCode: async () => {
				try {
					await JobService.cancelQueuedJob({
						workspace: effectiveWorkspace!,
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

	async function loadHandlerScriptArgs(
		p: string,
		kind: 'script' | 'flow',
		defaultArgs: string[] = []
	) {
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
					kind === 'script'
						? await ScriptService.getScriptByPath({ workspace: effectiveWorkspace!, path: p })
						: await FlowService.getFlowByPath({ workspace: effectiveWorkspace!, path: p })
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

	$effect(() => {
		if (effectiveWorkspace) {
			loadSlackResources()
			loadTeamsResources()
		}
		loadSmtpConfiguration()
	})

	$effect(() => {
		const { path, kind } = draft.custom
		path &&
			loadHandlerScriptArgs(path, kind, errorHandlerArgs).then(
				(schema) => (customHandlerSchema = schema)
			)
	})

	$effect(() => {
		const path = draft.slack.path
		path &&
			loadHandlerScriptArgs(path, 'script', [...errorHandlerArgs, 'slack']).then(
				(schema) => (slackHandlerSchema = schema)
			)
	})
</script>

<div class={classNames('space-y-2', noMargin ? '' : 'mt-2')}>
	<ToggleButtonGroup bind:selected={draft.selected} disabled={!isEditable}>
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
			{#if showInstanceAlerts}
				<ToggleButton
					label="Instance alerts"
					value="instance_alerts"
					{item}
					disabled={!isEditable}
					tooltip="Report failures to the instance critical alert channels"
				/>
			{/if}
		{/snippet}
	</ToggleButtonGroup>

	<div class="flex flex-col gap-6 p-4 rounded-md shadow-sm bg-surface-tertiary">
		{#if draft.selected === 'custom'}
			<div class="flex flex-col gap-1">
				<div class="flex flex-row">
					<ScriptPicker
						disabled={!isEditable || !$enterpriseLicense}
						kinds={['script', 'failure']}
						allowFlow={true}
						bind:scriptPath={draft.custom.path}
						bind:itemKind={draft.custom.kind}
						allowRefresh={isEditable}
						workspace={effectiveWorkspace}
						clearable
					/>

					{#if !draft.custom.path}
						<Button
							btnClasses="ml-4 whitespace-nowrap"
							variant="default"
							size="xs"
							href={templateHref}
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
			{#if draft.custom.path}
				<div>
					<p class="font-semibold text-xs mb-1">Extra arguments</p>
					{#await import('$lib/components/SchemaForm.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default
							disabled={!isEditable}
							schema={customHandlerSchema}
							bind:args={draft.custom.args}
							shouldHideNoInputs
							className="text-xs"
						/>
					{/await}
					{#if customHandlerSchema && customHandlerSchema.properties && Object.keys(customHandlerSchema.properties).length === 0}
						<div class="text-xs text-secondary">This error handler takes no extra arguments</div>
					{/if}
				</div>
			{/if}
		{:else if draft.selected === 'slack'}
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
					checked={!!draft.slack.path}
					options={{ right: toggleText }}
					on:change={(e) => {
						draft.slack.path = e.detail ? builtinHubPath(errorOrRecovery, 'slack') : undefined
					}}
				/>
			{/if}

			{#if workspaceConnectedToSlack && draft.slack.path}
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
							bind:args={draft.slack.args}
							shouldHideNoInputs
							compact
							className="text-xs"
						/>
					{/await}

					{#if $enterpriseLicense}
						<Button
							disabled={emptyString(draft.slack.args['channel'])}
							wrapperClasses="w-fit"
							variant="default"
							on:click={() => sendSlackMessage(draft.slack.args['channel'])}
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
										href={`/run/${connectionTestJob.uuid}?workspace=${effectiveWorkspace}`}
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
		{:else if draft.selected === 'teams'}
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
					checked={!!draft.teams.path}
					options={{ right: toggleText }}
					on:change={(e) => {
						draft.teams.path = e.detail ? builtinHubPath(errorOrRecovery, 'teams') : undefined
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
								workspace={effectiveWorkspace}
								teamId={teams_team_guid}
								selectedChannel={draft.teams.args['channel']
									? {
											channel_id: draft.teams.args['channel'],
											channel_name: draft.teams.args['channel_name']
										}
									: undefined}
								onSelectedChannelChange={(channel) => {
									draft.teams.args['channel'] = channel?.channel_id
									draft.teams.args['channel_name'] = channel?.channel_name
								}}
								onError={(e) => sendUserToast('Failed to load channels: ' + e.message, true)}
							/>
						</div>
					</div>
					{#if $enterpriseLicense && draft.teams.path && workspaceConnectedToTeams}
						<Button
							disabled={emptyString(draft.teams.args['channel'])}
							btnClasses="w-32 text-center whitespace-nowrap"
							variant="default"
							on:click={() => sendTeamsMessage(draft.teams.args['channel'] ?? '')}
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
									href={`/run/${connectionTestJob.uuid}?workspace=${effectiveWorkspace}`}
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
		{:else if draft.selected === 'email'}
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
							() => draft.email.args.email_recipients ?? [],
							(recipients) => (draft.email.args.email_recipients = recipients)
						}
						placeholder="Enter email addresses..."
						onCreateItem={(email) => {
							const emailRegex = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/
							if (!emailRegex.test(email)) {
								sendUserToast('Invalid email format', true)
								return
							}
							draft.email.args.email_recipients = [
								...(draft.email.args.email_recipients ?? []),
								email
							]
						}}
						class="w-full"
					/>
					{#if draft.email.args.email_recipients?.length > 0}
						<span class="text-xs text-secondary">
							{draft.email.args.email_recipients.length} email{draft.email.args.email_recipients
								.length === 1
								? ''
								: 's'} configured
						</span>
					{/if}
				</div>
			{/if}
		{:else if draft.selected === 'instance_alerts'}
			<div class="flex flex-col gap-2">
				<span class="text-xs text-secondary">
					Failed jobs are reported to the Slack, Teams and email channels configured at the instance
					level. Those channels are managed in instance settings by a superadmin, not here, and the
					report is sent without adding an entry to the instance critical alert feed. Canceled jobs
					are not reported.
				</span>
				<a class="text-xs w-fit" href="/?workspace=admins#superadmin-settings">
					Configure the instance critical alert channels
				</a>
			</div>
		{/if}
	</div>
</div>
