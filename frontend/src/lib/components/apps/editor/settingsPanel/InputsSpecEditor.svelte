<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import { addWhitespaceBeforeCapitals, capitalize, classNames } from '$lib/utils'
	import { faArrowRight, faPen, faUpload, faUser } from '@fortawesome/free-solid-svg-icons'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Popover from '$lib/components/Popover.svelte'

	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import EvalInputEditor from './inputEditor/EvalInputEditor.svelte'
	import RowInputEditor from './inputEditor/RowInputEditor.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'
	import UploadInputEditor from './inputEditor/UploadInputEditor.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import type { InputType, UploadAppInput } from '../../inputType'

	export let id: string
	export let componentInput: RichConfiguration
	export let key: string
	export let hasRows: boolean = false
	export let userInputEnabled: boolean = false
	export let shouldCapitalize: boolean = true
	export let resourceOnly = false
	export let tooltip: string | undefined = undefined
	export let onlyStatic: boolean = false
	export let fieldType: InputType
	export let subFieldType: InputType | undefined
	export let format: string | undefined
	export let selectOptions: string[] | undefined
	export let fileUpload: UploadAppInput['fileUpload'] | undefined = undefined
	export let placeholder: string | undefined
	export let customTitle: string | undefined = undefined

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	$: if (componentInput == undefined) {
		//@ts-ignore
		componentInput = {
			type: 'static',
			value: undefined
		}
	}
</script>

{#if !(resourceOnly && (fieldType !== 'object' || !format?.startsWith('resource-')))}
	<div
		class={classNames(
			'flex gap-1',
			onlyStatic ? 'flex-row items-center justify-between' : 'flex-col'
		)}
	>
		<div class="flex justify-between items-end gap-1">
			<span class="text-xs font-semibold truncate text-gray-800">
				{customTitle
					? customTitle
					: shouldCapitalize
					? capitalize(addWhitespaceBeforeCapitals(key))
					: key}
				{#if tooltip}
					<Tooltip>
						{tooltip}
					</Tooltip>
				{/if}
			</span>

			<div class={classNames('flex gap-x-2 gap-y-1 flex-wrap justify-end items-center')}>
				{#if !onlyStatic && componentInput?.type && componentInput.type != 'eval'}
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
						{#if userInputEnabled && !format?.startsWith('resource-')}
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
						{#if fileUpload}
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

		{#if componentInput?.type === 'connected'}
			<ConnectedInputEditor bind:componentInput />
		{:else if componentInput?.type === 'row'}
			<RowInputEditor bind:componentInput />
		{:else if componentInput?.type === 'static'}
			<div class={onlyStatic ? 'w-2/3 flex justify-end' : 'w-full'}>
				<StaticInputEditor
					{fieldType}
					{subFieldType}
					{selectOptions}
					{format}
					{placeholder}
					bind:componentInput
				/>
			</div>
		{:else if componentInput?.type === 'eval'}
			<EvalInputEditor {hasRows} {id} bind:componentInput />
		{:else if componentInput?.type === 'upload'}
			<UploadInputEditor bind:componentInput {fileUpload} />
		{:else if componentInput?.type === 'user'}
			<span class="text-2xs italic text-gray-600">Field's value is set by the user</span>
		{/if}
	</div>
{/if}
