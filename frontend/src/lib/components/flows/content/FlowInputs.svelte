<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { RawScript, Script } from '$lib/gen'

	import { faCode, faCodeBranch, faRepeat } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from '../pickers/FlowScriptPicker.svelte'
	import PickHubScript from '../pickers/PickHubScript.svelte'
	import PickScript from '../pickers/PickScript.svelte'

	export let failureModule: boolean
	export let shouldDisableTriggerScripts: boolean = false
	const dispatch = createEventDispatcher()
</script>

<div class="space-y-4 p-4">
	<div class="text-sm font-bold">Common script</div>
	<div class="grid sm:grid-col-2 lg:grid-cols-3 gap-4">
		<FlowScriptPicker
			label="Inline Python (3.10)"
			icon={faCode}
			iconColor="text-green-500"
			on:click={() => {
				dispatch('new', {
					language: RawScript.language.PYTHON3,
					kind: 'script',
					subkind: failureModule ? 'failure' : 'flow'
				})
			}}
		/>

		<FlowScriptPicker
			label="Inline Typescript (Deno)"
			icon={faCode}
			iconColor="text-blue-800"
			on:click={() => {
				dispatch('new', {
					language: RawScript.language.DENO,
					kind: 'script',
					subkind: failureModule ? 'failure' : 'flow'
				})
			}}
		/>

		<FlowScriptPicker
			label="Inline Go"
			icon={faCode}
			iconColor="text-blue-700"
			on:click={() => {
				dispatch('new', {
					language: RawScript.language.GO,
					kind: 'script',
					subkind: failureModule ? 'failure' : 'flow'
				})
			}}
		/>

		{#if !failureModule}
			<FlowScriptPicker
				label={`Inline PostgreSQL`}
				icon={faCode}
				iconColor="text-blue-800"
				on:click={() =>
					dispatch('new', { language: RawScript.language.DENO, kind: 'script', subkind: 'pgsql' })}
			/>
		{/if}

		<PickScript
			customText={failureModule ? 'Error Handler from workspace' : undefined}
			kind={failureModule ? Script.kind.FAILURE : Script.kind.SCRIPT}
			on:pick
		/>
		<PickHubScript
			customText={failureModule ? 'Error Handler from Hub' : undefined}
			kind={failureModule ? Script.kind.FAILURE : Script.kind.SCRIPT}
			on:pick
		/>
	</div>

	{#if !shouldDisableTriggerScripts}
		<div class="text-sm font-bold">
			Trigger script
			<Tooltip>
				Used as a first step most commonly with an intenal state and a schedule to watch for changes
				on an external system, compute the diff since last time, set the new state. The diffs are
				then treated one by one with a for-loop.
			</Tooltip>
		</div>

		<div class="grid sm:grid-col-1 md:grid-col-2 lg:grid-cols-3 gap-4">
			<PickScript customText="Trigger script from workspace" kind={Script.kind.TRIGGER} on:pick />
			<PickHubScript customText="Trigger script from Hub" kind={Script.kind.TRIGGER} on:pick />
			<FlowScriptPicker
				label="Inline Typescript (Deno)"
				icon={faCode}
				iconColor="text-blue-800"
				on:click={() => dispatch('new', { language: RawScript.language.DENO, kind: 'trigger' })}
			/>
		</div>
	{/if}
	{#if !failureModule}
		<div class="text-sm font-bold">
			Approval step
			<Tooltip>
				Inlined common scripts can be turned into approval step by changing their suspend settings.
				An approval step will suspend the execution of a flow until it has been approved through the
				resume endpoints or the approval page by and solely by the recipients of those secret urls.
				Use getResumeEndpoints from the wmill client to generate those URLs.
			</Tooltip>
		</div>
		<div class="grid sm:grid-col-1 md:grid-col-2 lg:grid-cols-3 gap-4">
			<PickScript customText="Approval step from workspace" kind={Script.kind.APPROVAL} on:pick />
			<PickHubScript
				customText={'Approval step from the Hub'}
				kind={Script.kind.APPROVAL}
				on:pick
			/>
		</div>

		<div class="text-sm font-bold">Flow primitive</div>

		<div class="grid sm:grid-col-1 md:grid-col-2 lg:grid-cols-3 gap-4">
			<FlowScriptPicker
				label={`Create one branch`}
				icon={faCodeBranch}
				iconColor="text-blue-500"
				on:click={() => dispatch('branchone')}
			/>
			<FlowScriptPicker
				label={`Create all branches`}
				icon={faCodeBranch}
				iconColor="text-blue-500"
				on:click={() => dispatch('branchall')}
			/>

			<FlowScriptPicker
				label={`Create a for-loop`}
				icon={faRepeat}
				iconColor="text-blue-500"
				on:click={() => dispatch('loop')}
			/>
		</div>
	{/if}
</div>
