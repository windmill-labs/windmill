<script lang="ts">
	import { RawScript, Script } from '$lib/gen'

	import { faCode, faRepeat } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from '../pickers/FlowScriptPicker.svelte'
	import PickHubScript from '../pickers/PickHubScript.svelte'
	import PickScript from '../pickers/PickScript.svelte'

	export let shouldDisableLoopCreation: boolean = false
	export let shouldDisableTriggerScripts: boolean = false
	export let failureModule: boolean

	const dispatch = createEventDispatcher()
</script>

<div class="space-y-4 p-4">
	{#if !shouldDisableTriggerScripts}
		<div class="text-sm font-bold">Scripts</div>
	{/if}

	<div class="grid sm:grid-col-2 lg:grid-cols-3 gap-4">
		<PickScript kind={failureModule ? Script.kind.FAILURE : Script.kind.SCRIPT} on:pick />
		<PickHubScript kind={failureModule ? Script.kind.FAILURE : Script.kind.SCRIPT} on:pick />

		<FlowScriptPicker
			label={`Create a for-loop here`}
			disabled={shouldDisableLoopCreation}
			icon={faRepeat}
			iconColor="text-blue-500"
			on:click={() => dispatch('loop')}
		/>

		{#if !failureModule}
			<FlowScriptPicker
				label={`New PostgreSQL query`}
				icon={faCode}
				iconColor="text-blue-800"
				on:click={() =>
					dispatch('new', { language: RawScript.language.DENO, kind: 'script', subkind: 'pgsql' })}
			/>
		{/if}

		<FlowScriptPicker
			label="New Python script (3.10)"
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
			label="New Typescript script (Deno)"
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
			label="New Go script"
			icon={faCode}
			iconColor="text-blue-700"
			on:click={() =>
				dispatch('new', {
					language: RawScript.language.GO,
					kind: 'script',
					subkind: failureModule ? 'failure' : 'flow'
				})}
		/>
	</div>

	{#if !shouldDisableTriggerScripts}
		<div class="text-sm font-bold">Trigger scripts</div>

		<div class="grid sm:grid-col-1 md:grid-col-2 lg:grid-cols-3 gap-4">
			<PickScript kind={Script.kind.TRIGGER} on:pick />
			<PickHubScript kind={Script.kind.TRIGGER} on:pick />
			<FlowScriptPicker
				label="New Typescript script (Deno)"
				icon={faCode}
				iconColor="text-blue-800"
				on:click={() => dispatch('new', { language: RawScript.language.DENO, kind: 'trigger' })}
			/>
		</div>
	{/if}
</div>
