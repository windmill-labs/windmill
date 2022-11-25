<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { capitalize, classNames } from '$lib/utils'
	import { faBolt, faLink, faUser } from '@fortawesome/free-solid-svg-icons'
	import type { ComponentInputsSpec } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	export let componentInputSpecs: ComponentInputsSpec

	let openedProp = Object.keys(componentInputSpecs)[0]
</script>

<div class="w-full flex flex-col gap-4">
	{#each Object.keys(componentInputSpecs) as inputSpecKey, index (index)}
		{@const component = componentInputSpecs[inputSpecKey]}
		<div
			class={classNames(
				'w-full text-xs rounded-md py-1.5 px-2 cursor-pointer justify-between flex',
				'bg-gray-100 text-black'
			)}
		>
			{inputSpecKey}
			{#if component.type === 'static' && component.fieldType}
				<Badge color="dark-blue">{capitalize(component.fieldType)}</Badge>
			{/if}
		</div>
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
	{/each}
</div>
