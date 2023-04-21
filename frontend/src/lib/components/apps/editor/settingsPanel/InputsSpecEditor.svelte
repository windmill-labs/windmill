<script lang="ts">
	import { addWhitespaceBeforeCapitals, capitalize, classNames } from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import EvalInputEditor from './inputEditor/EvalInputEditor.svelte'
	import RowInputEditor from './inputEditor/RowInputEditor.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'
	import UploadInputEditor from './inputEditor/UploadInputEditor.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import type { InputType, UploadAppInput } from '../../inputType'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Pen, Plug2, Upload, User } from 'lucide-svelte'
	import { fieldTypeToTsType } from '../../utils'

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
	export let displayType: boolean = false

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
		<div class="flex justify-between items-end">
			<div class="flex flex-row gap-4 items-center">
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
				{#if displayType}
					<div class="text-xs text-gray-500">
						{fieldType === 'array' && subFieldType
							? `${fieldTypeToTsType(subFieldType)}[]`
							: fieldTypeToTsType(fieldType)}
					</div>
				{/if}
			</div>

			<div class={classNames('flex gap-x-2 gap-y-1 flex-wrap justify-end items-center')}>
				{#if !onlyStatic && componentInput?.type && componentInput.type != 'eval'}
					<ToggleButtonGroup
						class="h-7"
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
						<ToggleButton value="static" icon={Pen} iconOnly tooltip="Static" />
						{#if userInputEnabled && !format?.startsWith('resource-')}
							<ToggleButton value="user" icon={User} iconOnly tooltip="User Input" />
						{/if}
						{#if fileUpload}
							<ToggleButton value="upload" icon={Upload} iconOnly tooltip="Upload" />
						{/if}
						<ToggleButton value="connected" icon={Plug2} iconOnly tooltip="Connect" />
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
