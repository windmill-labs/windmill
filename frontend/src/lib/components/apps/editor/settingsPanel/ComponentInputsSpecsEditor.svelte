<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import { faBolt, faLink, faUser } from '@fortawesome/free-solid-svg-icons'
	import type { ComponentInputsSpec } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	export let componentInputSpecs: ComponentInputsSpec

	let openedProp = Object.keys(componentInputSpecs)[0]
</script>

<div class="w-full flex flex-col gap-4">
	{#each Object.keys(componentInputSpecs) as inputSpecKey}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div
			class={classNames(
				'w-full text-xs font-bold  border rounded-md py-1 px-2 cursor-pointer hover:bg-gray-800 hover:text-white transition-all',
				openedProp !== inputSpecKey ? 'bg-gray-200 ' : 'bg-gray-600 text-gray-300'
			)}
			on:click={() => {
				openedProp = inputSpecKey
			}}
		>
			{inputSpecKey}
		</div>
		{#if inputSpecKey === openedProp}
			<div class="flex flex-col w-full gap-2">
				<ToggleButtonGroup bind:selected={componentInputSpecs[inputSpecKey].type}>
					<ToggleButton position="left" value="static" startIcon={{ icon: faBolt }} size="xs">
						Static
					</ToggleButton>
					<ToggleButton position="right" value="output" startIcon={{ icon: faLink }} size="xs">
						Dynamic
					</ToggleButton>
				</ToggleButtonGroup>
				<InputsSpecEditor bind:appInputTransform={componentInputSpecs[inputSpecKey]} />
			</div>
		{/if}
	{/each}
</div>
