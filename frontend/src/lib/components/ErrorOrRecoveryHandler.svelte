<script lang="ts">
	import { Alert, Button, Tab, Tabs } from '$lib/components/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import type { Schema, SupportedLanguage } from '$lib/common'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { emptySchema, emptyString, sendUserToast, tryEvery } from '$lib/utils'
	import { JobService, Script, ScriptService, WorkspaceService } from '$lib/gen'
	import { inferArgs } from '$lib/infer'

	import { CheckCircle2, Loader2, RotateCw, XCircle } from 'lucide-svelte'

	const slackRecoveryHandler = 'hub/2430/slack/schedule-recovery-handler-slack'
	const slackHandlerScriptPath = 'hub/6512/workspace-or-schedule-error-handler-slack'

	export let errorOrRecovery: 'error' | 'recovery'
	export let isEditable: boolean
	export let slackToggleText: string = 'enable'
	export let showScriptHelpText: boolean = false
	export let handlerSelected: 'custom' | 'slack'
	export let handlersOnlyForEe: string[]

	export let handlerPath: string | undefined
	export let handlerExtraArgs: Record<string, any>

	export let customInitialScriptPath: string | undefined
	export let customScriptTemplate: string
	export let customHandlerKind: 'flow' | 'script' = 'script'
	let customHandlerSchema: Schema | undefined

	let slackHandlerSchema: Schema | undefined
	let workspaceConnectedToSlack: boolean | undefined = undefined
	let slackConnectionTestJob:
		| { uuid: string; is_success: boolean; in_progress: boolean }
		| undefined

	async function loadSlackResources() {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		if (!emptyString(settings.slack_name) && !emptyString(settings.slack_team_id)) {
			workspaceConnectedToSlack = true
		} else {
			workspaceConnectedToSlack = false
		}
	}

	async function sendSlackMessage(channel: string): Promise<void> {
		let submitted_job = await WorkspaceService.runSlackMessageTestJob({
			workspace: $workspaceStore!,
			requestBody: {
				hub_script_path: handlerPath,
				channel: channel,
				test_msg: `This is a notification to test the connection between Slack and Windmill workspace '${$workspaceStore!}'`
			}
		})
		slackConnectionTestJob = {
			uuid: submitted_job.job_uuid!,
			in_progress: true,
			is_success: false
		}
		tryEvery({
			tryCode: async () => {
				const testResult = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
					id: slackConnectionTestJob!.uuid
				})
				slackConnectionTestJob!.in_progress = false
				slackConnectionTestJob!.is_success = testResult.success
			},
			timeoutCode: async () => {
				try {
					await JobService.cancelQueuedJob({
						workspace: $workspaceStore!,
						id: slackConnectionTestJob!.uuid,
						requestBody: {
							reason: 'Slack message not sent after after 5s'
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

	async function loadHandlerScriptArgs(p: string, defaultArgs: string[] = []) {
		try {
			let schema: Schema | undefined = emptySchema()
			if (p.startsWith('hub/')) {
				const hubScript = await ScriptService.getHubScriptByPath({
					path: p
				})

				if (hubScript.schema?.properties) {
					schema = hubScript.schema
				} else {
					await inferArgs(hubScript.language as SupportedLanguage, hubScript.content ?? '', schema)
				}
			} else {
				const script = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: p })
				schema = script.schema as Schema
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
		if (scriptPath === undefined) {
			return false
		}
		if (errorOrRecovery == 'error') {
			return (
				scriptPath.startsWith('hub/') &&
				scriptPath.endsWith('/workspace-or-schedule-error-handler-slack')
			)
		} else {
			return (
				scriptPath.startsWith('hub/') && scriptPath.endsWith('/schedule-recovery-handler-slack')
			)
		}
	}

	$: {
		if ($workspaceStore) {
			loadSlackResources()
		}
	}

	$: handlerPath &&
		!isSlackHandler(handlerPath) &&
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
			'success_started_at'
		]).then((schema) => (customHandlerSchema = schema))

	$: handlerPath &&
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
			'slack'
		]).then((schema) => (slackHandlerSchema = schema))
</script>

<div>
	<Tabs bind:selected={handlerSelected} class="mt-2 mb-4">
		{#if $enterpriseLicense}
			<Tab value="slack">Slack</Tab>
			<Tab value="custom">
				Custom
				<slot name="custom-tab-tooltip" />
			</Tab>
		{:else}
			<Tab value="custom">
				Custom
				<slot name="custom-tab-tooltip" />
			</Tab>
			<Tab value="slack">Slack {handlersOnlyForEe.includes('slack') ? '(ee only)' : ''}</Tab>
		{/if}
	</Tabs>
</div>

{#if handlerSelected === 'custom'}
	<div class="flex flex-row mb-2">
		<ScriptPicker
			disabled={!isEditable}
			initialPath={customInitialScriptPath}
			kinds={[Script.kind.SCRIPT, Script.kind.FAILURE]}
			allowFlow={true}
			bind:scriptPath={handlerPath}
			bind:itemKind={customHandlerKind}
			allowRefresh
		/>

		{#if handlerPath === undefined}
			<Button
				btnClasses="ml-4 mt-2"
				color="dark"
				size="xs"
				href={customScriptTemplate}
				target="_blank">Create from template</Button
			>
		{/if}
	</div>
	{#if showScriptHelpText}
		<div class="text-xs">
			Example of error handler scripts can be found on <a
				target="_blank"
				href="https://hub.windmill.dev/failures"
			>
				Windmill Hub</a
			>
		</div>
	{/if}
	{#if handlerPath}
		<p class="font-semibold text-sm mt-4 mb-2">Extra arguments</p>
		<SchemaForm
			disabled={!isEditable}
			schema={customHandlerSchema}
			bind:args={handlerExtraArgs}
			shouldHideNoInputs
			class="text-xs"
		/>
		{#if customHandlerSchema && customHandlerSchema.properties && Object.keys(customHandlerSchema.properties).length === 0}
			<div class="text-xs texg-gray-700">This error handler takes no extra arguments</div>
		{/if}
	{/if}
{:else if handlerSelected === 'slack'}
	<span class="w-full flex mb-3">
		<Toggle
			disabled={!$enterpriseLicense || !isEditable}
			checked={isSlackHandler(handlerPath)}
			options={{ right: slackToggleText }}
			on:change={async (e) => {
				if (e.detail && errorOrRecovery === 'error') {
					handlerPath = slackHandlerScriptPath
				} else if (e.detail && errorOrRecovery === 'recovery') {
					handlerPath = slackRecoveryHandler
				} else {
					handlerPath = undefined
				}
			}}
		/>
	</span>
	{#if workspaceConnectedToSlack}
		<SchemaForm
			disabled={!$enterpriseLicense || !isSlackHandler(handlerPath)}
			schema={slackHandlerSchema}
			schemaSkippedValues={['slack']}
			schemaFieldTooltip={{
				channel: 'Slack channel name without the "#" - example: "windmill-alerts"'
			}}
			bind:args={handlerExtraArgs}
			shouldHideNoInputs
			class="text-xs"
		/>
	{:else if workspaceConnectedToSlack == undefined}
		<Loader2 class="animate-spin" />
	{/if}
	{#if $enterpriseLicense && isSlackHandler(handlerPath)}
		{#if workspaceConnectedToSlack == false}
			<Alert type="error" title="Workspace not connected to Slack">
				<div class="flex flex-row gap-x-1 w-full items-center">
					<p class="text-clip grow min-w-0">
						The workspace needs to be connected to Slack to use this feature. You can <a
							target="_blank"
							href="/workspace_settings?tab=slack">configure it here</a
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
			{#if slackConnectionTestJob !== undefined}
				<p class="text-normal text-2xs mt-1 gap-2">
					{#if slackConnectionTestJob.in_progress}
						<RotateCw size={14} />
					{:else if slackConnectionTestJob.is_success}
						<CheckCircle2 size={14} class="text-green-600" />
					{:else}
						<XCircle size={14} class="text-red-700" />
					{/if}
					Message sent via Windmill job
					<a
						target="_blank"
						href={`/run/${slackConnectionTestJob.uuid}?workspace=${$workspaceStore}`}
					>
						{slackConnectionTestJob.uuid}
					</a>
				</p>
			{/if}
		{/if}
	{/if}
{/if}
