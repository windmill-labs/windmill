<script lang="ts">
	import { RawScript, Script } from '$lib/gen'

	import { faCode, faRepeat } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from '../pickers/FlowScriptPicker.svelte'
	import PickHubScript from '../pickers/PickHubScript.svelte'
	import PickScript from '../pickers/PickScript.svelte'

	export let shouldDisableTriggerScripts: boolean = false
	export let shouldDisableLoopCreation: boolean = false

	export let failureModule: boolean

	const dispatch = createEventDispatcher()
</script>

<div class="space-y-4 p-4">
	<div class="text-sm font-bold">Inline script</div>
	<div class="grid sm:grid-col-2 lg:grid-cols-3 gap-4">
		<FlowScriptPicker
			label="Python (3.10)"
			icon={faCode}
			iconColor="text-green-500"
			on:click={() =>
				dispatch('new', {
					language: RawScript.language.PYTHON3,
					kind: 'script',
					subkind: failureModule ? 'failure' : 'flow'
				})}
		/>

		<FlowScriptPicker
			label="Typescript (Deno)"
			icon={faCode}
			iconColor="text-blue-800"
			on:click={() =>
				dispatch('new', {
					language: RawScript.language.DENO,
					kind: 'script',
					subkind: failureModule ? 'failure' : 'flow'
				})}
		/>

		<FlowScriptPicker
			label="Go"
			icon={faCode}
			iconColor="text-blue-700"
			on:click={() =>
				dispatch('new', {
					language: RawScript.language.GO,
					kind: 'script',
					subkind: failureModule ? 'failure' : 'flow'
				})}
		/>

		{#if !failureModule}
			<FlowScriptPicker
				label={`PostgreSQL`}
				icon={faCode}
				iconColor="text-blue-800"
				on:click={() =>
					dispatch('new', { language: RawScript.language.DENO, kind: 'script', subkind: 'pgsql' })}
			/>
		{/if}
	</div>
	<div class="text-sm font-bold">Pre-made script</div>

	<div class="grid sm:grid-col-2 lg:grid-cols-3 gap-4">
		<PickScript
			customText={failureModule ? 'Pick an error handler from your workspace' : undefined}
			kind={failureModule ? Script.kind.FAILURE : Script.kind.SCRIPT}
			on:pick
		/>
		<PickHubScript
			customText={failureModule ? 'Pick an error handler from your workspace' : undefined}
			kind={failureModule ? Script.kind.FAILURE : Script.kind.SCRIPT}
			on:pick
		/>
	</div>

	{#if !shouldDisableTriggerScripts}
		<div class="text-sm font-bold">Trigger script</div>

		<div class="grid sm:grid-col-1 md:grid-col-2 lg:grid-cols-3 gap-4">
			<PickScript customText="Trigger script from workspace" kind={Script.kind.TRIGGER} on:pick />
			<PickHubScript customText="Trigger script from Hub" kind={Script.kind.TRIGGER} on:pick />
			<FlowScriptPicker
				label="New Typescript script (Deno)"
				icon={faCode}
				iconColor="text-blue-800"
				on:click={() => dispatch('new', { language: RawScript.language.DENO, kind: 'trigger' })}
			/>
		</div>
	{/if}

	{#if !failureModule}
		<div class="text-sm font-bold">Approval step</div>
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
				label={`Create a for-loop`}
				icon={faRepeat}
				iconColor="text-blue-500"
				on:click={() => dispatch('loop')}
			/>
		</div>
	{/if}
</div>
