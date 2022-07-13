<script lang="ts">
	import { FlowModuleValue } from '$lib/gen/models/FlowModuleValue'

	import { faCode } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from './pickers/FlowScriptPicker.svelte'
	import PickHubScript from './pickers/PickHubScript.svelte'
	import PickScript from './pickers/PickScript.svelte'

	const dispatch = createEventDispatcher()
</script>

<div class="columns-2">
	<PickScript on:pick />
	<PickHubScript on:pick />
	<FlowScriptPicker
		label="New Typescript script (Deno)"
		icon={faCode}
		iconColor="text-blue-800"
		on:click={() => dispatch('new', { language: FlowModuleValue.language.DENO })}
	/>
	<FlowScriptPicker
		label="New Python script (3.10)"
		icon={faCode}
		iconColor="text-yellow-500"
		on:click={() => dispatch('new', { language: FlowModuleValue.language.PYTHON3 })}
	/>
</div>
