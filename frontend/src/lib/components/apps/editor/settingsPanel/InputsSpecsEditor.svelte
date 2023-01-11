<script lang="ts">
	import { Badge, ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { capitalize } from '$lib/utils'
	import { faArrowRight, faPen, faTableCells, faUser } from '@fortawesome/free-solid-svg-icons'
	import { fieldTypeToTsType } from '../../utils'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '../../inputType'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let inputSpecs: Record<
		string,
		(StaticAppInput | ConnectedAppInput | UserAppInput | RowAppInput) & { onlyStatic?: boolean }
	>
	export let userInputEnabled: boolean = true
	export let staticOnly: boolean = false
	export let shouldCapitalize: boolean = true
	export let rowColumns = false

	const { connectingInput } = getContext<AppEditorContext>('AppEditorContext')
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(inputSpecs) as inputSpecKey (inputSpecKey)}
			{@const input = inputSpecs[inputSpecKey]}
			<div class="flex flex-col gap-1">
				<div class="flex justify-between items-end gap-1">
					<span class="text-sm font-semibold truncate">
						{shouldCapitalize ? capitalize(inputSpecKey) : inputSpecKey}
					</span>

					<div class="flex gap-2 flex-wrap items-center">
						<Badge color="blue">
							{input.fieldType === 'array' && input.subFieldType
								? `${capitalize(fieldTypeToTsType(input.subFieldType))}[]`
								: capitalize(fieldTypeToTsType(input.fieldType))}
						</Badge>

						{#if !inputSpecs[inputSpecKey].onlyStatic}
							<ToggleButtonGroup
								bind:selected={inputSpecs[inputSpecKey].type}
								on:selected={(e) => {
									console.log(inputSpecs[inputSpecKey])
									if (e.detail == 'connected' && !inputSpecs[inputSpecKey]['connection']) {
										$connectingInput = {
											opened: true,
											input: undefined,
											hoveredComponent: undefined
										}
									}
								}}
							>
								<ToggleButton
									title="Static"
									position="left"
									value="static"
									startIcon={{ icon: faPen }}
									size="xs"
									iconOnly
								/>
								{#if rowColumns}
									<ToggleButton
										title="Column"
										position="center"
										value="row"
										startIcon={{ icon: faTableCells }}
										size="xs"
										disabled={staticOnly}
										><Tooltip
											>Use the column name to have the value of the cell be passed to the action</Tooltip
										></ToggleButton
									>
								{/if}
								{#if userInputEnabled && !input.format?.startsWith('resource-')}
									<ToggleButton
										title="User Input"
										position="center"
										value="user"
										startIcon={{ icon: faUser }}
										size="xs"
										iconOnly
										disabled={staticOnly}
									/>
								{/if}
								<ToggleButton
									title="Connect"
									position="right"
									value="connected"
									startIcon={{ icon: faArrowRight }}
									size="xs"
									iconOnly
									disabled={staticOnly}
								/>
							</ToggleButtonGroup>
						{/if}
					</div>
				</div>

				<InputsSpecEditor bind:componentInput={inputSpecs[inputSpecKey]} />
			</div>
		{/each}
	</div>
{:else}
	<div class="text-gray-500 text-sm">No inputs</div>
{/if}
