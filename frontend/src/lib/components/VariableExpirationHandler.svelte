<script lang="ts" module>
	/** Arguments the sweep always sends. Stripped from the extra-args form so an admin is only
	 * asked for what the dispatch cannot supply. */
	export const variableExpirationHandlerArgs = [
		'workspace_id',
		'variable_path',
		'description',
		'value_expires_at',
		'is_secret'
	]
</script>

<script lang="ts">
	import { Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import SlackConnectionStatus from '$lib/components/common/slack/SlackConnectionStatus.svelte'
	import TeamsConnectionStatus from '$lib/components/common/teams/TeamsConnectionStatus.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ChannelSelector from '$lib/components/ChannelSelector.svelte'
	import MsTeamsIcon from '$lib/components/icons/MSTeamsIcon.svelte'

	import type { Schema, SupportedLanguage } from '$lib/common'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
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
	import { untrack } from 'svelte'
	import { builtinHubPath, handlerDraftToStored, type HandlerDraft } from './handlerDraft.svelte'

	interface Props {
		isEditable: boolean
		/** Owned by the caller, which restores it on Discard; every tab edits its own slot. */
		draft: HandlerDraft
		/** Href of the "create a handler" script template, already carrying its query string. */
		customScriptTemplate: string
	}

	let { isEditable, draft = $bindable(), customScriptTemplate }: Props = $props()

	let customHandlerSchema: Schema | undefined = $state()
	let slackHandlerSchema: Schema | undefined = $state()

	let workspaceConnectedToSlack: boolean | undefined = $state(undefined)
	let workspaceConnectedToTeams: boolean | undefined = $state(undefined)
	let slackTeamName: string | undefined = $state(undefined)
	let teamsTeamName: string | undefined = $state(undefined)
	let teamsTeamGuid: string | undefined = $state(undefined)

	let testJob: { uuid: string; is_success: boolean; in_progress: boolean } | undefined = $state()

	let builtinSlot = $derived(
		draft.selected === 'slack' || draft.selected === 'teams' ? draft[draft.selected] : undefined
	)

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

	async function loadHandlerSchema(p: string, kind: 'script' | 'flow', defaultArgs: string[]) {
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
					kind === 'script'
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
		const handler = handlerDraftToStored(draft)
		const submitted = await WorkspaceService.runVariableExpirationTestJob({
			workspace: $workspaceStore!,
			requestBody: {
				handler_path: `${handler.kind}/${handler.path}`,
				extra_args: handler.args
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

	// The result belongs to the destination it was sent from.
	$effect(() => {
		draft.selected
		untrack(() => (testJob = undefined))
	})

	$effect(() => {
		const { path, kind } = draft.custom
		path &&
			loadHandlerSchema(path, kind, variableExpirationHandlerArgs).then(
				(schema) => (customHandlerSchema = schema)
			)
	})

	$effect(() => {
		const path = draft.slack.path
		path &&
			loadHandlerSchema(path, 'script', [...variableExpirationHandlerArgs, 'slack']).then(
				(schema) => (slackHandlerSchema = schema)
			)
	})
</script>

<div class="space-y-2">
	<ToggleButtonGroup bind:selected={draft.selected} disabled={!isEditable}>
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
		{#if draft.selected === 'custom'}
			<div class="flex flex-row gap-2 items-center">
				<ScriptPicker
					disabled={!isEditable}
					bind:scriptPath={draft.custom.path}
					bind:itemKind={draft.custom.kind}
					allowRefresh={isEditable}
					allowFlow
					clearable
				/>
				{#if !draft.custom.path}
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
					{#if customHandlerSchema?.properties && Object.keys(customHandlerSchema.properties).length === 0}
						<div class="text-xs text-secondary">This handler takes no extra arguments</div>
					{/if}
				</div>
			{/if}
		{:else if draft.selected === 'slack'}
			<SlackConnectionStatus
				isConnected={workspaceConnectedToSlack}
				{slackTeamName}
				mode="workspace"
				onRefresh={loadSlackResources}
			/>

			{#if workspaceConnectedToSlack}
				<Toggle
					disabled={!$enterpriseLicense || !isEditable}
					checked={!!draft.slack.path}
					options={{ right: 'Notify a Slack channel when a variable is about to expire' }}
					on:change={(e) => {
						draft.slack.path = e.detail ? builtinHubPath('variableExpiration', 'slack') : undefined
					}}
				/>
			{/if}

			{#if workspaceConnectedToSlack && draft.slack.path}
				<div class="flex flex-col gap-2">
					{#await import('$lib/components/SchemaForm.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default
							disabled={!$enterpriseLicense || !isEditable}
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
				</div>
			{:else if workspaceConnectedToSlack === undefined}
				<Loader2 class="animate-spin" size={14} />
			{/if}
		{:else if draft.selected === 'teams'}
			<TeamsConnectionStatus
				isConnected={workspaceConnectedToTeams}
				{teamsTeamName}
				mode="workspace"
				onRefresh={loadTeamsResources}
			/>

			{#if workspaceConnectedToTeams}
				<Toggle
					disabled={!$enterpriseLicense || !isEditable}
					checked={!!draft.teams.path}
					options={{ right: 'Notify a Teams channel when a variable is about to expire' }}
					on:change={(e) => {
						draft.teams.path = e.detail ? builtinHubPath('variableExpiration', 'teams') : undefined
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
						selectedChannel={draft.teams.args.channel
							? {
									channel_id: draft.teams.args.channel,
									channel_name: draft.teams.args.channel_name
								}
							: undefined}
						onSelectedChannelChange={(channel) => {
							draft.teams.args.channel = channel?.channel_id
							draft.teams.args.channel_name = channel?.channel_name
						}}
						onError={(e) => sendUserToast('Failed to load channels: ' + e.message, true)}
					/>
				</div>
			{:else if workspaceConnectedToTeams === undefined}
				<Loader2 class="animate-spin" size={14} />
			{/if}
		{/if}

		{#if builtinSlot?.path && isEditable && $enterpriseLicense}
			<!-- `-mt-4`: belongs with the channel field above, not a full card section away. -->
			<div class="flex flex-col gap-2 -mt-4">
				<Button
					disabled={emptyString(builtinSlot.args.channel)}
					wrapperClasses="w-fit"
					variant="default"
					unifiedSize="md"
					on:click={sendTestMessage}
				>
					Send test message
				</Button>
				<p class="text-2xs text-secondary">
					Runs the handler now with the channel selected above, against a placeholder variable, as
					g/variable_expiration_handler.
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
								href={`/run/${testJob.uuid}?workspace=${$workspaceStore}`}
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
	</div>
</div>
