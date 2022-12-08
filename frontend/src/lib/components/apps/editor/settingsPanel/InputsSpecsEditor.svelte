<script lang="ts">
	import { Badge, ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { capitalize, classNames } from '$lib/utils'
	import { faBolt, faLink, faUser } from '@fortawesome/free-solid-svg-icons'
	import type { AppInputs } from '../../inputType'
	import { fieldTypeToTsType, sanitizeInputSpec } from '../../utils'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	export let inputSpecs: AppInputs
	export let userInputEnabled: boolean = true
	export let staticOnly: boolean = true

	let openedProp: string | undefined = inputSpecs ? Object.keys(inputSpecs)[0] : undefined
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-2">
		{#each Object.keys(inputSpecs) as inputSpecKey, index (index)}
			{@const input = inputSpecs[inputSpecKey]}
			<div>
				<div
					class={classNames(
						'w-full text-xs font-bold py-1.5 px-2 cursor-pointer transition-all justify-between flex items-center border border-gray-3 rounded-md',
						'bg-white border-gray-300  hover:bg-gray-100 focus:bg-gray-100 text-gray-700',
						openedProp === inputSpecKey ? 'outline outline-gray-500 outline-offset-1' : ''
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
						{#if staticOnly}
							<ToggleButtonGroup bind:selected={inputSpecs[inputSpecKey].type}>
								<ToggleButton position="left" value="static" startIcon={{ icon: faBolt }} size="xs">
									Static
								</ToggleButton>
								<ToggleButton
									position={userInputEnabled ? 'center' : 'right'}
									value="connected"
									startIcon={{ icon: faLink }}
									size="xs"
								>
									Connect
								</ToggleButton>
								{#if userInputEnabled}
									<ToggleButton
										position="right"
										value="user"
										startIcon={{ icon: faUser }}
										size="xs"
									>
										User
									</ToggleButton>
								{/if}
							</ToggleButtonGroup>
						{/if}
						<InputsSpecEditor
							bind:componentInput={inputSpecs[inputSpecKey]}
							canHide={userInputEnabled}
						/>
					</div>
				{/if}
			</div>
		{/each}
	</div>
{:else}
	<div class="text-gray-500 text-sm">No inputs</div>
{/if}
