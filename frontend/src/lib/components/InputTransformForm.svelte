<script module>
	const dynamicTemplateRegexPairs = buildPrefixRegex([
		'flow_input',
		'results',
		'resource',
		'variable',
		'flow_env'
	])
</script>

<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { InputCat, DynamicInput as DynamicInputTypes } from '$lib/utils'
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import { computeShow } from '$lib/utils'

	import ArgInput from './ArgInput.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import DynamicInputHelpBox from './flows/content/DynamicInputHelpBox.svelte'
	import type { PropPickerWrapperContext } from './flows/propPicker/PropPickerWrapper.svelte'
	import { codeToStaticTemplate, getDefaultExpr } from './flows/utils.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import { Button, ButtonType } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { tick } from 'svelte'
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
	import S3ArrayHelperButton from './S3ArrayHelperButton.svelte'
	import { inputBorderClass } from './text_input/TextInput.svelte'
	import FakeMonacoPlaceHolder from './FakeMonacoPlaceHolder.svelte'
	import SchemaFormTransform from './SchemaFormTransform.svelte'

	// We add 'ai' for ai agent tools. 'ai' means the field will be filled by the AI agent dynamically.
	type PropertyType = InputTransform['type'] | 'ai'

	interface Props {
		schema: Schema | { properties?: Record<string, any>; required?: string[] }
		arg: InputTransform | any
		argName: string
		headerTooltip?: string | undefined
		headerTooltipIconClass?: string
		HeaderTooltipIcon?: any
		extraLib?: string
		inputCheck?: boolean
		previousModuleId: string | undefined
		pickForField?: string | undefined
		variableEditor?: VariableEditor | undefined
		itemPicker?: ItemPicker | undefined
		noDynamicToggle?: boolean
		argExtra?: Record<string, any>
		pickableProperties?: PickableProperties | undefined
		enableAi?: boolean
		hideHelpButton?: boolean
		class?: string
		editor?: SimpleEditor | undefined
		otherArgs?: Record<string, InputTransform>
		helperScript?: DynamicInputTypes.HelperScript | undefined
		isAgentTool?: boolean
		s3StorageConfigured?: boolean
	}

	let {
		schema = $bindable(),
		arg = $bindable(),
		argName = $bindable(),
		headerTooltip = undefined,
		headerTooltipIconClass = '',
		HeaderTooltipIcon = InfoIcon,
		extraLib = $bindable('missing extraLib'),
		inputCheck = $bindable(true),
		previousModuleId,
		pickForField = $bindable(undefined),
		variableEditor = undefined,
		itemPicker = undefined,
		noDynamicToggle = false,
		argExtra = {},
		pickableProperties = undefined,
		enableAi = false,
		hideHelpButton = false,
		class: className = '',
		editor = $bindable(undefined),
		otherArgs = {},
		helperScript = undefined,
		isAgentTool = false,
		s3StorageConfigured = true
	}: Props = $props()

	let monaco: SimpleEditor | undefined = $state(undefined)
	let monacoTemplate: TemplateEditor | undefined = $state(undefined)
	let focusedPrev = false

	let hidden = $state(false)

	const variableMatch = (value: string): RegExpMatchArray | null =>
		value.match(/^variable\('([^']+)'\)$/)
	const resourceMatch = (value: string): RegExpMatchArray | null =>
		value.match(/^resource\('([^']+)'\)$/)

	const dispatch = createEventDispatcher()

	$effect(() => {
		editor = monaco
	})

	const { shouldUpdatePropertyType, exprsToSet } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	const propPickerWrapperContext: PropPickerWrapperContext | undefined =
		getContext<PropPickerWrapperContext>('PropPickerWrapper')
	const { inputMatches, focusProp, propPickerConfig, clearFocus } = propPickerWrapperContext ?? {}

	let inputCat = $derived(
		computeInputCat(
			schema?.properties?.[argName]?.type,
			schema?.properties?.[argName]?.format,
			schema?.properties?.[argName]?.items?.type,
			schema?.properties?.[argName]?.enum,
			schema?.properties?.[argName]?.contentEncoding
		)
	)

	let propertyType = $state(getPropertyType(arg))

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

	function updatePropertyType() {
		propertyType = $shouldUpdatePropertyType?.[argName] || 'static'
		shouldUpdatePropertyType?.set({
			...$shouldUpdatePropertyType,
			[argName]: undefined
		})
	}

	// Try to parse JavaScript expression back to static object/array for visual editing
	function tryParseJsToStatic(expr: string, schemaType: string): any | null {
		if (!expr || typeof expr !== 'string') return null

		try {
			// Remove wrapping parentheses if present
			let code = expr.trim()
			if (code.startsWith('(') && code.endsWith(')')) {
				code = code.slice(1, -1).trim()
			}

			// Try to parse as object literal or array
			// This is a simple heuristic - check if it looks like a static structure
			if (schemaType === 'object' && code.startsWith('{') && code.endsWith('}')) {
				// Try to evaluate it safely
				// For now, use a simple regex-based approach to extract static-looking values
				// If the object contains only simple expressions, convert it
				const hasComplexLogic = /\bif\b|\bfor\b|\bwhile\b|\bfunction\b|\breturn\b/.test(code)
				if (!hasComplexLogic) {
					// Attempt to parse it
					try {
						const parsed = new Function(`return ${code}`)()
						return parsed
					} catch {
						return null
					}
				}
			} else if (schemaType === 'array' && code.startsWith('[') && code.endsWith(']')) {
				// Similar for arrays
				const hasComplexLogic = /\bif\b|\bfor\b|\bwhile\b|\bfunction\b|\breturn\b/.test(code)
				if (!hasComplexLogic) {
					try {
						const parsed = new Function(`return ${code}`)()
						return parsed
					} catch {
						return null
					}
				}
			}
		} catch (e) {
			return null
		}

		return null
	}

	function getPropertyType(arg: InputTransform | any): PropertyType {
		// For agent tools, if static with undefined/empty value, treat as 'ai', meaning the field will be filled by the AI agent dynamically.
		if (
			isAgentTool &&
			((arg?.type === 'static' && arg?.value === undefined) || arg?.type === 'ai')
		) {
			if (arg?.type === 'static') {
				arg.type = 'ai'
			}
			return 'ai'
		}

		let type: PropertyType = arg?.type ?? 'static'

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

		// Try to parse javascript expressions for objects/arrays back to static for visual editing
		if (type == 'javascript' && (inputCat === 'object' || inputCat === 'list')) {
			const parsed = tryParseJsToStatic(arg.expr, inputCat === 'object' ? 'object' : 'array')
			if (parsed !== null) {
				type = 'static'
				arg.value = parsed
				// Keep the expr for reference but switch to static mode for UI
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
			if (arg.expr != undefined) {
				arg.expr = undefined
			}
		}
	}

	let codeInjectionDetected = $state(false)

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

	function appendPathToArrayExpr(currentExpr: string | undefined, path: string) {
		const trimmedExpr = currentExpr?.trim() || ''

		let newExpr = trimmedExpr
		if (trimmedExpr.startsWith('[') && trimmedExpr.endsWith(']')) {
			// Parse existing array and append new item
			const innerContent = trimmedExpr.slice(1, -1).trim()
			if (innerContent) {
				newExpr = `[${innerContent}, ${path}]`
			} else {
				newExpr = `[${path}]`
			}
		} else {
			// Create new array with single item
			newExpr = `[${path}]`
		}
		arg.expr = newExpr
		arg.type = 'javascript'

		// Update Monaco editor after setting the expression
		tick().then(() => {
			monaco?.setCode(newExpr)
		})

		// Dispatch change
		dispatch('change', { argName, arg })
	}

	async function switchToJsAndConnect(onPath: (path: string) => void) {
		// Switch to JavaScript mode
		propertyType = 'javascript'
		arg.type = 'javascript'
		arg.expr = arg.expr || '[]'
		arg.value = undefined

		// Wait for the component to re-render and Monaco to be available
		await tick()

		// Activate connect mode
		focusProp?.(argName, 'connect', (path) => {
			onPath(path)
			return true
		})
	}

	function connectProperty(rawValue: string) {
		// Extract path from variable('x') or resource('x') format
		const varMatch = variableMatch(rawValue)
		const resMatch = resourceMatch(rawValue)

		if (varMatch) {
			arg.type = 'static'
			propertyType = 'static'
			arg.value = '$var:' + varMatch[1]
			monacoTemplate?.setCode(arg.value)
		} else if (resMatch) {
			arg.type = 'static'
			propertyType = 'static'
			arg.value = '$res:' + resMatch[1]
			monacoTemplate?.setCode(arg.value)
		} else {
			arg.expr = getDefaultExpr(undefined, previousModuleId, rawValue)
			arg.type = 'javascript'
			propertyType = 'javascript'
			monaco?.setCode(arg.expr)
		}
	}

	// This only works if every fields are static, as we can't eval javascript
	function handleFieldVisibility(
		schema: Schema | any,
		arg: InputTransform | any,
		otherArgs: Record<string, any>
	) {
		const schemaProperty = schema?.properties?.[argName]
		if (schemaProperty?.showExpr) {
			// Build args object with current field value and other context
			const currentValue = propertyType === 'static' ? arg?.value : arg?.expr

			// Convert otherArgs from InputTransform objects to their actual values
			const contextArgs = {
				[argName]: currentValue
			}

			let hasJavascript = false

			// Extract values from InputTransform objects in otherArgs
			Object.keys(otherArgs ?? {}).forEach((key) => {
				if (otherArgs[key].type === 'javascript') {
					hasJavascript = true
				}
				const otherArg = otherArgs[key]
				const otherArgValue = otherArg.type === 'static' ? otherArg.value : otherArg.expr
				contextArgs[key] = otherArgValue
			})

			const shouldShow = computeShow(argName, schemaProperty.showExpr, contextArgs)
			if (shouldShow || hasJavascript) {
				hidden = false
			} else if (!hidden) {
				hidden = true
				// Clear the arg value when hidden (following SchemaForm pattern)
				if (arg) {
					arg.value = undefined
					arg.expr = undefined
				}
				// Make sure validation passes when hidden
				inputCheck = true
			}
		} else {
			// No showExpr, always show
			hidden = false
		}
	}

	function onFocus() {
		focused = true
	}

	let prevArg: any = undefined
	function onArgChange() {
		const newArg = { arg, propertyType, inputCat }
		if (!deepEqual(newArg, prevArg)) {
			prevArg = structuredClone($state.snapshot(newArg))
			updateStaticInput(inputCat, propertyType, arg)
		}
	}

	function updateStaticInput(
		inputCat: InputCat,
		propertyType: PropertyType,
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

	let resourceTypes: string[] | undefined = $state(undefined)

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

	let focused = $state(false)
	let stepInputGen: StepInputGen | undefined = $state(undefined)

	loadResourceTypes()

	$effect(() => {
		$exprsToSet?.[argName] && untrack(() => setExpr())
	})
	$effect(() => {
		$shouldUpdatePropertyType?.[argName] &&
			arg?.type === $shouldUpdatePropertyType?.[argName] &&
			untrack(() => updatePropertyType())
	})
	$effect(() => {
		arg?.value
		arg?.expr
		inputCat && propertyType && arg && untrack(() => onArgChange())
	})
	$effect(() => {
		;[focused]
		untrack(() => updateFocused(focused))
	})
	$effect(() => {
		schema?.properties?.[argName]?.default && untrack(() => setDefaultCode())
	})
	$effect.pre(() => {
		// Monitor changes that affect field visibility
		JSON.stringify(schema)
		JSON.stringify(arg)
		JSON.stringify(otherArgs)

		untrack(() => handleFieldVisibility(schema, arg, otherArgs))
	})
	let connecting = $derived(
		$propPickerConfig?.propName == argName && $propPickerConfig?.insertionMode == 'connect'
	)
	let shouldShowS3ArrayHelper = $derived(
		inputCat === 'list' &&
			['s3object', 's3_object'].includes(schema?.properties?.[argName]?.items?.resourceType)
	)

	let suggestion: string | undefined = $state()

	// Svelte bug ...
	// Somehow the value is updated in the UI of the parent, but not in the children
	// when passed as a prop. setTimeout is a workaround to force the update
	let visiblePropertyType = $state(untrack(() => (suggestion ? 'javascript' : propertyType)))
	$effect(() => {
		let value = suggestion ? 'javascript' : propertyType
		setTimeout(() => (visiblePropertyType = value), 1)
	})
</script>

{#if arg != undefined && !hidden}
	<div class={twMerge('relative group flex flex-col gap-1', className)}>
		<div class="flex flex-row flex-wrap justify-between gap-1">
			<div class="flex grow min-h-7 items-end">
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
							class="border text-gray-400 dark:text-gray-500 text-2xs font-medium mr-2 px-1 !py-[1px] rounded ml-2.5 {propertyType ==
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
				<div
					class="flex flex-row gap-x-2 z-10 absolute right-0 group-hover:bg-surface transition-colors"
				>
					{#if enableAi}
						<StepInputGen
							bind:this={stepInputGen}
							{focused}
							{arg}
							schemaProperty={schema?.properties?.[argName]}
							on:showExpr={(e) => (suggestion = e.detail || undefined)}
							on:setExpr={(e) => {
								arg = { type: 'javascript', expr: e.detail }
								propertyType = 'javascript'
								monaco?.setCode('')
								monaco?.insertAtCursor(e.detail)
							}}
							{pickableProperties}
							{argName}
							btnClass={twMerge(
								'h-7 min-w-8 px-2',
								'group-hover:opacity-100 transition-opacity',
								!connecting ? 'opacity-0' : ''
							)}
						/>
					{/if}

					{#if propPickerWrapperContext}
						<FlowPlugConnect
							wrapperClasses={twMerge(
								connecting ? 'h-6 w-7' : 'h-7 w-8',
								'group-hover:opacity-100 transition-opacity p-0',
								!connecting ? 'opacity-0' : ''
							)}
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

					<div class="{ButtonType.UnifiedHeightClasses.sm} relative">
						<ToggleButtonGroup
							selected={visiblePropertyType}
							class="h-full"
							on:selected={(e) => {
								if (e.detail == propertyType || suggestion) return
								const staticTemplate = isStaticTemplate(inputCat)

								if (e.detail === 'ai') {
									// Switch to AI mode: static with no value
									if (arg) {
										arg.type = 'ai'
										arg.value = undefined
										arg.expr = undefined
									}
									propertyType = 'ai'
								} else if (e.detail === 'javascript') {
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
							{#snippet children({ item })}
								{#if isAgentTool}
									<ToggleButton
										small
										label="AI"
										value="ai"
										tooltip="Let the AI agent fill this field dynamically"
										{item}
									/>
								{/if}

								{#if isStaticTemplate(inputCat)}
									<ToggleButton
										size="sm"
										tooltip={`Write text or surround javascript with \`\$\{\` and \`\}\`. Use \`results\` to connect to another node\'s output.`}
										value="static"
										label={'${}'}
										{item}
										class="h-full text-xs"
									/>
								{:else}
									<ToggleButton
										size="sm"
										label="static"
										value="static"
										{item}
										class="h-full text-xs"
									/>
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
										disabled={inputCat === 'dynamic'}
										small
										tooltip="JavaScript expression ('flow_input' or 'results')."
										value="javascript"
										icon={FunctionSquare}
										{item}
										class="h-full"
									/>
								{/if}
							{/snippet}
						</ToggleButtonGroup>
					</div>
				</div>
			{/if}
		</div>

		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="relative w-full" onkeyup={handleKeyUp}>
			<!-- {#if $propPickerConfig?.propName == argName && $propPickerConfig?.insertionMode == 'connect'}
				<span
					class={'text-white  z-50 px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute top-0 right-0 bg-blue-500'}
				>
					Connect input &rightarrow;
				</span>
			{/if} -->
			<!-- {inputCat}
			{propertyType} -->
			<div class="relative flex flex-row items-top gap-1 justify-between">
				<div class="min-w-0 grow">
					{#if suggestion}
						<div
							class={`bg-surface-input rounded-md pl-2 overflow-auto ${inputBorderClass({ forceFocus: true })}`}
						>
							<FakeMonacoPlaceHolder autoheight code={suggestion} fontSize={12} />
						</div>
					{/if}
					<div
						class={suggestion ? 'opacity-0 absolute' : ''}
						onkeydowncapture={(e) => {
							if (e.key === 'Tab' && suggestion) {
								e.preventDefault()
							}
						}}
					>
						{@render innerInput()}
					</div>

					{#snippet innerInput()}
						{#if propertyType === 'ai'}
							<div
								class="text-sm text-tertiary italic p-3 bg-surface-secondary rounded-md border border-gray-200"
							>
								<span class="flex items-center gap-2">
									<InfoIcon size={16} />
									This field will be filled by the AI agent dynamically
								</span>
							</div>
							{#if argName && schema?.properties?.[argName]?.description}
								<div class="text-xs italic py-1 text-hint">
									<pre class="font-main whitespace-normal">
										{schema.properties[argName].description}
									</pre>
								</div>
							{/if}
						{:else if isStaticTemplate(inputCat) && propertyType == 'static' && !noDynamicToggle}
							<div class="flex flex-col gap-1">
								{#if argName && schema?.properties?.[argName]?.description}
									<div class="text-xs text-secondary">
										<pre class="font-main whitespace-normal">
										{schema.properties[argName].description}
										</pre>
									</div>
								{/if}

								{#if arg}
									<TemplateEditor
										bind:this={monacoTemplate}
										{extraLib}
										on:focus={onFocus}
										on:blur={() => {
											focused = false
										}}
										bind:code={arg.value}
										fontSize={12}
										on:change={() => {
											dispatch('change', { argName, arg })
										}}
										loadAsync
										class="bg-surface-input"
									/>
								{/if}
							</div>
						{:else if (propertyType === undefined || propertyType == 'static') && schema?.properties?.[argName]}
							{@const schemaProperty = schema.properties[argName]}
							{@const isObjectWithProperties =
								(schemaProperty.type === 'object' && schemaProperty.properties) ||
								(schemaProperty.type === 'array' &&
									schemaProperty.items?.type === 'object' &&
									schemaProperty.items?.properties)}

							{#if isObjectWithProperties}
								<!-- Use recursive InputTransformForm for nested objects/arrays with properties -->
								{#if schemaProperty.type === 'object' && schemaProperty.properties}
									<!-- Direct object with properties -->
									<div class="border rounded-md px-4 pt-4 pb-2">
										{#if arg.value && typeof arg.value === 'object' && !Array.isArray(arg.value)}
											<!-- Convert arg.value (plain values) to InputTransform format for nested fields -->
											{@const nestedArgs = Object.fromEntries(
												Object.keys(schemaProperty.properties).map((key) => {
													const val = arg.value?.[key]
													// If already an InputTransform, keep it; otherwise wrap as static
													if (val && typeof val === 'object' && ('type' in val || 'expr' in val)) {
														return [key, val]
													}
													return [key, { type: 'static', value: val }]
												})
											)}
											<SchemaFormTransform
												schema={{
													properties: schemaProperty.properties,
													required: schemaProperty.required ?? [],
													$schema: '',
													type: 'object'
												}}
												bind:args={
													() => nestedArgs,
													(v) => {
														// Check if any nested field uses javascript mode
														let hasJavascriptField = false
														const plainValues = {}
														const jsExprParts = []

														Object.keys(v ?? {}).forEach((key) => {
															const transform = v[key]
															if (transform?.type === 'javascript') {
																hasJavascriptField = true
																// Remove wrapping if it's already wrapped
																let expr = transform.expr
																if (expr.startsWith('(') && expr.endsWith(')')) {
																	expr = expr.slice(1, -1)
																}
																jsExprParts.push(`"${key}": ${expr}`)
															} else {
																const val =
																	transform?.type === 'static' ? transform.value : transform?.value
																plainValues[key] = val
																// For JS object construction
																jsExprParts.push(`"${key}": ${JSON.stringify(val)}`)
															}
														})

														if (hasJavascriptField) {
															// Switch parent to javascript mode with full object expression
															// BUT keep propertyType as 'static' so UI stays in visual mode
															arg.type = 'javascript'
															arg.expr = `({\n  ${jsExprParts.join(',\n  ')}\n})`
															// Keep arg.value for visual editing
															arg.value = plainValues
														} else {
															// All fields are static, keep as static object
															arg.type = 'static'
															arg.value = plainValues
															arg.expr = undefined
														}
														dispatch('change', { argName, arg })
													}
												}
												{extraLib}
												{previousModuleId}
												{pickableProperties}
												{enableAi}
												{otherArgs}
												{isAgentTool}
												{s3StorageConfigured}
											/>
										{/if}
									</div>
								{:else if schemaProperty.type === 'array' && schemaProperty.items?.type === 'object' && schemaProperty.items?.properties}
									<!-- Array of objects with properties - each item should support template strings -->
									<div class="flex flex-col gap-2">
										{#if Array.isArray(arg.value)}
											{#each arg.value as item, i}
												<div class="border rounded-md px-4 pt-4 pb-2 relative">
													<button
														class="absolute top-2 right-2 p-1 hover:bg-surface-hover rounded"
														onclick={() => {
															arg.value = arg.value.filter((_, idx) => idx !== i)
															dispatch('change', { argName, arg })
														}}
													>
														<span class="text-xs">✕</span>
													</button>
													{@const itemArgs = Object.fromEntries(
														Object.keys(schemaProperty.items.properties).map((key) => {
															const val = item?.[key]
															if (
																val &&
																typeof val === 'object' &&
																('type' in val || 'expr' in val)
															) {
																return [key, val]
															}
															return [key, { type: 'static', value: val }]
														})
													)}
													<SchemaFormTransform
														schema={{
															properties: schemaProperty.items.properties,
															required: schemaProperty.items.required ?? [],
															$schema: '',
															type: 'object'
														}}
														bind:args={
															() => itemArgs,
															(v) => {
																// Check if any field in this array item uses javascript
																let hasJavascriptField = false
																const plainValues = {}
																const jsExprParts = []

																Object.keys(v ?? {}).forEach((key) => {
																	const transform = v[key]
																	if (transform?.type === 'javascript') {
																		hasJavascriptField = true
																		let expr = transform.expr
																		if (expr.startsWith('(') && expr.endsWith(')')) {
																			expr = expr.slice(1, -1)
																		}
																		jsExprParts.push(`"${key}": ${expr}`)
																	} else {
																		const val =
																			transform?.type === 'static'
																				? transform.value
																				: transform?.value
																		plainValues[key] = val
																		jsExprParts.push(`"${key}": ${JSON.stringify(val)}`)
																	}
																})

																// Update the array item
																arg.value[i] = plainValues

																// Check if ANY item in the array has javascript
																let arrayHasJavascript = hasJavascriptField
																if (!arrayHasJavascript && Array.isArray(arg.value)) {
																	// Check other items for javascript expressions stored as strings
																	arrayHasJavascript = arg.value.some((item, idx) => {
																		if (idx === i) return false
																		return Object.values(item ?? {}).some(
																			(val) =>
																				typeof val === 'string' &&
																				(val.includes('${') ||
																					val.startsWith('`') ||
																					val.includes('results.'))
																		)
																	})
																}

																if (arrayHasJavascript) {
																	// Convert entire array to javascript mode
																	// BUT keep UI in visual mode by preserving arg.value
																	const arrayExprParts = arg.value.map((item, idx) => {
																		if (idx === i && hasJavascriptField) {
																			return `{\n    ${jsExprParts.join(',\n    ')}\n  }`
																		} else {
																			const itemParts = Object.keys(item ?? {}).map((key) => {
																				return `"${key}": ${JSON.stringify(item[key])}`
																			})
																			return `{\n    ${itemParts.join(',\n    ')}\n  }`
																		}
																	})
																	arg.type = 'javascript'
																	arg.expr = `([\n  ${arrayExprParts.join(',\n  ')}\n])`
																	// Keep arg.value for visual editing - don't set to undefined!
																}

																dispatch('change', { argName, arg })
															}
														}
														{extraLib}
														{previousModuleId}
														{pickableProperties}
														{enableAi}
														{otherArgs}
														{isAgentTool}
														{s3StorageConfigured}
													/>
												</div>
											{/each}
										{/if}
										<Button
											variant="default"
											color="light"
											size="xs"
											on:click={() => {
												if (!Array.isArray(arg.value)) {
													arg.value = []
												}
												arg.value = [...arg.value, {}]
												dispatch('change', { argName, arg })
											}}
										>
											Add item
										</Button>
									</div>
								{/if}
							{:else}
								<!-- Use ArgInput for simple types and objects without properties -->
								<ArgInput
									{resourceTypes}
									noMargin
									compact
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
									{helperScript}
									{s3StorageConfigured}
									otherArgs={Object.fromEntries(
										Object.entries(otherArgs).map(([key, transform]) => [
											key,
											transform?.type === 'static' ? transform.value : transform?.expr
										])
									)}
								>
									{#snippet innerBottomSnippet()}
										{#if shouldShowS3ArrayHelper}
											<S3ArrayHelperButton
												{connecting}
												onClick={() =>
													switchToJsAndConnect((path) => appendPathToArrayExpr(arg.expr, path))}
											/>
										{/if}
									{/snippet}
								</ArgInput>
							{/if}
						{:else if arg.expr != undefined}
							<div
								class={`bg-surface-input rounded-md flex flex-col pl-2 overflow-auto ${inputBorderClass({ forceFocus: focused })}`}
							>
								<SimpleEditor
									small
									bind:this={monaco}
									bind:code={arg.expr}
									{extraLib}
									lang="javascript"
									shouldBindKey={false}
									renderLineHighlight="none"
									hideLineNumbers
									on:focus={() => (focused = true)}
									on:blur={() => (focused = false)}
									on:change={() => {
										dispatch('change', { argName, arg })
									}}
									autoHeight
									loadAsync
								/>
								<!-- <input type="text" bind:value={arg.expr} /> -->
							</div>

							{#if shouldShowS3ArrayHelper}
								<S3ArrayHelperButton
									class="mt-2"
									{connecting}
									onClick={() =>
										focusProp?.(argName, 'connect', (path) => {
											appendPathToArrayExpr(arg.expr, path)
											return true
										})}
								/>
							{/if}

							{#if argName && schema?.properties?.[argName]?.description}
								<div class="text-xs italic py-1 text-secondary">
									<pre class="font-main whitespace-normal"
										>{schema.properties[argName].description}</pre
									>
								</div>
							{/if}

							{#if !hideHelpButton}
								<DynamicInputHelpBox />
							{/if}

							<div class="mb-2"></div>
						{:else}
							<span class="text-xs text-red-500">
								Not recognized input type {argName} ({arg.expr}, {propertyType})
							</span>
							<div class="flex mt-2">
								<Button
									variant="default"
									size="xs"
									on:click={() => {
										arg.expr = ''
									}}>Set expr to empty string</Button
								></div
							>
						{/if}
					{/snippet}
				</div>
			</div>
		</div>
	</div>
{/if}
