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
	import type { InputConnection, InputType, UploadAppInput } from '../../inputType'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { FunctionSquare, Pen, Plug, Plug2, Upload, User } from 'lucide-svelte'
	import { fieldTypeToTsType } from '../../utils'
	import EvalV2InputEditor from './inputEditor/EvalV2InputEditor.svelte'
	import { Button } from '$lib/components/common'

	export let id: string
	export let componentInput: RichConfiguration
	export let key: string
	export let userInputEnabled: boolean = false
	export let shouldCapitalize: boolean = true
	export let resourceOnly = false
	export let tooltip: string | undefined = undefined
	export let fieldType: InputType
	export let subFieldType: InputType | undefined
	export let format: string | undefined
	export let selectOptions: string[] | undefined
	export let fileUpload: UploadAppInput['fileUpload'] | undefined = undefined
	export let placeholder: string | undefined
	export let customTitle: string | undefined = undefined
	export let displayType: boolean = false
	export let allowTypeChange: boolean = true
	export let shouldFormatExpression: boolean = false

	const { connectingInput, app } = getContext<AppViewerContext>('AppViewerContext')

	let evalV2editor: EvalV2InputEditor
	function applyConnection(connection: InputConnection) {
		const expr = `${connection.componentId}.${connection.path}`
		//@ts-ignore
		componentInput = {
			...componentInput,
			type: 'evalv2',
			expr: expr,
			connections: [{ componentId: connection.componentId, id: connection.path.split('.')[0] }]
		}
		evalV2editor?.setCode(expr)
		$app = $app
	}

	$: if (componentInput == undefined) {
		//@ts-ignore
		componentInput = {
			type: 'static',
			value: undefined
		}
	}
</script>

{#if !(resourceOnly && (fieldType !== 'object' || !format?.startsWith('resource-')))}
	<div class={classNames('flex gap-1', 'flex-col')}>
		<div class="flex justify-between items-end">
			<div class="flex flex-row gap-4 items-center">
				<div class="flex items-center">
					<span class="text-xs font-semibold truncate text-primary">
						{customTitle
							? customTitle
							: shouldCapitalize
							? capitalize(addWhitespaceBeforeCapitals(key))
							: key}
					</span>
					{#if tooltip}
						<Tooltip small>
							{tooltip}
						</Tooltip>
					{/if}
				</div>
				{#if displayType}
					<div class="text-xs text-tertiary mr-1">
						{fieldType === 'array' && subFieldType
							? `${fieldTypeToTsType(subFieldType)}[]`
							: fieldTypeToTsType(fieldType)}
					</div>
				{/if}
			</div>

			<div class={classNames('flex gap-x-2 gap-y-1 justify-end items-center')}>
				{#if componentInput?.type && allowTypeChange}
					<ToggleButtonGroup
						class="h-7"
						bind:selected={componentInput.type}
						on:selected={(e) => {
							if (
								e.detail == 'evalv2' &&
								componentInput['value'] != undefined &&
								(componentInput['expr'] == '' || componentInput['expr'] == undefined)
							) {
								componentInput['expr'] = JSON.stringify(componentInput['value'])
							}

							if (shouldFormatExpression) {
								componentInput['expr'] = JSON.stringify(JSON.parse(componentInput['expr']), null, 4)
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
						{#if componentInput?.type === 'connected'}
							<ToggleButton value="connected" icon={Plug2} iconOnly tooltip="Connect" />
						{/if}
						{#if componentInput?.type === 'eval'}
							<ToggleButton value="eval" icon={FunctionSquare} iconOnly tooltip="Eval Legacy" />
						{/if}
						<ToggleButton value="evalv2" icon={FunctionSquare} iconOnly tooltip="Eval" />
					</ToggleButtonGroup>
					<div>
						<Button
							size="xs"
							variant="border"
							color="light"
							title="Connect"
							on:click={() => {
								$connectingInput = {
									opened: true,
									input: undefined,
									hoveredComponent: undefined,
									onConnect: applyConnection
								}
							}}
							id="schema-plug"
						>
							<Plug size={14} />
						</Button>
					</div>
				{/if}
			</div>
		</div>

		{#if componentInput?.type === 'connected'}
			<ConnectedInputEditor bind:componentInput />
		{:else if componentInput?.type === 'row'}
			<RowInputEditor bind:componentInput />
		{:else if componentInput?.type === 'static'}
			<div class={'w-full flex flex-row-reverse'}>
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
			<EvalInputEditor {id} bind:componentInput />
		{:else if componentInput?.type === 'evalv2'}
			<EvalV2InputEditor field={key} bind:this={evalV2editor} {id} bind:componentInput />
		{:else if componentInput?.type === 'upload'}
			<UploadInputEditor bind:componentInput {fileUpload} />
		{:else if componentInput?.type === 'user'}
			<span class="text-2xs italic text-tertiary">Field's value is set by the user</span>
		{/if}
	</div>
{/if}
