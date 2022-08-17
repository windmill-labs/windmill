<script lang="ts">
	import { RawScript } from '$lib/gen'

	import { faCode, faRepeat } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from './pickers/FlowScriptPicker.svelte'
	import PickHubScript from './pickers/PickHubScript.svelte'
	import PickScript from './pickers/PickScript.svelte'

	export let shouldDisableLoopCreation: boolean = false
	export let shouldDisableTriggerScripts: boolean = false
	const dispatch = createEventDispatcher()
</script>

<div class="space-y-4">
	{#if !shouldDisableTriggerScripts}
		<div class="text-sm font-bold">Scripts</div>
	{/if}

	<div class="grid sm:grid-col-2 lg:grid-cols-3 gap-4">
		<PickScript on:pick />
		<PickHubScript on:pick />

		<FlowScriptPicker
			label={`Create a for-loop here`}
			disabled={shouldDisableLoopCreation}
			icon={faRepeat}
			iconColor="text-blue-500"
			on:click={() => dispatch('loop')}
		/>

		<FlowScriptPicker
			label={`New Postgres SQL query`}
			icon={faCode}
			iconColor="text-blue-800"
			on:click={() =>
				dispatch('pick', {
					path: 'hub/173/postgresql/execute_query_and_return_results_postgresql'
				})}
		/>

		<FlowScriptPicker
			label="New Python script (3.10)"
			icon={faCode}
			iconColor="text-green-500"
			on:click={() => dispatch('new', { language: RawScript.language.PYTHON3 })}
		/>

		<FlowScriptPicker
			label="New Typescript script (Deno)"
			icon={faCode}
			iconColor="text-blue-800"
			on:click={() => dispatch('new', { language: RawScript.language.DENO, isTrigger: false })}
		/>
	</div>

	{#if !shouldDisableTriggerScripts}
		<div class="text-sm font-bold">Trigger scripts</div>

		<div class="grid sm:grid-col-1 md:grid-col-2 lg:grid-cols-3 gap-4">
			<PickScript isTrigger={true} on:pick />
			<PickHubScript isTrigger={true} on:pick />
			<FlowScriptPicker
				label="New Typescript script (Deno)"
				icon={faCode}
				iconColor="text-blue-800"
				on:click={() => dispatch('new', { language: RawScript.language.DENO, isTrigger: true })}
			/>
		</div>
	{/if}
</div>
