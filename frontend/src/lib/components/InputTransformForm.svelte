<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { InputCat } from '$lib/utils'
	import { getContext } from 'svelte'

	import ArgInput from './ArgInput.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import DynamicInputHelpBox from './flows/content/DynamicInputHelpBox.svelte'
	import type { PropPickerWrapperContext } from './flows/propPicker/PropPickerWrapper.svelte'
	import { codeToStaticTemplate, getDefaultExpr } from './flows/utils'
	import SimpleEditor from './SimpleEditor.svelte'
	import { Button } from './common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	import type VariableEditor from './VariableEditor.svelte'
	import type ItemPicker from './ItemPicker.svelte'
	import type { InputTransform } from '$lib/gen'
	import TemplateEditor from './TemplateEditor.svelte'
	import { setInputCat as computeInputCat, isCodeInjection } from '$lib/utils'
	import { FunctionSquare, Plug } from 'lucide-svelte'
	import { getResourceTypes } from './resourceTypesStore'
	import type { FlowCopilotContext } from './copilot/flow'
	import StepInputGen from './copilot/StepInputGen.svelte'
	import type { PickableProperties } from './flows/previousResults'

	export let schema: Schema
	export let arg: InputTransform | any
	export let argName: string
	export let extraLib: string = 'missing extraLib'
	export let inputCheck: boolean = true
	export let previousModuleId: string | undefined
	export let pickForField: string | undefined = undefined
	export let variableEditor: VariableEditor | undefined = undefined
	export let itemPicker: ItemPicker | undefined = undefined
	export let noDynamicToggle = false
	export let argExtra: Record<string, any> = {}
	export let pickableProperties: PickableProperties | undefined = undefined
	export let enableAi = false

	let monaco: SimpleEditor | undefined = undefined
	let monacoTemplate: TemplateEditor | undefined = undefined
	let argInput: ArgInput | undefined = undefined

	$: inputCat = computeInputCat(
		schema.properties[argName].type,
		schema.properties[argName].format,
		schema.properties[argName].items?.type,
		schema.properties[argName].enum,
		schema.properties[argName].contentEncoding
	)

	let propertyType = getPropertyType(arg)

	const { shouldUpdatePropertyType, exprsToSet } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	function setExpr() {
		const newArg = $exprsToSet?.[argName]
		if (newArg) {
			if (newArg.type === 'javascript') {
				propertyType = 'javascript'
				arg = {
					type: 'javascript',
					expr: newArg.expr
				}
				monaco?.setCode(arg.expr)
			}
			// copilot only sets javascript so static case is not handled
		}
		exprsToSet?.set({
			...$exprsToSet,
			[argName]: undefined
		})
	}

	$: $exprsToSet?.[argName] && setExpr()

	function updatePropertyType() {
		propertyType = $shouldUpdatePropertyType?.[argName] || 'static'
		shouldUpdatePropertyType?.set({
			...$shouldUpdatePropertyType,
			[argName]: undefined
		})
	}

	$: $shouldUpdatePropertyType?.[argName] &&
		arg?.type === $shouldUpdatePropertyType?.[argName] &&
		updatePropertyType()

	function getPropertyType(arg: InputTransform | any): 'static' | 'javascript' {
		let type: 'static' | 'javascript' = arg?.type ?? 'static'

		if (
			type == 'javascript' &&
			isStaticTemplate(inputCat) &&
			(arg?.expr?.length === 0 || arg?.expr?.[0] === '`')
		) {
			const newValue = codeToStaticTemplate(arg.expr)
			if (newValue) {
				type = 'static'
				arg.value = newValue
			}
		}

		return type
	}

	function setPropertyType(rawValue: string) {
		if (!arg) {
			return
		}

		if (isCodeInjection(rawValue)) {
			arg.expr = getDefaultExpr(
				argName,
				previousModuleId,
				`\`${rawValue.toString().replaceAll('`', '\\`')}\``
			)
			arg.type = 'javascript'
			propertyType = 'static'
		} else {
			if (arg.type === 'javascript' && propertyType === 'static') {
				arg.type = 'static'
			}
			if (arg.type) {
				propertyType = arg.type
			}
		}
	}

	function isStaticTemplate(inputCat: InputCat) {
		return inputCat === 'string' || inputCat === 'sql' || inputCat == 'yaml'
	}

	function connectProperty(rawValue: string) {
		arg.expr = getDefaultExpr(undefined, previousModuleId, rawValue)
		arg.type = 'javascript'
		propertyType = 'javascript'
		monaco?.setCode(arg.expr)
	}

	function onFocus() {
		focused = true
		if (isStaticTemplate(inputCat)) {
			focusProp(argName, 'append', (path) => {
				const toAppend = `\$\{${path}}`
				arg.value = `${arg.value ?? ''}${toAppend}`
				monacoTemplate?.setCode(arg.value)
				setPropertyType(arg.value)
				argInput?.focus()
				return false
			})
		} else {
			focusProp(argName, 'insert', (path) => {
				arg.expr = path
				arg.type = 'javascript'
				propertyType = 'javascript'
				monaco?.setCode(arg.expr)
				return true
			})
		}
	}

	const { focusProp, propPickerConfig } = getContext<PropPickerWrapperContext>('PropPickerWrapper')

	$: isStaticTemplate(inputCat) && propertyType == 'static' && setPropertyType(arg?.value)

	function setDefaultCode() {
		if (!arg?.value) {
			monacoTemplate?.setCode(schema.properties[argName].default)
		}
	}

	$: schema.properties[argName].default && setDefaultCode()

	let resourceTypes: string[] | undefined = undefined

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

	let focused = false
	let stepInputGen: StepInputGen | undefined = undefined

	loadResourceTypes()
