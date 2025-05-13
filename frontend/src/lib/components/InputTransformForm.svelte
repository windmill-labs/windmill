<script context="module">
	const dynamicTemplateRegexPairs = buildPrefixRegex([
		'flow_input',
		'results',
		'resource',
		'variable'
	])
</script>

<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { InputCat } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'

	import ArgInput from './ArgInput.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import DynamicInputHelpBox from './flows/content/DynamicInputHelpBox.svelte'
	import type { PropPickerWrapperContext } from './flows/propPicker/PropPickerWrapper.svelte'
	import { codeToStaticTemplate, getDefaultExpr } from './flows/utils'
	import SimpleEditor from './SimpleEditor.svelte'
	import { Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { tick } from 'svelte'
	import { fade } from 'svelte/transition'
	import { buildPrefixRegex } from './flows/previousResults'
	import type VariableEditor from './VariableEditor.svelte'
	import type ItemPicker from './ItemPicker.svelte'
	import type { InputTransform } from '$lib/gen'
	import TemplateEditor from './TemplateEditor.svelte'
	import { setInputCat as computeInputCat, isCodeInjection } from '$lib/utils'
	import { FunctionSquare, InfoIcon } from 'lucide-svelte'
	import { getResourceTypes } from './resourceTypesStore'
	import type { FlowCopilotContext } from './copilot/flow'
	import StepInputGen from './copilot/StepInputGen.svelte'
	import type { PickableProperties } from './flows/previousResults'
	import { twMerge } from 'tailwind-merge'
	import FlowPlugConnect from './FlowPlugConnect.svelte'
	import { deepEqual } from 'fast-equals'
	export let schema: Schema | { properties?: Record<string, any>; required?: string[] }
	export let arg: InputTransform | any
	export let argName: string
	export let headerTooltip: string | undefined = undefined
	export let headerTooltipIconClass = ''
	export let HeaderTooltipIcon = InfoIcon
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
	export let hideHelpButton = false

	let monaco: SimpleEditor | undefined = undefined
	let monacoTemplate: TemplateEditor | undefined = undefined
	let argInput: ArgInput | undefined = undefined
	let focusedPrev = false

	const dispatch = createEventDispatcher()

	$: inputCat = computeInputCat(
		schema?.properties?.[argName]?.type,
		schema?.properties?.[argName]?.format,
		schema?.properties?.[argName]?.items?.type,
		schema?.properties?.[argName]?.enum,
		schema?.properties?.[argName]?.contentEncoding
	)

	let propertyType = getPropertyType(arg)

	const { shouldUpdatePropertyType, exprsToSet } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	const propPickerWrapperContext: PropPickerWrapperContext | undefined =
		getContext<PropPickerWrapperContext>('PropPickerWrapper')
	const { inputMatches, focusProp, propPickerConfig, clearFocus } = propPickerWrapperContext ?? {}

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

	let codeInjectionDetected = false

	function checkCodeInjection(rawValue: string) {
		if (!arg || !rawValue || rawValue.length < 3 || !dynamicTemplateRegexPairs) {
			return undefined
		}
		if (rawValue.trim() !== rawValue) {
			return undefined
		}
		const matches = dynamicTemplateRegexPairs.filter(({ regex }) => regex.test(rawValue))
		if (matches.length > 0) {
			return matches.map((m) => ({ word: m.word, value: rawValue }))
		}
		return undefined
	}

	async function setJavaScriptExpr(rawValue: string) {
		arg = {
			type: 'javascript',
			expr: rawValue
		}
		propertyType = 'javascript'
		monaco?.setCode('')
		monaco?.insertAtCursor(rawValue)
		await tick()
		monaco?.focus()
		await tick()
		monaco?.setCursorToEnd()
	}

	function handleKeyUp(e: KeyboardEvent) {
		if (
			e.key === 'Tab' &&
			isStaticTemplate(inputCat) &&
			propertyType == 'static' &&
			!noDynamicToggle &&
			codeInjectionDetected
		) {
			setJavaScriptExpr(arg.value)
		} else {
			stepInputGen?.onKeyUp?.(e)
		}
	}

	function isStaticTemplate(inputCat: InputCat) {
		return inputCat === 'string' || inputCat === 'sql' || inputCat == 'yaml'
	}

	function connectProperty(rawValue: string) {
		// Extract path from variable('x') or resource('x') format
		const varMatch = rawValue.match(/^variable\('([^']+)'\)$/)
		const resourceMatch = rawValue.match(/^resource\('([^']+)'\)$/)

		if (varMatch) {
			arg.type = 'static'
			propertyType = 'static'
			arg.value = '$var:' + varMatch[1]
			monacoTemplate?.setCode(arg.value)
		} else if (resourceMatch) {
			arg.type = 'static'
			propertyType = 'static'
			arg.value = '$res:' + resourceMatch[1]
			monacoTemplate?.setCode(arg.value)
		} else {
			arg.expr = getDefaultExpr(undefined, previousModuleId, rawValue)
			arg.type = 'javascript'
			propertyType = 'javascript'
			monaco?.setCode(arg.expr)
		}
	}

	function onFocus() {
		focused = true
		if (isStaticTemplate(inputCat)) {
			focusProp?.(argName, 'append', (path) => {
				const toAppend = `\$\{${path}}`
				arg.value = `${arg.value ?? ''}${toAppend}`
				monacoTemplate?.setCode(arg.value)
				setPropertyType(arg.value)
				argInput?.focus()
				return false
			})
		} else {
			focusProp?.(argName, 'insert', (path) => {
				arg.expr = path
				arg.type = 'javascript'
				propertyType = 'javascript'
				monaco?.setCode(arg.expr)
				return true
			})
		}
	}

	let prevArg: any = undefined
	$: inputCat && propertyType && arg && onArgChange()
	function onArgChange() {
		const newArg = { arg, propertyType, inputCat }
		if (!deepEqual(newArg, prevArg)) {
			prevArg = structuredClone(newArg)
			updateStaticInput(inputCat, propertyType, arg)
		}
	}

	function updateStaticInput(
		inputCat: InputCat,
		propertyType: 'static' | 'javascript',
		arg: InputTransform | any
	) {
		if (!isStaticTemplate(inputCat)) {
			return
		}
		if (propertyType == 'static') {
			setPropertyType(arg?.value)
			codeInjectionDetected = checkCodeInjection(arg?.value) != undefined
		} else if (propertyType == 'javascript' && focused && inputMatches) {
			// setPropertyType(arg?.expr)
			$inputMatches = checkCodeInjection(arg?.expr)
		}
	}

	function setDefaultCode() {
		if (!arg?.value) {
			monacoTemplate?.setCode(schema.properties?.[argName]?.default)
		}
	}

	function updateFocused(newFocused: boolean) {
		if (focusedPrev && !newFocused && inputMatches) {
			$inputMatches = undefined
		}
		focusedPrev = focused
	}
	$: updateFocused(focused)

	$: schema?.properties?.[argName]?.default && setDefaultCode()

	let resourceTypes: string[] | undefined = undefined

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

	let focused = false
	let stepInputGen: StepInputGen | undefined = undefined

	loadResourceTypes()

	$: connecting =
		$propPickerConfig?.propName == argName && $propPickerConfig?.insertionMode == 'connect'
</script>

{#if arg != undefined}
	<div
		class={twMerge(
			'pl-2 pt-2 pb-2 ml-2 relative hover:bg-surface hover:shadow-md transition-all duration-200',
			$propPickerConfig?.propName == argName
				? 'bg-surface border-l-4 border-blue-500 shadow-md rounded-l-md z-50 '
				: 'hover:rounded-md',
			$$props.class
		)}
	>
		<div class="flex flex-row justify-between gap-1 pb-1 px-2">
			<div class="flex flex-wrap grow">
				<FieldHeader
					label={argName}
					simpleTooltip={headerTooltip}
					simpleTooltipIconClass={headerTooltipIconClass}
					SimpleTooltipIcon={HeaderTooltipIcon}
					format={schema?.properties?.[argName]?.format}
					contentEncoding={schema?.properties?.[argName]?.contentEncoding}
					required={schema.required?.includes(argName)}
					type={schema.properties?.[argName]?.type}
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
							schemaProperty={schema?.properties?.[argName]}
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
							let:item
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
													? '(' + JSON.stringify(arg?.value, null, 4) + ')'
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
									} else if (inputCat == 'list' || inputCat == 'object') {
										if (arg) {
											try {
												let newExpr = arg.expr
												if (newExpr.startsWith('(') && newExpr.endsWith(')')) {
													newExpr = newExpr.slice(1, -1)
												}
												arg.value = JSON.parse(newExpr)
											} catch (e) {
												arg.value = undefined
											}
											arg.expr = undefined
											arg.type = 'static'
										}
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
									{item}
								/>
							{:else}
								<ToggleButton small label="Static" value="static" {item} />
							{/if}

							{#if codeInjectionDetected && propertyType == 'static'}
								<Button
									size="xs2"
									color="light"
									btnClasses="font-normal text-xs w-fit bg-green-100 text-green-800 hover:bg-green-100 dark:text-green-300 dark:bg-green-700 dark:hover:bg-green-600"
									on:click={() => setJavaScriptExpr(arg.value)}
								>
									<span class="font-normal whitespace-nowrap flex gap-2 items-center"
										><FunctionSquare size={14} /> detected -
										<span class="font-bold">TAB</span>
									</span>
								</Button>
							{:else}
								<ToggleButton
									small
									light
									tooltip="JavaScript expression ('flow_input' or 'results')."
									value="javascript"
									icon={FunctionSquare}
									{item}
								/>
							{/if}
						</ToggleButtonGroup>
					</div>

					{#if propPickerWrapperContext}
						<FlowPlugConnect
							id="flow-editor-plug"
							{connecting}
							on:click={() => {
								if (
									$propPickerConfig?.propName == argName &&
									$propPickerConfig?.insertionMode == 'connect'
								) {
									clearFocus()
								} else {
									focusProp?.(argName, 'connect', (path) => {
										connectProperty(path)
										dispatch('change', { argName })
										return true
									})
								}
							}}
						/>
					{/if}
				</div>
			{/if}
		</div>

		<div class="max-w-xs"></div>
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div class="relative" on:keyup={handleKeyUp}>
			<!-- {#if $propPickerConfig?.propName == argName && $propPickerConfig?.insertionMode == 'connect'}
				<span
					class={'text-white  z-50 px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute top-0 right-0 bg-blue-500'}
				>
					Connect input &rightarrow;
				</span>
			{/if} -->
			<!-- {inputCat}
			{propertyType} -->
			<div class="relative flex flex-row items-top gap-2 justify-between">
				<div class="min-w-0 grow">
					{#if isStaticTemplate(inputCat) && propertyType == 'static' && !noDynamicToggle}
						{#if argName && schema?.properties?.[argName]?.description}
							<div class="text-xs italic pb-1 text-secondary">
								<pre class="font-main">{schema.properties[argName].description}</pre>
							</div>
						{/if}
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
									on:change={() => {
										dispatch('change', { argName, arg })
									}}
									loadAsync
								/>
							{/if}
						</div>
					{:else if (propertyType === undefined || propertyType == 'static') && schema?.properties?.[argName]}
						<ArgInput
							{resourceTypes}
							noMargin
							compact
							bind:this={argInput}
							on:focus={onFocus}
							on:blur={() => {
								focused = false
							}}
							shouldDispatchChanges
							on:change={() => {
								dispatch('change', { argName, arg })
							}}
							label={argName}
							bind:editor={monaco}
							bind:description={schema.properties[argName].description}
							bind:value={arg.value}
							type={schema.properties[argName].type}
							oneOf={schema.properties[argName].oneOf}
							required={schema.required?.includes(argName)}
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
							nullable={schema.properties[argName].nullable}
							bind:title={schema.properties[argName].title}
							bind:placeholder={schema.properties[argName].placeholder}
						/>
					{:else if arg.expr != undefined}
						<div class="border mt-2">
							<SimpleEditor
								bind:this={monaco}
								bind:code={arg.expr}
								on:change={() => {
									dispatch('change', { argName, arg })
								}}
								{extraLib}
								lang="javascript"
								shouldBindKey={false}
								on:focus={() => {
									focused = true
									focusProp?.(argName, 'insert', (path) => {
										monaco?.insertAtCursor(path)
										return false
									})
								}}
								on:change={() => {
									dispatch('change', { argName, arg })
								}}
								on:blur={() => {
									focused = false
								}}
								autoHeight
								loadAsync
							/>
						</div>
						{#if !hideHelpButton}
							<DynamicInputHelpBox />
						{/if}
						<div class="mb-2"></div>
					{:else}
						Not recognized input type {argName} ({arg.expr}, {propertyType})
						<div class="flex mt-2">
							<Button
								variant="border"
								size="xs"
								on:click={() => {
									arg.expr = ''
								}}>Set expr to empty string</Button
							></div
						>
					{/if}
				</div>

				{#if $propPickerConfig?.propName == argName}
					<div
						class="text-blue-500 absolute top-2 lg:-right-2.5 -right-1"
						in:fade={{ duration: 200 }}
					>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							width="16"
							height="24"
							viewBox="0 0 24 24"
							fill="currentColor"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<polyline points="24 24 12 12 24 0" />
						</svg>
					</div>
				{:else}
					<div class="w-0"></div>
				{/if}
			</div>
		</div>
	</div>
{/if}
