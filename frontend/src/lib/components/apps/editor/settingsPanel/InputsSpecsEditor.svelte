<script lang="ts">
	import { Badge, ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { capitalize } from '$lib/utils'
	import { faArrowRight, faPen, faUser } from '@fortawesome/free-solid-svg-icons'
	import { fieldTypeToTsType } from '../../utils'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import type { ConnectedAppInput, StaticAppInput, UserAppInput } from '../../inputType'

	export let inputSpecs: Record<string, StaticAppInput | ConnectedAppInput | UserAppInput>
	export let userInputEnabled: boolean = true
	export let staticOnly: boolean = false
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(inputSpecs) as inputSpecKey, index (index)}
			{@const input = inputSpecs[inputSpecKey]}
			{#if true}
				<div class="flex flex-col gap-2">
					<div class="flex justify-between items-center">
						<span class="text-xs font-semibold">{capitalize(inputSpecKey)}</span>

						<div class="flex gap-2 items-center">
							<Badge color="blue">
								{input.fieldType === 'array' && input.subFieldType
									? `${capitalize(fieldTypeToTsType(input.subFieldType))}[]`
									: capitalize(fieldTypeToTsType(input.fieldType))}
							</Badge>

							<ToggleButtonGroup bind:selected={inputSpecs[inputSpecKey].type}>
								<ToggleButton
									position="left"
									value="static"
									startIcon={{ icon: faPen }}
									size="xs"
									iconOnly
								/>
								<ToggleButton
									position={userInputEnabled ? 'center' : 'right'}
									value="connected"
									startIcon={{ icon: faArrowRight }}
									size="xs"
									iconOnly
									disabled={staticOnly}
								/>
								{#if userInputEnabled}
									<ToggleButton
										position="right"
										value="user"
										startIcon={{ icon: faUser }}
										size="xs"
										iconOnly
										disabled={staticOnly && !userInputEnabled}
									/>
								{/if}
							</ToggleButtonGroup>
						</div>
					</div>

					<InputsSpecEditor
						bind:componentInput={inputSpecs[inputSpecKey]}
						canHide={userInputEnabled}
					/>
				</div>
			{/if}
		{/each}
	</div>
{:else}
	<div class="text-gray-500 text-sm">No inputs</div>
{/if}
