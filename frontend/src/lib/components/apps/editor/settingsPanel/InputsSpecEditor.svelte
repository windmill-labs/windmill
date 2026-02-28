<script lang="ts">
	import { addWhitespaceBeforeCapitals, capitalize, classNames } from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import EvalInputEditor from './inputEditor/EvalInputEditor.svelte'
	import RowInputEditor from './inputEditor/RowInputEditor.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'
	import { Button } from '$lib/components/common'
	import UploadInputEditor from './inputEditor/UploadInputEditor.svelte'
	import { getContext, createEventDispatcher } from 'svelte'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import type { InputConnection, InputType, UploadAppInput } from '../../inputType'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import {
		FunctionSquare,
		Loader2,
		Pen,
		Plug2,
		Upload,
		UploadCloud,
		User,
		Pipette
	} from 'lucide-svelte'
	import { fieldTypeToTsType } from '../../utils'
	import EvalV2InputEditor from './inputEditor/EvalV2InputEditor.svelte'
	import ConnectionButton from '$lib/components/common/button/ConnectionButton.svelte'

	import Toggle from '$lib/components/Toggle.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'

	interface Props {
		id: string
		componentInput: RichConfiguration
		key: string
		userInputEnabled?: boolean
		shouldCapitalize?: boolean
		resourceOnly?: boolean
		tooltip?: string | undefined
		fieldType: InputType
		subFieldType: InputType | undefined
		format: string | undefined
		selectOptions: string[] | undefined
		fileUpload?: UploadAppInput['fileUpload'] | undefined
		fileUploadS3?: UploadAppInput['fileUploadS3'] | undefined
		placeholder: string | undefined
		customTitle?: string | undefined
		displayType?: boolean
		allowTypeChange?: boolean
		shouldFormatExpression?: boolean
		fixedOverflowWidgets?: boolean
		loading?: boolean
		acceptSelf?: boolean
		recomputeOnInputChanged?: boolean
		showOnDemandOnlyToggle?: boolean
		documentationLink?: string | undefined
		securedContext?: boolean
		disabled?: boolean
	}

	let {
		id,
		componentInput = $bindable(),
		key,
		userInputEnabled = false,
		shouldCapitalize = true,
		resourceOnly = false,
		tooltip = undefined,
		fieldType,
		subFieldType,
		format,
		selectOptions,
		fileUpload = undefined,
		fileUploadS3 = undefined,
		placeholder,
		customTitle = undefined,
		displayType = false,
		allowTypeChange = true,
		disabled = false,
		shouldFormatExpression = false,
		fixedOverflowWidgets = true,
		loading = false,
		acceptSelf = false,
		recomputeOnInputChanged = true,
		showOnDemandOnlyToggle = true,
		documentationLink = undefined,
		securedContext = false
	}: Props = $props()

	$effect.pre(() => {
		if (componentInput == undefined) {
			//@ts-ignore
			componentInput = {
				type: 'static',
				value: undefined
			}
		}
	})

	const { connectingInput, app, workspace } = getContext<AppViewerContext>('AppViewerContext')

	const dispatch = createEventDispatcher()

	let evalV2editor: EvalV2InputEditor | undefined = $state()
	let s3FilePicker: S3FilePicker | undefined = $state()
	let s3PickerSelection: { s3: string; storage?: string } | undefined = $state(undefined)
	let s3FolderPrefix: string = $state('')
	let s3FileUploadRawMode = $state(componentInput?.type == 'uploadS3' && !!componentInput.value?.s3)
	let s3JsonEditor: SimpleEditor | undefined = $state()

	function updateSelectedS3File() {
		if (s3PickerSelection) {
			if (componentInput.type === 'uploadS3') {
				componentInput.value = {
					...s3PickerSelection
				}
				s3JsonEditor?.setCode(JSON.stringify(s3PickerSelection, null, 2))
			}
			s3FileUploadRawMode = true
		}
	}

	function applyConnection(connection: InputConnection) {
		const expr = `${connection.componentId}.${connection.path}`
		//@ts-ignore
		componentInput = {
			...componentInput,
			type: 'evalv2',
			expr: expr,
			connections: [
				{ componentId: connection.componentId, id: connection.path.split('.')[0].split('[')[0] }
			]
		}
		evalV2editor?.setCode(expr)
		$app = $app
	}

	function closeConnection() {
		dispatch('closeConnection')
		$connectingInput = {
			opened: false,
			hoveredComponent: undefined,
			input: undefined,
			onConnect: () => {}
		}
	}

	function openConnection() {
		dispatch('openConnection')
		$connectingInput = {
			opened: true,
			input: undefined,
			hoveredComponent: undefined,
			onConnect: applyConnection
		}
	}
</script>