</script>

{#if arg != undefined}
	<div class={$$props.class}>
		<div class="flex flex-row justify-between gap-1 pb-1">
			<div class="flex flex-wrap grow">
				<FieldHeader
					label={argName}
					format={schema.properties[argName].format}
					contentEncoding={schema.properties[argName].contentEncoding}
					required={schema.required.includes(argName)}
					type={schema.properties[argName].type}
				/>

				{#if isStaticTemplate(inputCat)}
					<div>
						<span
							class="bg-blue-100 text-blue-800 text-sm font-medium mr-2 px-2.5 !py-0.5 rounded ml-2 {propertyType ==
								'static' && arg.type === 'javascript'
								? 'visible'
								: 'invisible'}"
						>
							{'${...}'}
						</span>
					</div>
				{/if}
			</div>
			{#if !noDynamicToggle}
				<div class="flex flex-row gap-x-2 gap-y-1 flex-wrap z-10 items-center">
					{#if enableAi}
						<StepInputGen
							bind:this={stepInputGen}
							{focused}
							{arg}
							schemaProperty={schema.properties[argName]}
							showPopup={(isStaticTemplate(inputCat) && propertyType == 'static') ||
								propertyType === undefined ||
								propertyType === 'static' ||
								arg?.expr?.length > 0}
							on:showExpr={(e) => {
								setTimeout(() => {
									if (monaco && propertyType === 'javascript') {
										monaco.setSuggestion(e.detail)
									}
								}, 0)
							}}
							on:setExpr={(e) => {
								arg = {
									type: 'javascript',
									expr: e.detail
								}
								propertyType = 'javascript'
								monaco?.setCode('')
								monaco?.insertAtCursor(e.detail)
							}}
							{pickableProperties}
							{argName}
						/>
					{/if}
					<div>
						<ToggleButtonGroup
							selected={propertyType}
							on:selected={(e) => {
								if (e.detail == propertyType) return
								const staticTemplate = isStaticTemplate(inputCat)
								if (e.detail === 'javascript') {
									if (arg.expr == undefined) {
										arg.expr = getDefaultExpr(
											argName,
											previousModuleId,
											staticTemplate
												? `\`${arg?.value?.toString().replaceAll('`', '\\`') ?? ''}\``
												: arg.value
												? JSON.stringify(arg?.value, null, 4)
												: ''
										)
									}
									if (arg) {
										arg.value = undefined
										arg.type = 'javascript'
									}
									propertyType = 'javascript'
								} else {
									if (staticTemplate) {
										if (arg) {
											arg.value = codeToStaticTemplate(arg.expr)
											arg.expr = undefined
										}
										setPropertyType(arg?.value)
									} else {
										if (arg) {
											arg.type = 'static'
											arg.value = undefined
											arg.expr = undefined
										}
									}
									propertyType = 'static'
								}
							}}
						>
							{#if isStaticTemplate(inputCat)}
								<ToggleButton
									tooltip={`Write text or surround javascript with \`\$\{\` and \`\}\`. Use \`results\` to connect to another node\'s output.`}
									light
									value="static"
									size="xs2"
									label={'${}'}
								/>
							{:else}
								<ToggleButton small label="Static" value="static" />
							{/if}

							<ToggleButton
								small
								light
								tooltip="Javascript expression ('flow_input' or 'results')."
								value="javascript"
								icon={FunctionSquare}
							/>
						</ToggleButtonGroup>
					</div>

					<Button
						title="Connect to another node's output"
						variant="border"
						color="light"
						size="xs2"
						on:click={() => {
							focusProp(argName, 'connect', (path) => {
								connectProperty(path)
								return true
							})
						}}
						id="flow-editor-plug"
					>
						<Plug size={16} /> &rightarrow;
					</Button>
				</div>
			{/if}
		</div>

		<div class="max-w-xs" />
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			class="relative {$propPickerConfig?.propName == argName
				? 'outline outline-offset-0 outline-2 outline-blue-500 rounded-md'
				: ''}"
			on:keyup={stepInputGen?.onKeyUp}
		>
			{#if $propPickerConfig?.propName == argName && $propPickerConfig?.insertionMode == 'connect'}
				<span
					class={'text-white  z-50 px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute top-0 right-0 bg-blue-500'}
				>
					Connect input &rightarrow;
				</span>
			{/if}
			<!-- {inputCat}
			{propertyType} -->
			{#if isStaticTemplate(inputCat) && propertyType == 'static' && !noDynamicToggle}
				<div class="mt-2 min-h-[28px]">
					{#if arg}
						<TemplateEditor
							bind:this={monacoTemplate}
							{extraLib}
							on:focus={onFocus}
							on:blur={() => {
								focused = false
							}}
							bind:code={arg.value}
							fontSize={14}
						/>
					{/if}
				</div>
			{:else if propertyType === undefined || propertyType == 'static'}
				<ArgInput
					{resourceTypes}
					noMargin
					compact
					bind:this={argInput}
					on:focus={onFocus}
					on:blur={() => {
						focused = false
					}}
					label={argName}
					bind:editor={monaco}
					bind:description={schema.properties[argName].description}
					bind:value={arg.value}
					type={schema.properties[argName].type}
					required={schema.required.includes(argName)}
					bind:pattern={schema.properties[argName].pattern}
					bind:valid={inputCheck}
					defaultValue={schema.properties[argName].default}
					bind:enum_={schema.properties[argName].enum}
					bind:format={schema.properties[argName].format}
					contentEncoding={schema.properties[argName].contentEncoding}
					bind:itemsType={schema.properties[argName].items}
					properties={schema.properties[argName].properties}
					nestedRequired={schema.properties[argName].required}
					displayHeader={false}
					extra={argExtra}
					{variableEditor}
					{itemPicker}
					bind:pickForField
					showSchemaExplorer
				/>
			{:else if arg.expr != undefined}
				<div class="border rounded mt-2 border-gray-300">
					<SimpleEditor
						bind:this={monaco}
						bind:code={arg.expr}
						{extraLib}
						lang="javascript"
						shouldBindKey={false}
						on:focus={() => {
							focused = true
							focusProp(argName, 'insert', (path) => {
								monaco?.insertAtCursor(path)
								return false
							})
						}}
						on:blur={() => {
							focused = false
						}}
						autoHeight
					/>
				</div>
				<DynamicInputHelpBox />
				<div class="mb-2" />
			{:else}
				Not recognized input type {argName} ({arg.expr}, {propertyType})
			{/if}
		</div>
	</div>
{/if}
