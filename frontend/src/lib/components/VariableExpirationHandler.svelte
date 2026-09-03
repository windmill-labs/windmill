<script lang="ts" module>
	import { hubPaths } from '$lib/hub'

	/** Arguments the sweep always sends. Stripped from the extra-args form so an admin is only
	 * asked for what the dispatch cannot supply. */
	export const variableExpirationHandlerArgs = [
		'workspace_id',
		'variable_path',
		'description',
		'value_expires_at',
		'is_secret'
	]

	/** Built-ins are recognised by path suffix, not by hub id: the id is assigned when the
	 * script is pushed to the hub and differs per environment, while the slug is ours. */
	export const slackVariableExpirationPathEnding = '/variable-expiration-handler-slack'
	export const teamsVariableExpirationPathEnding = '/variable-expiration-handler-teams'

	export type VariableExpirationHandlerType = 'slack' | 'teams' | 'custom'

	export function getVariableExpirationHandlerType(
		path: string | undefined
	): VariableExpirationHandlerType {
		if (!path?.startsWith('hub/')) {
			return 'custom'
		}
		if (path.endsWith(slackVariableExpirationPathEnding)) {
			return 'slack'
		}
		if (path.endsWith(teamsVariableExpirationPathEnding)) {
			return 'teams'
		}
		return 'custom'
	}
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
	import MsTeamsIcon from '$lib/components/icons/MSTeamsIcon.svelte'

	import type { Schema, SupportedLanguage } from '$lib/common'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, emptyString, sendUserToast, tryEvery } from '$lib/utils'
	import {
		FlowService,
		JobService,
		type Script,
		ScriptService,
		WorkspaceService,
		type Flow
	} from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { CircleCheck, CircleX, ExternalLink, Loader2, RotateCw } from 'lucide-svelte'

	interface Props {
		isEditable: boolean
		handlerSelected: VariableExpirationHandlerType
		handlerPath: string | undefined
		handlerExtraArgs: Record<string, any>
		handlerKind: 'flow' | 'script'
		/** Href of the "create a handler" script template, already carrying its query string. */
		customScriptTemplate: string
	}

	let {
		isEditable,
		handlerSelected = $bindable('custom'),
		handlerPath = $bindable(),
		handlerExtraArgs = $bindable(),
		handlerKind = $bindable('script'),
		customScriptTemplate
	}: Props = $props()

	const CHANNEL_KEY = 'channel'
	const CHANNEL_NAME_KEY = 'channel_name'

	let customHandlerSchema: Schema | undefined = $state()
	let slackHandlerSchema: Schema | undefined = $state()

	let workspaceConnectedToSlack: boolean | undefined = $state(undefined)
	let workspaceConnectedToTeams: boolean | undefined = $state(undefined)
	let slackTeamName: string | undefined = $state(undefined)
	let teamsTeamName: string | undefined = $state(undefined)
	let teamsTeamGuid: string | undefined = $state(undefined)

	let testJob: { uuid: string; is_success: boolean; in_progress: boolean } | undefined = $state()

	let isSlackHandler = $derived(getVariableExpirationHandlerType(handlerPath) === 'slack')
	let isTeamsHandler = $derived(getVariableExpirationHandlerType(handlerPath) === 'teams')

	async function loadSlackResources() {
		const settings = await WorkspaceService.getPublicSettings({ workspace: $workspaceStore! })
		workspaceConnectedToSlack =
			!emptyString(settings.slack_name) && !emptyString(settings.slack_team_id)
		slackTeamName = workspaceConnectedToSlack ? settings.slack_name : undefined
	}

	async function loadTeamsResources() {
		const settings = await WorkspaceService.getPublicSettings({ workspace: $workspaceStore! })
		workspaceConnectedToTeams =
			!emptyString(settings.teams_team_name) && !emptyString(settings.teams_team_id)
		teamsTeamName = workspaceConnectedToTeams ? settings.teams_team_name : undefined
		teamsTeamGuid = workspaceConnectedToTeams ? settings.teams_team_guid : undefined
	}

	async function loadHandlerSchema(p: string, defaultArgs: string[]) {
		try {
			let schema: Schema | undefined = emptySchema()
			if (p.startsWith('hub/')) {
				const hubScript = await ScriptService.getHubScriptByPath({ path: p })
				if ((hubScript.schema as any)?.properties) {
					schema = hubScript.schema as any
				} else {
					await inferArgs(hubScript.language as SupportedLanguage, hubScript.content ?? '', schema)
				}
			} else {
				let scriptOrFlow: Script | Flow =
					handlerKind === 'script'
						? await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: p })
						: await FlowService.getFlowByPath({ workspace: $workspaceStore!, path: p })
				schema = scriptOrFlow.schema as Schema
			}
			if (schema?.properties) {
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

	async function sendTestMessage() {
		const submitted = await WorkspaceService.runVariableExpirationTestJob({
			workspace: $workspaceStore!,
			requestBody: {
				handler_path: `${isSlackHandler || isTeamsHandler ? 'script' : handlerKind}/${handlerPath}`,
				extra_args: handlerExtraArgs
			}
		})
		testJob = { uuid: submitted.job_uuid!, in_progress: true, is_success: false }
		tryEvery({
			tryCode: async () => {
				const result = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
					id: testJob!.uuid
				})
				testJob!.in_progress = false
				testJob!.is_success = result.success
			},
			timeoutCode: async () => {
				try {
					await JobService.cancelQueuedJob({
						workspace: $workspaceStore!,
						id: testJob!.uuid,
						requestBody: { reason: 'Test message not sent after 10s' }
					})
				} catch (err) {
					console.error(err)
				}
			},
			interval: 500,
			timeout: 10000
		})
	}

	$effect(() => {
		if ($workspaceStore) {
			loadSlackResources()
			loadTeamsResources()
		}
	})

	// The Slack built-in reads the workspace bot token as a resource. Derived from the selected
	// handler rather than typed by the admin, so it is stripped before the unsaved-changes check.
	$effect(() => {
		handlerExtraArgs['slack'] = isSlackHandler ? '$res:f/slack_bot/bot_token' : undefined
	})

	// Remembered per destination so flipping between tabs to compare them does not discard a
	// channel that was already picked.
	let lastHandlerSelected: VariableExpirationHandlerType | undefined = $state(undefined)
	let channelCache = $state({
		slack: undefined as string | undefined,
		teams: undefined as string | undefined
	})
	let pathCache: Partial<Record<VariableExpirationHandlerType, string | undefined>> = $state({})
	$effect(() => {
		if (lastHandlerSelected !== handlerSelected && lastHandlerSelected !== undefined) {
			if (lastHandlerSelected !== 'custom') {
				channelCache[lastHandlerSelected] = handlerExtraArgs[CHANNEL_KEY]
			}
			pathCache[lastHandlerSelected] = handlerPath

			if (handlerSelected === 'custom') {
				delete handlerExtraArgs[CHANNEL_KEY]
				delete handlerExtraArgs[CHANNEL_NAME_KEY]
			} else {
				handlerExtraArgs[CHANNEL_KEY] = channelCache[handlerSelected] ?? ''
			}
			handlerPath = pathCache[handlerSelected]
			testJob = undefined
		}
		lastHandlerSelected = handlerSelected
	})

	$effect(() => {
		handlerPath &&
			!isSlackHandler &&
			!isTeamsHandler &&
			loadHandlerSchema(handlerPath, variableExpirationHandlerArgs).then(
				(schema) => (customHandlerSchema = schema)
			)
	})

	$effect(() => {
		handlerPath &&
			isSlackHandler &&
			loadHandlerSchema(handlerPath, [...variableExpirationHandlerArgs, 'slack']).then(
				(schema) => (slackHandlerSchema = schema)
			)
	})
