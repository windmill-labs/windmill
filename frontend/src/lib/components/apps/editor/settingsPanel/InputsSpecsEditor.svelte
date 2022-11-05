<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { faBolt, faLink, faUser } from '@fortawesome/free-solid-svg-icons'
	import type { InputsSpec } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	export let inputSpecs: InputsSpec
	export let componenId: string
</script>

{#each Object.keys(inputSpecs) as inputSpecKey}
	<span class="text-sm font-bold">{inputSpecKey}</span>
	<ToggleButtonGroup bind:selected={inputSpecs[inputSpecKey].type}>
		<ToggleButton position="left" value="static" startIcon={{ icon: faBolt }} size="xs">
			Static
		</ToggleButton>
		<ToggleButton position="center" value="output" startIcon={{ icon: faLink }} size="xs">
			Dynamic
		</ToggleButton>
		<ToggleButton position="right" value="user" startIcon={{ icon: faUser }} size="xs">
			User
		</ToggleButton>
	</ToggleButtonGroup>

	<InputsSpecEditor
		bind:appInputTransform={inputSpecs[inputSpecKey]}
		propKey={inputSpecKey}
		{componenId}
	/>
{/each}
