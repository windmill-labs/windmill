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
	export let shouldCapitalize: boolean = true
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(inputSpecs) as inputSpecKey, index (index)}
			{@const input = inputSpecs[inputSpecKey]}
			{#if true}
				<div class="flex flex-col gap-1">
					<div class="flex justify-between items-end gap-1">
						<span class="text-xs font-semibold">
							{shouldCapitalize ? capitalize(inputSpecKey) : inputSpecKey}
						</span>

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
									position="right"
									value="connected"
									startIcon={{ icon: faArrowRight }}
									size="xs"
									iconOnly
									disabled={staticOnly}
								/>
							</ToggleButtonGroup>
						</div>
					</div>

					<InputsSpecEditor
						bind:componentInput={inputSpecs[inputSpecKey]}
						userInputEnabled={userInputEnabled && input.format === undefined}
					/>
				</div>
			{/if}
		{/each}
	</div>
{:else}
	<div class="text-gray-500 text-sm">No inputs</div>
{/if}