</script>

<div class="space-y-2">
	<ToggleButtonGroup bind:selected={handlerSelected} disabled={!isEditable}>
		{#snippet children({ item })}
			<ToggleButton label="Slack" value="slack" {item} disabled={!isEditable} />
			<ToggleButton label="Teams" value="teams" {item} disabled={!isEditable} />
			<ToggleButton
				label="Custom"
				value="custom"
				{item}
				disabled={!isEditable}
				tooltip="Run your own script or flow"
			/>
		{/snippet}
	</ToggleButtonGroup>

	<div class="flex flex-col gap-6 p-4 rounded-md shadow-sm bg-surface-tertiary">
		{#if handlerSelected === 'custom'}
			<div class="flex flex-row gap-2 items-center">
				<ScriptPicker
					disabled={!isEditable}
					bind:scriptPath={handlerPath}
					bind:itemKind={handlerKind}
					allowRefresh={isEditable}
					allowFlow
					clearable
				/>
				{#if !handlerPath}
					<Button
						variant="default"
						unifiedSize="sm"
						btnClasses="whitespace-nowrap"
						href={customScriptTemplate}
						disabled={!isEditable}
						target="_blank"
					>
						Create a handler
					</Button>
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
					{#if customHandlerSchema?.properties && Object.keys(customHandlerSchema.properties).length === 0}
						<div class="text-xs text-secondary">This handler takes no extra arguments</div>
					{/if}
				</div>
			{/if}
		{:else if handlerSelected === 'slack'}
			<SlackConnectionStatus
				isConnected={workspaceConnectedToSlack}
				{slackTeamName}
				mode="workspace"
				onRefresh={loadSlackResources}
			/>

			{#if workspaceConnectedToSlack}
				<Toggle
					disabled={!isEditable}
					checked={isSlackHandler}
					options={{ right: 'Notify a Slack channel when a variable is about to expire' }}
					on:change={async (e) => {
						handlerPath = e.detail ? hubPaths.slackVariableExpirationHandler : undefined
					}}
				/>
			{/if}

			{#if workspaceConnectedToSlack && isSlackHandler}
				<div class="flex flex-col gap-2">
					{#await import('$lib/components/SchemaForm.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default
							disabled={!isEditable}
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
				</div>
			{:else if workspaceConnectedToSlack === undefined}
				<Loader2 class="animate-spin" size={14} />
			{/if}
		{:else if handlerSelected === 'teams'}
			<TeamsConnectionStatus
				isConnected={workspaceConnectedToTeams}
				{teamsTeamName}
				mode="workspace"
				onRefresh={loadTeamsResources}
			/>

			{#if workspaceConnectedToTeams}
				<Toggle
					disabled={!isEditable}
					checked={isTeamsHandler}
					options={{ right: 'Notify a Teams channel when a variable is about to expire' }}
					on:change={async (e) => {
						handlerPath = e.detail ? hubPaths.teamsVariableExpirationHandler : undefined
					}}
				/>

				<div class="w-2/3 flex flex-col gap-2">
					<div class="flex flex-row items-center gap-2">
						<p class="text-xs text-emphasis font-semibold">Teams channel</p>
						<MsTeamsIcon size={14} />
					</div>
					<ChannelSelector
						containerClass="flex-grow"
						minWidth="200px"
						placeholder="Search Teams channels"
						workspace={$workspaceStore}
						teamId={teamsTeamGuid}
						selectedChannel={handlerExtraArgs[CHANNEL_KEY]
							? {
									channel_id: handlerExtraArgs[CHANNEL_KEY],
									channel_name: handlerExtraArgs[CHANNEL_NAME_KEY]
								}
							: undefined}
						onSelectedChannelChange={(channel) => {
							handlerExtraArgs[CHANNEL_KEY] = channel?.channel_id
							handlerExtraArgs[CHANNEL_NAME_KEY] = channel?.channel_name
						}}
						onError={(e) => sendUserToast('Failed to load channels: ' + e.message, true)}
					/>
				</div>
			{:else if workspaceConnectedToTeams === undefined}
				<Loader2 class="animate-spin" size={14} />
			{/if}
		{/if}

		{#if (isSlackHandler || isTeamsHandler) && isEditable}
			<div class="flex flex-col gap-2">
				<Button
					disabled={emptyString(handlerExtraArgs[CHANNEL_KEY])}
					wrapperClasses="w-fit"
					variant="default"
					unifiedSize="md"
					on:click={sendTestMessage}
				>
					Send test message
				</Button>
				<p class="text-2xs text-secondary">
					Runs the handler now against a placeholder variable, as g/variable_expiration_handler.
					Save the handler first: the test runs what is stored.
				</p>
				{#if testJob !== undefined}
					<div class="flex items-center gap-2 p-4 rounded-md bg-surface-tertiary">
						<p class="text-normal text-2xs flex items-center gap-4">
							{#if testJob.in_progress}
								<RotateCw size={14} class="animate-spin" />
								Sending message...
							{:else if testJob.is_success}
								<CircleCheck size={14} class="text-green-600" />
								Message sent via Windmill job
							{:else}
								<CircleX size={14} class="text-red-700" />
								Message not sent
							{/if}
							<a
								target="_blank"
								href={`${base}/run/${testJob.uuid}?workspace=${$workspaceStore}`}
								class="inline-flex items-center gap-1"
							>
								{testJob.uuid}
								<ExternalLink size={12} class="inline-block" />
							</a>
						</p>
					</div>
				{/if}
			</div>
		{/if}

		{#if handlerSelected !== 'custom' && workspaceConnectedToSlack === false && handlerSelected === 'slack'}
			<Alert type="info" title="Connect Slack first">
				Connect this workspace to Slack to notify a channel without writing a script.
			</Alert>
		{/if}
	</div>
</div>
