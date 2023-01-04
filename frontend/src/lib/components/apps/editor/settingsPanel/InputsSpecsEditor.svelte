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

	export let inputSpecs: Record<
		string,
		(StaticAppInput | ConnectedAppInput | UserAppInput | RowAppInput) & { onlyStatic?: boolean }
	>
	export let userInputEnabled: boolean = true
	export let staticOnly: boolean = false
	export let shouldCapitalize: boolean = true
	export let rowColumns = false
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(inputSpecs) as inputSpecKey (inputSpecKey)}
			{@const input = inputSpecs[inputSpecKey]}
			<div class="flex flex-col gap-1">
				<div class="flex justify-between items-end gap-1">
					<span class="text-xs font-semibold">
						{shouldCapitalize ? capitalize(inputSpecKey) : inputSpecKey}
					</span>

					<div class="flex gap-2 flex-wrap items-center">
						<Badge color="blue">
							{input.fieldType === 'array' && input.subFieldType
								? `${capitalize(fieldTypeToTsType(input.subFieldType))}[]`
								: capitalize(fieldTypeToTsType(input.fieldType))}
						</Badge>

						{#if !inputSpecs[inputSpecKey].onlyStatic}
							<ToggleButtonGroup bind:selected={inputSpecs[inputSpecKey].type}>
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
										title="From Row"
										position="center"
										value="row"
										startIcon={{ icon: faTableCells }}
										size="xs"
										iconOnly
										disabled={staticOnly}
									/>
								{/if}
								{#if userInputEnabled && (!input.format?.startsWith('resource-') || true)}
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
									title="Connected"
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
