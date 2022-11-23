<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { RawScript, Script } from '$lib/gen'

	import { faBolt, faBuilding, faCheck, faCode } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from '../pickers/FlowScriptPicker.svelte'
	import PickHubScript from '../pickers/PickHubScript.svelte'
	import WorkspaceScriptPicker from '../pickers/WorkspaceScriptPicker.svelte'

	export let failureModule: boolean
	export let shouldDisableTriggerScripts: boolean = false
	const dispatch = createEventDispatcher()
	let kind: 'script' | 'failure' | 'approval' | 'trigger' = failureModule ? 'failure' : 'script'
	let pick_existing: 'workspace' | 'hub' = 'hub'
</script>

<div class="space-y-4 p-4 h-full flex flex-col">
	{#if !failureModule}
		<div class="center-center">
			<div class="max-w-min">
				<ToggleButtonGroup bind:selected={kind}>
					<ToggleButton position="left" value="script" size="sm" startIcon={{ icon: faCode }}>
						Common
					</ToggleButton>
					{#if !shouldDisableTriggerScripts}
						<ToggleButton position="center" value="trigger" size="sm" startIcon={{ icon: faBolt }}>
							Trigger <Tooltip>
								Used as a first step most commonly with a state and a schedule to watch for changes
								on an external system, compute the diff since last time, set the new state. The
								diffs are then treated one by one with a for-loop.
							</Tooltip>
						</ToggleButton>
					{/if}
					<ToggleButton position="right" value="approval" size="sm" startIcon={{ icon: faCheck }}>
						Approval <Tooltip>
							An approval step will suspend the execution of a flow until it has been approved
							through the resume endpoints or the approval page by and solely by the recipients of
							those secret urls. Use getResumeEndpoints from the wmill client to generate those
							URLs.
						</Tooltip>
					</ToggleButton>
				</ToggleButtonGroup>
			</div>
		</div>
	{/if}
	<h3>
		Inline new
		<Tooltip>
			Embed a script directly inside a flow instead of saving the script into your workspace for
			reuse. You can always save an inline script later.
		</Tooltip>
	</h3>
	<div class="flex flex-row flex-wrap gap-2">
		<FlowScriptPicker
			label="Typescript"
			lang={Script.language.DENO}
			on:click={() => {
				dispatch('new', {
					language: RawScript.language.DENO,
					kind,
					subkind: 'flow'
				})
			}}
		/>

		<FlowScriptPicker
			label="Python"
			lang={Script.language.PYTHON3}
			on:click={() => {
				dispatch('new', {
					language: RawScript.language.PYTHON3,
					kind,
					subkind: kind
				})
			}}
		/>

		{#if kind != 'approval'}
			<FlowScriptPicker
				label="Go"
				lang={Script.language.GO}
				on:click={() => {
					dispatch('new', {
						language: RawScript.language.GO,
						kind,
						subkind: 'flow'
					})
				}}
			/>
		{/if}

		{#if kind == 'script'}
			<FlowScriptPicker
				label="Bash"
				lang={Script.language.BASH}
				on:click={() => {
					dispatch('new', {
						language: RawScript.language.BASH,
						kind,
						subkind: 'flow'
					})
				}}
			/>

			{#if !failureModule}
				<FlowScriptPicker
					label={`PostgreSQL`}
					lang="pgsql"
					on:click={() =>
						dispatch('new', { language: RawScript.language.DENO, kind, subkind: 'pgsql' })}
				/>
			{/if}
		{/if}
	</div>

	<h3 class="pt-4">Use pre-made</h3>
	<div class="max-w-min">
		<ToggleButtonGroup bind:selected={pick_existing}>
			<ToggleButton position="left" value="hub" size="xs">
				<WindmillIcon white={pick_existing == 'hub'} height="16px" width="16px" /><div
					class="mr-1"
				/>Hub
			</ToggleButton>
			<ToggleButton position="right" value="workspace" size="xs" startIcon={{ icon: faBuilding }}>
				Workspace
			</ToggleButton>
		</ToggleButtonGroup>
	</div>
	{#if pick_existing == 'hub'}
		<PickHubScript {kind} on:pick />
	{:else}
		<WorkspaceScriptPicker {kind} on:pick />
	{/if}
</div>
