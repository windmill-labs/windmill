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
	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'
	import { faRotate, faRotateRight, faTimes } from '@fortawesome/free-solid-svg-icons'

	export let isEditable: boolean
	export let slackToggleText: string = 'enable'
	export let showScriptHelpText: boolean = false
	export let handlerSelected: 'custom' | 'slack'
	export let handlersOnlyForEe: string[]

	export let handlerPath: string | undefined
	export let handlerExtraArgs: Record<string, any>

	export let customScriptTemplate: string
	export let customHandlerKind: 'flow' | 'script' = 'script'
	let customHandlerSchema: Schema | undefined

	export let slackHandlerScriptPath: string
	let slackHandlerSchema: Schema | undefined
	let workspaceConnectedToSlack: boolean
	let slackConnectionTestJob:
		| { uuid: string; is_success: boolean; in_progress: boolean }
		| undefined

	async function loadSlackResources() {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		if (!emptyString(settings.slack_name) && !emptyString(settings.slack_team_id)) {
			workspaceConnectedToSlack = true
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

	$: {
		if ($workspaceStore) {
			loadSlackResources()
		}
	}

	$: handlerPath &&
		handlerPath !== slackHandlerScriptPath &&
		loadHandlerScriptArgs(handlerPath, [
			'path',
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

	$: loadHandlerScriptArgs(slackHandlerScriptPath, [
		'path',
		'is_flow',
		'schedule_path',
		'error',
		'error_started_at',
		'failed_times',
		'started_at',
		'success_times',
		'success_result',
		'success_started_at'
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
			initialPath={handlerPath}
			kind={Script.kind.SCRIPT}
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
			checked={handlerPath === slackHandlerScriptPath}
			options={{ right: slackToggleText }}
			on:change={async (e) => {
				handlerPath = e.detail ? slackHandlerScriptPath : undefined
			}}
		/>
	</span>
	{#if workspaceConnectedToSlack}
		<SchemaForm
			disabled={!$enterpriseLicense || handlerPath !== slackHandlerScriptPath}
			schema={slackHandlerSchema}
			schemaSkippedValues={['slack']}
			schemaFieldTooltip={{
				channel: 'Slack channel name without the "#" - example: "windmill-alerts"'
			}}
			bind:args={handlerExtraArgs}
			shouldHideNoInputs
			class="text-xs"
		/>
	{/if}
	{#if handlerPath === slackHandlerScriptPath && enterpriseLicense}
		{#if !workspaceConnectedToSlack}
			<Alert type="error" title="Workspace not connected to Slack">
				<div class="flex flex-row gap-x-1 w-full items-center">
					<p class="text-clip grow min-w-0">
						The workspace needs to be connected to Slack to use this feature. You can <a
							target="_blank"
							href="/workspace_settings?tab=slack">configure it here</a
						>.
					</p>
					<Button variant="border" color="light" on:click={loadSlackResources}>
						<Icon scale={0.8} data={faRotateRight} />
					</Button>
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
						<Icon scale={0.8} data={faRotate} class="mr-1" />
					{:else if slackConnectionTestJob.is_success}
						<Icon scale={0.8} data={check} class="mr-1 text-green-600" />
					{:else}
						<Icon scale={0.8} data={faTimes} class="mr-1 text-red-700" />
					{/if}
					Message sent via Windmill job
					<a
						target="_blank"
						href={`/run/${slackConnectionTestJob.uuid}?workspace=${$workspaceStore}`}
						>{slackConnectionTestJob.uuid}</a
					>
				</p>
			{/if}
		{/if}
	{/if}
{/if}
