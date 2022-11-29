<script lang="ts">
	import { Badge, ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { capitalize, classNames } from '$lib/utils'
	import { faBolt, faLink, faUser } from '@fortawesome/free-solid-svg-icons'
	import type { InputsSpec } from '../../types'
	import { fieldTypeToTsType } from '../../utils'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	export let inputSpecs: InputsSpec
	export let userInputEnabled: boolean = true

	let openedProp: string | undefined = Object.keys(inputSpecs)[0]

	const userTypeKeys = ['value']
	const staticTypeKeys = ['value']
	const dynamicTypeKeys = ['id', 'name']

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
</script>

<div class="w-full flex flex-col gap-2">
	{#each Object.keys(inputSpecs) as inputSpecKey, index (index)}
		{@const input = inputSpecs[inputSpecKey]}
		<div>
			<div
				class={classNames(
					'w-full text-xs font-bold py-1.5 px-2 cursor-pointer transition-all justify-between flex items-center border border-gray-3 rounded-md',
					openedProp === inputSpecKey
						? 'bg-gray-700 hover:bg-gray-900 focus:bg-gray-900 text-white'
						: 'bg-white border-gray-300  hover:bg-gray-100 focus:bg-gray-100 text-gray-700'
				)}
				on:keypress
				on:click={() => {
					if (openedProp === inputSpecKey) {
						openedProp = undefined
					} else {
						openedProp = inputSpecKey
					}
				}}
			>
				{inputSpecKey}
				{#if input?.fieldType}
					<Badge color={openedProp === inputSpecKey ? 'dark-blue' : 'blue'}>
						{capitalize(fieldTypeToTsType(input.fieldType))}
					</Badge>
				{/if}
			</div>
			{#if inputSpecKey === openedProp}
				<div class="flex flex-col w-full gap-2 my-2">
					<ToggleButtonGroup
						bind:selected={inputSpecs[inputSpecKey].type}
						on:selected={({ detail }) => sanitizeInputSpec(detail, inputSpecKey)}
					>
						<ToggleButton position="left" value="static" startIcon={{ icon: faBolt }} size="xs">
							Static
						</ToggleButton>
						<ToggleButton
							position={userInputEnabled ? 'center' : 'right'}
							value="output"
							startIcon={{ icon: faLink }}
							size="xs"
						>
							Dynamic
						</ToggleButton>
						{#if userInputEnabled}
							<ToggleButton position="right" value="user" startIcon={{ icon: faUser }} size="xs">
								User
							</ToggleButton>
						{/if}
					</ToggleButtonGroup>
					<InputsSpecEditor
						bind:appInputTransform={inputSpecs[inputSpecKey]}
						canHide={userInputEnabled}
					/>
				</div>
			{/if}
		</div>
	{/each}
</div>
