<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import { faBolt, faLink, faUser } from '@fortawesome/free-solid-svg-icons'
	import type { InputsSpec } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	export let inputSpecs: InputsSpec

	const userTypeKeys = ['schemaProperty', 'defaultValue', 'value']
	const staticTypeKeys = ['visible', 'value', 'fieldType']
	const dynamicTypeKeys = ['id', 'name', 'defaultValue']

	function sanitizeInputSpec(type: 'user' | 'static' | 'output', inputSpecKey: string) {
		const inputSpec = inputSpecs[inputSpecKey]
		if (type === 'user') {
			for (const key of staticTypeKeys) {
				delete inputSpec[key]
			}
			for (const key of dynamicTypeKeys) {
				delete inputSpec[key]
			}
		} else if (type === 'static') {
			for (const key of userTypeKeys) {
				delete inputSpec[key]
			}
			for (const key of dynamicTypeKeys) {
				delete inputSpec[key]
			}
		} else if (type === 'output') {
			for (const key of userTypeKeys) {
				delete inputSpec[key]
			}
			for (const key of staticTypeKeys) {
				delete inputSpec[key]
			}
		}

		inputSpecs[inputSpecKey] = inputSpec
	}

	let openedProp = Object.keys(inputSpecs)[0]
</script>

<div class="w-full flex flex-col gap-4">
	{#each Object.keys(inputSpecs) as inputSpecKey}
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
				<ToggleButtonGroup
					bind:selected={inputSpecs[inputSpecKey].type}
					on:select={(x) => sanitizeInputSpec(x.detail, inputSpecKey)}
				>
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
				<InputsSpecEditor bind:appInputTransform={inputSpecs[inputSpecKey]} canHide />
			</div>
		{/if}
	{/each}
</div>
