<script lang="ts">
	import { RawScript } from '$lib/gen'

	import { faCode } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import FlowScriptPicker from './pickers/FlowScriptPicker.svelte'
	import PickHubScript from './pickers/PickHubScript.svelte'
	import PickScript from './pickers/PickScript.svelte'

	export let isTrigger: boolean

	const dispatch = createEventDispatcher()
</script>

<div class="columns-2">
	<PickScript {isTrigger} on:pick />
	<PickHubScript {isTrigger} on:pick />
	<FlowScriptPicker
		label="New Typescript {isTrigger ? 'trigger ' : ''}script (Deno)"
		icon={faCode}
		iconColor="text-blue-800"
		on:click={() => dispatch('new', { language: RawScript.language.DENO })}
	/>

	<FlowScriptPicker
		disabled={isTrigger}
		label="New Python {isTrigger ? 'trigger ' : ''}script (3.10)"
		icon={faCode}
		iconColor="text-yellow-500"
		on:click={() => dispatch('new', { language: RawScript.language.PYTHON3 })}
		tooltip={isTrigger
			? 'Python is not supported for trigger scripts yet but is supported for every other steps'
			: undefined}
	/>
</div>
