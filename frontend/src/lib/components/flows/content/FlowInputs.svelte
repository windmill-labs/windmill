<script lang="ts">
	import { Alert } from '$lib/components/common'
	import ToggleHubWorkspace from '$lib/components/ToggleHubWorkspace.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { RawScript, Script } from '$lib/gen'

	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from '../pickers/FlowScriptPicker.svelte'
	import PickHubScript from '../pickers/PickHubScript.svelte'
	import WorkspaceScriptPicker from '../pickers/WorkspaceScriptPicker.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { sendUserToast } from '$lib/toast'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Check, Code, Zap } from 'lucide-svelte'

	export let failureModule: boolean
	export let shouldDisableTriggerScripts: boolean = false
	export let noEditor: boolean
	export let summary: string | undefined = undefined

	const dispatch = createEventDispatcher()
	let kind: 'script' | 'failure' | 'approval' | 'trigger' = failureModule
		? 'failure'
		: summary == 'Trigger'
		? 'trigger'
		: summary == 'Approval'
		? 'approval'
		: 'script'
	let pick_existing: 'workspace' | 'hub' = 'hub'
	let filter = ''
</script>

<div class="p-4 h-full flex flex-col" id="flow-editor-flow-inputs">
	{#if summary == 'Terminate flow'}
		<Alert role="info" title="The flow stops here"
			>This is an identity step with an early stop that has 'true' for expression</Alert
		>
	{:else}{#if !failureModule}
			<div class="center-center">
				<div class="max-w-min">
					<ToggleButtonGroup bind:selected={kind}>
						<ToggleButton
							value="script"
							icon={Code}
							label="Action"
							tooltip="An action script is simply a script that is neither a trigger nor an approval script. Those are the majority of the scripts."
						/>
						{#if !shouldDisableTriggerScripts}
							<ToggleButton
								value="trigger"
								icon={Zap}
								label="Trigger"
								tooltip="Used as a first step most commonly with a state and a schedule to watch for changes on an external system, compute the diff since last time and set the new state. The diffs are then treated one by one with a for-loop."
							/>
						{/if}
						<ToggleButton
							value="approval"
							icon={Check}
							label="Approval"
							tooltip="An approval step will suspend the execution of a flow until it has been approved through the resume endpoints or the approval page by and solely by the recipients of those secret urls."
						/>
					</ToggleButtonGroup>
				</div>
			</div>
		{/if}
		{#if kind == 'trigger'}
			<div class="mt-2" />
			<Alert title="Trigger script automatic schedule" role="info">
				A schedule will be automatically attached to this flow to run every 15 minutes. Adjust
				frequency in 'Settings -> Schedule'</Alert
			>
		{/if}
		<h3 class="pb-2 pt-4">
			Inline new <span class="text-blue-500">{kind == 'script' ? 'action' : kind}</span> script
			<Tooltip documentationLink="https://www.windmill.dev/docs/flows/flow_error_handler">
				Embed a script directly inside a flow instead of saving the script into your workspace for
				reuse. You can always save an inline script to your workspace later.
			</Tooltip>
		</h3>

		{#if noEditor}
			<div
				class="py-0.5 text-2xs {summary == undefined || summary == ''
					? 'text-red-600'
					: 'text-ternary'}"
				>Pick a summary first, it will be used to create a separate file whose name will be derived
				from the summary</div
			>
			<input class="w-full" type="text" bind:value={summary} placeholder="Summary" />
			<div class="pb-2" />
		{/if}
		<div class="flex flex-row">
			<div class="flex flex-row flex-wrap gap-2" id="flow-editor-action-script">
				<FlowScriptPicker
					disabled={noEditor && (summary == undefined || summary == '')}
					label="TypeScript (Deno)"
					lang={Script.language.DENO}
					on:click={() => {
						dispatch('new', {
							language: RawScript.language.DENO,
							kind,
							subkind: 'flow',
							summary
						})
					}}
				/>

				<FlowScriptPicker
					disabled={noEditor && (summary == undefined || summary == '')}
					label="Python"
					lang={Script.language.PYTHON3}
					on:click={() => {
						dispatch('new', {
							language: RawScript.language.PYTHON3,
							kind,
							subkind: 'flow',
							summary
						})
					}}
				/>

				{#if kind != 'approval'}
					<FlowScriptPicker
						disabled={noEditor && (summary == undefined || summary == '')}
						label="Go"
						lang={Script.language.GO}
						on:click={() => {
							dispatch('new', {
								language: RawScript.language.GO,
								kind,
								subkind: 'flow',
								summary
							})
						}}
					/>
				{/if}

				{#if kind == 'script'}
					<FlowScriptPicker
						disabled={noEditor && (summary == undefined || summary == '')}
						label="Bash"
						lang={Script.language.BASH}
						on:click={() => {
							dispatch('new', {
								language: RawScript.language.BASH,
								kind,
								subkind: 'flow',
								summary
							})
						}}
					/>

					<FlowScriptPicker
						disabled={noEditor && (summary == undefined || summary == '')}
						label="REST"
						lang={Script.language.NATIVETS}
						on:click={() => {
							dispatch('new', {
								language: RawScript.language.NATIVETS,
								kind,
								subkind: 'flow',
								summary
							})
						}}
					/>

					{#if !failureModule}
						<FlowScriptPicker
							disabled={noEditor && (summary == undefined || summary == '')}
							label="PostgreSQL"
							lang={Script.language.POSTGRESQL}
							on:click={() => {
								dispatch('new', {
									language: RawScript.language.POSTGRESQL,
									kind,
									subkind: 'flow',
									summary
								})
							}}
						/>
						<FlowScriptPicker
							disabled={noEditor && (summary == undefined || summary == '')}
							label="MySQL"
							lang={Script.language.MYSQL}
							on:click={() => {
								dispatch('new', {
									language: RawScript.language.MYSQL,
									kind,
									subkind: 'flow',
									summary
								})
							}}
						/>
						<FlowScriptPicker
							disabled={noEditor && (summary == undefined || summary == '')}
							label="BigQuery"
							lang={Script.language.BIGQUERY}
							on:click={() => {
								dispatch('new', {
									language: RawScript.language.BIGQUERY,
									kind,
									subkind: 'flow',
									summary
								})
							}}
						/>
						<FlowScriptPicker
							disabled={noEditor && (summary == undefined || summary == '')}
							label="Snowflake"
							lang={Script.language.SNOWFLAKE}
							on:click={() => {
								dispatch('new', {
									language: RawScript.language.SNOWFLAKE,
									kind,
									subkind: 'flow',
									summary
								})
							}}
						/>

						<FlowScriptPicker
							disabled={noEditor && (summary == undefined || summary == '')}
							label="MS SQL Server"
							lang={Script.language.MSSQL}
							on:click={() => {
								dispatch('new', {
									language: RawScript.language.MSSQL,
									kind,
									subkind: 'flow',
									summary
								})
							}}
						/>

						<FlowScriptPicker
							disabled={noEditor && (summary == undefined || summary == '')}
							label="GraphQL"
							lang={Script.language.GRAPHQL}
							on:click={() => {
								dispatch('new', {
									language: RawScript.language.GRAPHQL,
									kind,
									subkind: 'flow',
									summary
								})
							}}
						/>

						<FlowScriptPicker
							disabled={noEditor && (summary == undefined || summary == '')}
							label={`Docker`}
							lang="docker"
							on:click={() => {
								if (isCloudHosted()) {
									sendUserToast(
										'You cannot use Docker scripts on the multi-tenant platform. Use a dedicated instance or self-host windmill instead.',
										true,
										[
											{
												label: 'Learn more',
												callback: () => {
													window.open('https://www.windmill.dev/docs/advanced/docker', '_blank')
												}
											}
										]
									)
									return
								}
								dispatch('new', {
									language: RawScript.language.BASH,
									kind,
									subkind: 'docker',
									summary
								})
							}}
						/>

						<FlowScriptPicker
							disabled={noEditor && (summary == undefined || summary == '')}
							label="PowerShell"
							lang={Script.language.POWERSHELL}
							on:click={() => {
								dispatch('new', {
									language: RawScript.language.POWERSHELL,
									kind,
									subkind: 'flow',
									summary
								})
							}}
						/>

						<!-- <FlowScriptPicker
							label={`MySQL`}
							lang="mysql"
							on:click={() =>
								dispatch('new', { language: RawScript.language.DENO, kind, subkind: 'mysql' })}
						/> -->
					{/if}
				{/if}

				<FlowScriptPicker
					disabled={noEditor && (summary == undefined || summary == '')}
					label="TypeScript (Bun)"
					lang={Script.language.BUN}
					on:click={() => {
						dispatch('new', {
							language: RawScript.language.BUN,
							kind,
							subkind: 'flow',
							summary
						})
					}}
				/>
			</div>
		</div>

		<h3 class="mb-2 mt-6"
			>Use pre-made <span class="text-blue-500">{kind == 'script' ? 'action' : kind}</span> script</h3
		>
		{#if pick_existing == 'hub'}
			<PickHubScript bind:filter {kind} on:pick>
				<ToggleHubWorkspace bind:selected={pick_existing} />
			</PickHubScript>
		{:else}
			<WorkspaceScriptPicker displayLock bind:filter {kind} on:pick>
				<ToggleHubWorkspace bind:selected={pick_existing} />
			</WorkspaceScriptPicker>
		{/if}
	{/if}
</div>