{#if !(resourceOnly && (fieldType !== 'object' || !format?.startsWith('resource-')))}
	<div class={classNames('flex gap-1 flex-col group')}>
		<div class="flex justify-between items-end">
			<div class="flex flex-row gap-4 items-center">
				<div class="flex items-center">
					<span class="!text-2xs font-semibold text-ellipsis text-primary">
						{customTitle
							? customTitle
							: shouldCapitalize
								? capitalize(addWhitespaceBeforeCapitals(key))
								: key}
					</span>
					{#if loading}
						<Loader2 size={14} class="animate-spin ml-2" />
					{/if}
					{#if tooltip}
						<Tooltip small {documentationLink}>
							{@html tooltip}
						</Tooltip>
					{/if}
				</div>
				{#if displayType}
					<div class="text-xs text-primary mr-1">
						{fieldType === 'array' && subFieldType
							? `${fieldTypeToTsType(subFieldType)}[]`
							: fieldTypeToTsType(fieldType)}
					</div>
				{/if}
			</div>

			<div class={classNames('flex gap-x-2 gap-y-1 justify-end items-center')}>
				{#if componentInput?.type && allowTypeChange !== false}
					<ConnectionButton
						small
						{closeConnection}
						{openConnection}
						isOpen={!!$connectingInput.opened}
						btnWrapperClasses={'h-6 w-8 opacity-0 group-hover:opacity-100 transition-opacity'}
						id="schema-plug-{key}"
					/>
					<ToggleButtonGroup
						bind:selected={componentInput.type}
						on:selected={(e) => {
							if (
								e.detail == 'evalv2' &&
								componentInput['value'] != undefined &&
								(componentInput['expr'] == '' || componentInput['expr'] == undefined)
							) {
								componentInput['expr'] = JSON.stringify(componentInput['value'], null, 2)
							} else if (fileUploadS3 && fieldType === 'text' && e.detail != 'uploadS3') {
								componentInput['value'] = ''
							} else if (e.detail == 'uploadS3') {
								s3FileUploadRawMode = false
								componentInput['value'] = { s3: '' }
							}

							if (shouldFormatExpression) {
								componentInput['expr'] = JSON.stringify(JSON.parse(componentInput['expr']), null, 4)
							}
						}}
					>
						{#snippet children({ item })}
							<ToggleButton small value="static" icon={Pen} iconOnly tooltip="Static" {item} />
							{#if userInputEnabled}
								<ToggleButton small value="user" icon={User} iconOnly tooltip="User Input" {item} />
							{/if}
							{#if fileUpload}
								<ToggleButton small value="upload" icon={Upload} iconOnly tooltip="Upload" {item} />
							{/if}
							{#if fileUploadS3}
								<ToggleButton
									value="uploadS3"
									icon={UploadCloud}
									iconOnly
									small
									tooltip="Upload S3"
									{item}
								/>
							{/if}
							{#if componentInput?.type === 'connected'}
								<ToggleButton
									value="connected"
									icon={Plug2}
									iconOnly
									small
									tooltip="Connect"
									{item}
								/>
							{/if}
							{#if componentInput?.type === 'eval'}
								<ToggleButton
									value="eval"
									icon={FunctionSquare}
									iconOnly
									small
									tooltip="Eval Legacy"
									{item}
								/>
							{/if}
							<ToggleButton
								value="evalv2"
								icon={FunctionSquare}
								iconOnly
								small
								tooltip="Eval"
								{item}
							/>
						{/snippet}
					</ToggleButtonGroup>
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
					{id}
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
			<EvalV2InputEditor
				{acceptSelf}
				field={key}
				bind:this={evalV2editor}
				{id}
				bind:componentInput
				{fixedOverflowWidgets}
				{recomputeOnInputChanged}
				{showOnDemandOnlyToggle}
				{securedContext}
				{disabled}
			/>
		{:else if componentInput?.type === 'upload'}
			<UploadInputEditor bind:componentInput {fileUpload} />
		{:else if componentInput?.type === 'uploadS3'}
			<div class="flex flex-col w-full gap-1">
				<Toggle
					class="flex justify-end"
					bind:checked={s3FileUploadRawMode}
					size="xs"
					options={{ left: 'Raw S3 object input' }}
				/>
				{#if s3FileUploadRawMode}
					{#await import('$lib/components/JsonEditor.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default
							code={JSON.stringify(componentInput.value ?? { s3: '' }, null, 2)}
							bind:value={componentInput.value}
							bind:editor={s3JsonEditor}
						/>
					{/await}
				{:else}
					<input
						type="text"
						placeholder="S3 Folder prefix"
						bind:value={s3FolderPrefix}
						aria-label="S3 Folder prefix"
					/>
					<UploadInputEditor
						bind:componentInput
						fileUpload={fileUploadS3}
						s3={true}
						{workspace}
						prefix={s3FolderPrefix}
						bind:s3FileUploadRawMode
					/>
				{/if}
				<Button
					variant="default"
					size="xs"
					btnClasses="mt-1"
					on:click={() => {
						s3PickerSelection = undefined
						s3FilePicker?.open?.()
					}}
					startIcon={{ icon: Pipette }}
				>
					Choose an existing file
				</Button>
			</div>
			<S3FilePicker
				bind:this={s3FilePicker}
				folderOnly={false}
				onSelectAndClose={(selected) => {
					s3PickerSelection = selected
					updateSelectedS3File()
				}}
				readOnlyMode={false}
				regexFilter={/\.(png|jpg|jpeg|svg|webp)$/i}
			/>
		{:else if componentInput?.type === 'user'}
			<span class="text-2xs italic text-primary">Field's value is set by the user</span>
		{/if}
		{#if (componentInput?.type === 'evalv2' || componentInput?.type === 'connected' || componentInput?.type === 'user') && ((fieldType == 'object' && format?.startsWith('resource-') && format !== 'resource-s3_object') || fieldType == 'resource')}
			<div class="flex flex-row items-center">
				<Toggle
					size="xs"
					bind:checked={componentInput.allowUserResources}
					options={{
						left: 'static resource select only',
						right: 'resources from users allowed'
					}}
				/>
				<Tooltip
					>Apps are executed on behalf of publishers and by default cannot access viewer's
					resources. If the resource passed here as a reference does not come from a static
					'Resource Select' component (which will be whitelisted by the auto-generated policy), you
					need to toggle this.</Tooltip
				>
			</div>
		{/if}
	</div>
{/if}
