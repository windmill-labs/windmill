<script lang="ts">
	import { RawScript } from '$lib/gen'

	import { faCode, faRepeat } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from './pickers/FlowScriptPicker.svelte'
	import PickHubScript from './pickers/PickHubScript.svelte'
	import PickScript from './pickers/PickScript.svelte'

	const dispatch = createEventDispatcher()
</script>

<div class="columns-2">
	<PickScript isTrigger={false} on:pick />
	<PickHubScript isTrigger={false} on:pick />
	<PickScript isTrigger={true} on:pick />
	<PickHubScript isTrigger={true} on:pick />
	<FlowScriptPicker
		label={`Create a for-loop here (coming soon)`}
		disabled={true}
		icon={faRepeat}
		iconColor="text-blue-500"
	/>

	<FlowScriptPicker
		label={`New Postgres SQL query`}
		icon={faCode}
		iconColor="text-blue-800"
		on:click={() => dispatch('pick', { path: 'hub/130/execute_custom_query_postgresql' })}
	/>

	<FlowScriptPicker
		label="New Python  script (3.10)"
		icon={faCode}
		iconColor="text-green-500"
		on:click={() => dispatch('new', { language: RawScript.language.PYTHON3 })}
	/>

	<FlowScriptPicker
		label="New Typescript script (Deno)"
		icon={faCode}
		iconColor="text-blue-800"
		on:click={() => dispatch('new', { language: RawScript.language.DENO })}
	/>
</div>
