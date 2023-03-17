<script lang="ts">
	import { Badge, ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { addWhitespaceBeforeCapitals, capitalize } from '$lib/utils'
	import { faArrowRight, faPen, faUpload, faUser } from '@fortawesome/free-solid-svg-icons'
	import { fieldTypeToTsType } from '../../utils'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Popover from '$lib/components/Popover.svelte'

	import type {
		ConnectedAppInput,
		EvalAppInput,
		RowAppInput,
		StaticAppInput,
		UploadAppInput,
		UserAppInput
	} from '../../inputType'
	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import EvalInputEditor from './inputEditor/EvalInputEditor.svelte'
	import RowInputEditor from './inputEditor/RowInputEditor.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'
	import UploadInputEditor from './inputEditor/UploadInputEditor.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'

	export let id: string
	export let componentInput: (
		| StaticAppInput
		| ConnectedAppInput
		| UserAppInput
		| RowAppInput
		| EvalAppInput
		| UploadAppInput
	) & { tooltip?: string; onlyStatic?: boolean }
	export let key: string
	export let hasRows: boolean = false
	export let userInputEnabled: boolean = false
	export let shouldCapitalize: boolean = true
	export let resourceOnly = false

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')
</script>

{#if !(resourceOnly && (componentInput.fieldType !== 'object' || !componentInput.format?.startsWith('resource-')))}
	<div class="flex flex-col gap-1">
		<div class="flex justify-between items-end gap-1">
			<span class="text-sm font-semibold truncate">
				{shouldCapitalize ? capitalize(addWhitespaceBeforeCapitals(key)) : key}
				{#if componentInput.tooltip}
					<Tooltip>
						{componentInput.tooltip}
					</Tooltip>
				{/if}
			</span>

			<div class="flex gap-x-2 gap-y-1 flex-wrap justify-end items-center">
				<Badge color="blue">
					{componentInput.fieldType === 'array' && componentInput.subFieldType
						? `${capitalize(fieldTypeToTsType(componentInput.subFieldType))}[]`
						: capitalize(fieldTypeToTsType(componentInput.fieldType))}
				</Badge>

				{#if !componentInput.onlyStatic && componentInput.type != 'eval'}
					<ToggleButtonGroup
						bind:selected={componentInput.type}
						on:selected={(e) => {
							if (e.detail == 'connected' && !componentInput['connection']) {
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
						{#if userInputEnabled && !componentInput.format?.startsWith('resource-')}
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
						{#if 'fileUpload' in componentInput}
							<Popover placement="bottom" notClickable disapperTimoout={0}>
								<ToggleButton
									position="center"
									value="upload"
									startIcon={{ icon: faUpload }}
									size="xs"
									iconOnly
								/>
								<svelte:fragment slot="text">Upload</svelte:fragment>
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

		{#if componentInput.type === 'connected'}
			<ConnectedInputEditor bind:componentInput />
		{:else if componentInput.type === 'row'}
			<RowInputEditor bind:componentInput />
		{:else if componentInput.type === 'static'}
			<StaticInputEditor bind:componentInput />
		{:else if componentInput.type === 'eval'}
			<EvalInputEditor {hasRows} {id} bind:componentInput />
		{:else if componentInput.type === 'upload'}
			<UploadInputEditor bind:componentInput />
		{:else if componentInput.type === 'user'}
			<span class="text-2xs italic text-gray-6f00">Field's value is set by the user</span>
		{/if}
	</div>
{/if}
