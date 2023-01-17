<script lang="ts">
	import { Badge, ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { addWhitespaceBeforeCapitals, capitalize } from '$lib/utils'
	import { faArrowRight, faPen, faTableCells, faUser } from '@fortawesome/free-solid-svg-icons'
	import { fieldTypeToTsType } from '../../utils'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, BaseAppComponent } from '../../types'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Popover from '$lib/components/Popover.svelte'

	export let id: string
	export let inputSpecs: BaseAppComponent['configuration']
	export let userInputEnabled: boolean = true
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
						{shouldCapitalize
							? capitalize(addWhitespaceBeforeCapitals(inputSpecKey))
							: inputSpecKey}
						{#if input.tooltip}
							<Tooltip>
								{input.tooltip}
							</Tooltip>
						{/if}
					</span>

					<div class="flex gap-x-2 gap-y-1 flex-wrap justify-end items-center">
						<Badge color="blue">
							{input.fieldType === 'array' && input.subFieldType
								? `${capitalize(fieldTypeToTsType(input.subFieldType))}[]`
								: capitalize(fieldTypeToTsType(input.fieldType))}
						</Badge>

						{#if !inputSpecs[inputSpecKey].onlyStatic && inputSpecs[inputSpecKey].type != 'eval'}
							<ToggleButtonGroup
								bind:selected={inputSpecs[inputSpecKey].type}
								on:selected={(e) => {
									if (e.detail == 'connected' && !inputSpecs[inputSpecKey]['connection']) {
										$connectingInput = {
											opened: true,
											input: undefined,
											hoveredComponent: undefined
										}
									}
								}}
							>
								<Popover placement="bottom" notClickable disapperTimoout={0}>
									<ToggleButton
										position="left"
										value="static"
										startIcon={{ icon: faPen }}
										size="xs"
										iconOnly
									/>
									<svelte:fragment slot="text">Static</svelte:fragment>
								</Popover>
								{#if rowColumns}
									<Popover placement="bottom" notClickable disapperTimoout={0}>
										<ToggleButton
											position="center"
											value="row"
											startIcon={{ icon: faTableCells }}
											size="xs"
										>
											<Tooltip scale={0.6} placement="top-end" wrapperClass="center-center">
												Use the column name to have the value of the cell be passed to the action
											</Tooltip>
										</ToggleButton>
										<svelte:fragment slot="text">Column</svelte:fragment>
									</Popover>
								{/if}
								{#if userInputEnabled && !input.format?.startsWith('resource-')}
									<Popover placement="bottom" notClickable disapperTimoout={0}>
										<ToggleButton
											position="center"
											value="user"
											startIcon={{ icon: faUser }}
											size="xs"
											iconOnly
										/>
										<svelte:fragment slot="text">User Input</svelte:fragment>
									</Popover>
								{/if}
								<Popover placement="bottom" notClickable disapperTimoout={0}>
									<ToggleButton
										position="right"
										value="connected"
										startIcon={{ icon: faArrowRight }}
										size="xs"
										iconOnly
									/>
									<svelte:fragment slot="text">Connect</svelte:fragment>
								</Popover>
							</ToggleButtonGroup>
						{/if}
					</div>
				</div>

				<InputsSpecEditor
					hasRows={rowColumns}
					{id}
					bind:componentInput={inputSpecs[inputSpecKey]}
				/>
			</div>
		{/each}
	</div>
{:else}
	<div class="text-gray-500 text-sm">No inputs</div>
{/if}
