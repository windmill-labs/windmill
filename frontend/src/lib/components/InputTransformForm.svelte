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
	import { createEventDispatcher, getContext, onDestroy, untrack, type Snippet } from 'svelte'
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
	import { escapeTemplateBackticks } from '$lib/utils/templateLiteral'
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
	import { slideDynamic } from '$lib/transitions'
	import InputError from './InputError.svelte'

	interface Props {
		schema: Schema | { properties?: Record<string, any>; required?: string[] }
		arg: InputTransform | any
		argName: string
		/** Display name, when the schema key isn't what the user should read. */
		label?: string
		/** Replaces the label header, so a setting's own toggle can name the field. */
		header?: Snippet
		/** Renders after the label: a button to unset the field, a badge. */
		labelExtra?: Snippet
		/** Drop the schema's description paragraph, for a form that carries it in a tooltip. */
		hideDescription?: boolean
		/** Keep the connect and transform controls out of the way until the row is reached, unless
		 *  the field already holds something the controls are needed to read. */
		subtleControls?: boolean
		/** The kind this field always holds, for a value that doesn't carry a `type` of its
		 *  own — a flow predicate is stored as a bare `{ expr }`. */
		argType?: InputTransform['type']
		/** Keep only the header: the setting owning this field is switched off, so there is
		 *  no value yet. Rendering the header here (rather than swapping in a bare toggle
		 *  outside) keeps one persistent row, so the field can slide in and out under it. */
		collapsed?: boolean
		/** Slide the input in and out as `collapsed` flips. */
		animateAppear?: boolean
		/** Message shown under the field, which also turns its border red. */
		error?: string | undefined
		headerTooltip?: string | undefined
		headerTooltipIconClass?: string
		HeaderTooltipIcon?: any
		extraLib?: string
		inputCheck?: boolean
		previousModuleId: string | undefined
		pickForField?: string | undefined
		variableEditor?: VariableEditor | undefined
		itemPicker?: ItemPicker | undefined
		/** Hide the static/expression switch, for a field that only ever holds one kind.
		 *  The connect button and the AI helper stay. */
		noDynamicToggle?: boolean
		/** Hide the connect button, for a surface with nothing to connect to. Distinct from
		 *  `noDynamicToggle`, which a field forced to an expression also sets. */
		noConnect?: boolean
		/** Drop the expression option, and every affordance that writes one: an expression reaching
		 *  such a field is stored and deployed like any other, whichever control put it there. The
		 *  rest of the switch stays, so a field can still be AI-filled or static. A field already
		 *  holding an expression keeps the option, or it could not be switched off it. */
		noJavascript?: boolean
		/** Replaces the default StepInputGen, for a field with its own AI helper. That
		 *  helper drives `suggestion` (its ghost text) and `aiOnKeyUp` (Tab to accept),
		 *  which the built-in one reaches through `stepInputGen` instead. */
		aiGen?: Snippet
		suggestion?: string
		focused?: boolean
		aiOnKeyUp?: (e: KeyboardEvent) => void
		argExtra?: Record<string, any>
		pickableProperties?: PickableProperties | undefined
		enableAi?: boolean
		hideHelpButton?: boolean
		class?: string
		editor?: SimpleEditor | undefined
		otherArgs?: Record<string, InputTransform>
		helperScript?: DynamicInputTypes.HelperScript | undefined
		isAgentTool?: boolean
		allowedAiTransforms?: string[] | undefined
		s3StorageConfigured?: boolean
		chatInputEnabled?: boolean
		workspace?: string | undefined
	}

	let {
		schema = $bindable(),
		arg = $bindable(),
		argName = $bindable(),
		label = undefined,
		header = undefined,
		labelExtra = undefined,
		hideDescription = false,
		subtleControls = false,
		argType = undefined,
		collapsed = false,
		animateAppear = false,
		error = undefined,
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
		noConnect = false,
		noJavascript = false,
		aiGen = undefined,
		suggestion = $bindable(),
		focused = $bindable(),
		aiOnKeyUp = undefined,
		argExtra = {},
		pickableProperties = undefined,
		enableAi = false,
		hideHelpButton = false,
		class: className = '',
		editor = $bindable(undefined),
		otherArgs = {},
		helperScript = undefined,
		isAgentTool = false,
		allowedAiTransforms = isAgentTool ? undefined : [],
		s3StorageConfigured = true,
		chatInputEnabled = false,
		workspace
	}: Props = $props()

	let monaco: SimpleEditor | undefined = $state(undefined)
	let monacoTemplate: TemplateEditor | undefined = $state(undefined)

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
	const pickerMode = $derived(propPickerWrapperContext?.pickerMode?.() ?? 'pane')
	const {
		inputMatches,
		connectProp: focusProp,
		propPickerConfig,
		clearConnect: clearFocus,
		openPicker,
		exprBeingEdited
	} = propPickerWrapperContext ?? {}

	let inputCat = $derived(
		computeInputCat(
			schema?.properties?.[argName]?.type,
			schema?.properties?.[argName]?.format,
			schema?.properties?.[argName]?.items?.type,
			schema?.properties?.[argName]?.enum,
			schema?.properties?.[argName]?.contentEncoding
		)
	)

	// Whether this specific field is allowed to use AI transforms
	let fieldAllowsAi = $derived(
		allowedAiTransforms === undefined || allowedAiTransforms.includes(argName)
	)

	// A `${}` field is static text that interpolates JavaScript, so it is only on offer where
	// expressions are. Elsewhere the same field is plain static: labelled `static`, edited in the
	// ordinary input, with no `${...}` hint promising an escape hatch that isn't there.
	let staticTemplateOffered = $derived(isStaticTemplate(inputCat) && !noJavascript)

	// `argType` wins over whatever the value carries: a predicate has no `type` field, so
	// inferring would land it on the static input instead of the expression editor.
	const argKind = $derived(argType ?? arg?.type)
	// Seeded once: `propertyType` is what the user switches, and `argType` is fixed per field.
	let propertyType = $state(untrack(() => argType) ?? getPropertyType(arg))

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

	function getPropertyType(arg: InputTransform | any): InputTransform['type'] {
		// For agent tools, if static with undefined/empty value, treat as 'ai', meaning the field will be filled by the AI agent dynamically.
		if (
			fieldAllowsAi &&
			((arg?.type === 'static' && arg?.value === undefined) || arg?.type === 'ai')
		) {
			if (arg?.type === 'static') {
				arg.type = 'ai'
			}
			return 'ai'
		}

		let type: InputTransform['type'] = arg?.type ?? 'static'

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

		// `${...}` becomes a JavaScript transform, so it is only read as one where such a transform
		// can be stored — the same condition `staticTemplateOffered` renders under. Elsewhere the
		// text stays what was typed, rather than turning into code the store then drops or, worse,
		// keeps pointing at a flow context this value will never be evaluated in.
		if (isCodeInjection(rawValue) && !noJavascript) {
			arg.expr = getDefaultExpr(
				argName,
				previousModuleId,
				`\`${escapeTemplateBackticks(rawValue.toString())}\``
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

	// A static value is whatever JSON the field holds, so it need not be a string, and the caller
	// runs inside an effect: throwing here would take the whole form down rather than one field.
	function checkCodeInjection(rawValue: unknown): { word: string; value: string }[] | undefined {
		if (typeof rawValue !== 'string') {
			return undefined
		}
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
			!noJavascript &&
			codeInjectionDetected
		) {
			setJavaScriptExpr(arg.value)
		} else {
			;(aiOnKeyUp ?? stepInputGen?.onKeyUp)?.(e)
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
		focusProp?.(argName, (path) => {
			onPath(path)
			return true
		})
	}

	/** A predicate is usually half-written when you reach for a property, so insert at the
	 *  cursor and leave the rest of the expression alone. Only a field that isn't an
	 *  expression yet gets replaced outright. */
	function pickIntoArg(path: string) {
		if (propertyType === 'javascript' && monaco) {
			propPickerWrapperContext?.onPick?.(path)
		} else {
			connectProperty(path)
		}
		dispatch('change', { argName })
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

		if (schemaProperty?.hideWhenChatEnabled && chatInputEnabled) {
			if (!hidden) {
				hidden = true
				if (arg) {
					arg.value = undefined
					arg.expr = undefined
				}
				inputCheck = true
			}
			return
		}

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

	function updatePropsBeingEdited(focused: boolean) {
		if (!exprBeingEdited) return
		let newPropsBeingEdited = [...$exprBeingEdited]
		if (focused) {
			newPropsBeingEdited.push(argName)
		} else {
			newPropsBeingEdited = newPropsBeingEdited.filter((p) => p !== argName)
		}
		if (!deepEqual(newPropsBeingEdited, $exprBeingEdited)) {
			exprBeingEdited.set(newPropsBeingEdited)
		}
	}

	// The column beside a settings row delivers here rather than through the host's `select`
	// handler, which can only reach a mounted expression editor. A collapsed setting has no
	// field at all, so it gives the target up and the column closes with it.
	$effect(() => {
		if (pickerMode !== 'sidePane') return
		propPickerWrapperContext?.setPickTarget?.(
			collapsed ? undefined : { id: argName, onSelect: pickIntoArg }
		)
	})

	onDestroy(() => {
		updatePropsBeingEdited(false)
		if (pickerMode === 'sidePane') {
			propPickerWrapperContext?.setPickTarget?.(undefined)
		}
	})

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
		propertyType: InputTransform['type'],
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

	let resourceTypes: string[] | undefined = $state(undefined)

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

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
		schema?.properties?.[argName]?.default && untrack(() => setDefaultCode())
	})
	$effect.pre(() => {
		// Monitor changes that affect field visibility
		JSON.stringify(schema)
		JSON.stringify(arg)
		JSON.stringify(otherArgs)

		untrack(() => handleFieldVisibility(schema, arg, otherArgs))
	})
	let connecting = $derived($propPickerConfig?.propName == argName)
	let fieldDescription = $derived(
		hideDescription ? undefined : schema?.properties?.[argName]?.description
	)
	// Fading the controls away is only safe while the row itself says what it holds. An expression
	// or an AI-filled value is only legible from the toggle, so those keep it on screen.
	let controlsPinned = $derived(connecting || propertyType !== 'static' || Boolean(suggestion))
	// Its picker builds an expression, so it goes with the expression option.
	let shouldShowS3ArrayHelper = $derived(
		inputCat === 'list' &&
			!noJavascript &&
			['s3object', 's3_object'].includes(schema?.properties?.[argName]?.items?.resourceType)
	)

	// Svelte bug ...
	// Somehow the value is updated in the UI of the parent, but not in the children
	// when passed as a prop. setTimeout is a workaround to force the update
	let visiblePropertyType = $state(untrack(() => (suggestion ? 'javascript' : propertyType)))
	$effect(() => {
		let value = suggestion ? 'javascript' : propertyType
		setTimeout(() => (visiblePropertyType = value), 1)
	})
</script>

{#if (arg != undefined || collapsed) && !hidden}
	<div class={twMerge('relative group flex flex-col gap-1', className)}>
		<!-- `relative` so the absolute button cluster below anchors to this row rather than
		     to the whole field, letting it share the label's baseline. `w-full` so an
		     `align-items` on the caller's class can't shrink the row to its label and pull
		     `right-0` onto it. -->
		<div class="relative w-full flex flex-row flex-wrap justify-between gap-1">
			<!-- min-h-7 reserves room for the button cluster beside a plain label; a custom
			     header is a control of its own and sets the row's height itself. -->
			<div class="flex grow items-end {header ? '' : 'min-h-7'}">
				{#if header}
					{@render header()}
				{:else}
					<FieldHeader
						label={label ?? argName}
						simpleTooltip={headerTooltip}
						simpleTooltipIconClass={headerTooltipIconClass}
						SimpleTooltipIcon={HeaderTooltipIcon}
						format={schema?.properties?.[argName]?.format}
						contentEncoding={schema?.properties?.[argName]?.contentEncoding}
						required={schema.required?.includes(argName)}
						type={schema.properties?.[argName]?.type}
					/>

					{@render labelExtra?.()}

					{#if staticTemplateOffered}
						<div>
							<span
								class="border text-gray-400 dark:text-gray-500 text-2xs font-medium mr-2 px-1 !py-[1px] rounded ml-2.5 {propertyType ==
									'static' && arg?.type === 'javascript'
									? 'visible'
									: 'invisible'}"
							>
								{'${...}'}
							</span>
						</div>
					{/if}
				{/if}
			</div>
			<!-- Nothing to connect to or switch while collapsed: there is no value yet. -->
			<div
				class={twMerge(
					'flex flex-row items-end gap-x-2 z-10 absolute right-0 bottom-0 group-hover:bg-surface transition-colors',
					collapsed ? 'hidden' : '',
					subtleControls && !controlsPinned
						? 'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity'
						: ''
				)}
			>
				{#if aiGen}
					{@render aiGen()}
				{:else if enableAi && !noJavascript}
					<StepInputGen
						bind:this={stepInputGen}
						{focused}
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
					/>
				{/if}

				{#if propPickerWrapperContext && !noConnect}
					<FlowPlugConnect
						wrapperClasses={twMerge(
							'group-hover:opacity-100 transition-opacity',
							!connecting ? 'opacity-0' : ''
						)}
						id="flow-editor-plug"
						{connecting}
						on:click={() => {
							if ($propPickerConfig?.propName == argName) {
								clearFocus()
							} else {
								focusProp?.(argName, (path) => {
									if (pickerMode === 'sidePane') {
										pickIntoArg(path)
									} else {
										connectProperty(path)
										dispatch('change', { argName })
									}
									return true
								})
							}
						}}
					/>
				{/if}

				{#if !noDynamicToggle}
					<div class="{ButtonType.UnifiedHeightClasses.xs} relative">
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
												? `\`${escapeTemplateBackticks(arg?.value?.toString() ?? '')}\``
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
											// Stated here, as the other branches state it. `setPropertyType`
											// only writes a type when the text is an interpolation, so leaving
											// it to that call means a field switched off `ai` keeps carrying
											// `ai` and reads straight back as it on the next render.
											arg.type = 'static'
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
									// On a field the agent can fill, "static with no value" is itself the
									// AI state (see `getPropertyType`), so leaving the value unset reads
									// this choice straight back as AI and the field can never be typed
									// into. An empty value of the field's own kind is what makes "I will
									// supply this one" representable.
									if (fieldAllowsAi && arg && arg.value === undefined) {
										arg.value = isStaticTemplate(inputCat) ? '' : null
									}
									propertyType = 'static'
								}
							}}
						>
							{#snippet children({ item })}
								{#if fieldAllowsAi}
									<!-- `h-full`, as its siblings have: the group is a row shorter than a `sm`
									     button, and without it this one stands proud of the others. -->
									<ToggleButton
										size="sm"
										label="AI"
										value="ai"
										tooltip="Let the AI agent fill this field dynamically"
										{item}
										class="h-full text-xs"
									/>
								{/if}

								{#if staticTemplateOffered}
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

								{#if noJavascript && propertyType !== 'javascript'}
									<!-- nothing: the expression option is not offered here -->
								{:else if codeInjectionDetected && propertyType == 'static'}
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
				{/if}
			</div>
		</div>

		{#if !collapsed}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<!-- A custom header means a setting's toggle owns this field, so the input is
			     indented under the toggle's label: `xs` switch (w-7) plus its ml-2. -->
			<div
				class="relative w-full {header ? 'pl-9' : ''}"
				onkeyup={handleKeyUp}
				transition:slideDynamic|global={{ duration: animateAppear ? 150 : 0 }}
			>
				<!-- {inputCat}
			{propertyType} -->
				<div class="relative flex flex-row items-top gap-1 justify-between">
					<div class="min-w-0 grow">
						<!-- The ghost text covers the input and only the input: it has to stay mounted
						     (Monaco, focus, Tab) and keep its height, or what follows slides up
						     underneath it — and the overlay must not reach the rows below, which is
						     what would bury the Help dropdown. -->
						<div class="relative">
							{#if suggestion}
								<div
									class={`absolute inset-0 z-10 bg-surface-input rounded-md pl-2 overflow-auto ${inputBorderClass({ forceFocus: true })}`}
								>
									<FakeMonacoPlaceHolder autoheight code={suggestion} fontSize={12} />
								</div>
							{/if}
							<div
								class={suggestion ? 'opacity-0' : ''}
								onkeydowncapture={(e) => {
									if (e.key === 'Tab' && suggestion) {
										e.preventDefault()
									}
								}}
							>
								{@render innerInput()}
							</div>
						</div>

						<InputError {error} />

						<!-- Rendered outside the `suggestion ? opacity-0` wrapper so the AI
					     step-input autocompletion (ghost text, accepted with Tab) doesn't
					     hide the Help dropdown — the two stay independent. -->
						{#if !hideHelpButton && propertyType === 'javascript' && argKind === 'javascript' && arg.expr != undefined}
							<DynamicInputHelpBox />
						{/if}

						{#snippet innerInput()}
							{#if propertyType === 'ai'}
								<div
									class="text-sm text-tertiary italic p-3 bg-surface-secondary rounded-md border border-gray-200"
								>
									<span class="flex items-center gap-2 text-xs">
										<InfoIcon size={13} />
										This field will be filled by the AI agent dynamically
									</span>
								</div>
								{#if fieldDescription}
									<div class="text-xs italic py-1 text-hint">
										<pre class="font-main whitespace-normal">
										{fieldDescription}
									</pre>
									</div>
								{/if}
							{:else if staticTemplateOffered && propertyType == 'static' && !noDynamicToggle}
								<div class="flex flex-col gap-1">
									{#if fieldDescription}
										<div class="text-xs text-secondary">
											<pre class="font-main whitespace-normal">
										{fieldDescription}
										</pre>
										</div>
									{/if}

									{#if arg}
										<TemplateEditor
											bind:this={monacoTemplate}
											{extraLib}
											minRows={schema?.properties?.[argName]?.minRows}
											placeholder={schema?.properties?.[argName]?.placeholder}
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
								<ArgInput
									{resourceTypes}
									{workspace}
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
									bind:description={
										() => fieldDescription,
										(v) => {
											const property = schema.properties?.[argName]
											if (!hideDescription && property) property.description = v
										}
									}
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
									{chatInputEnabled}
									otherArgs={Object.fromEntries(
										Object.entries(otherArgs).map(([key, transform]) => [
											key,
											transform?.type === 'static'
												? transform.value
												: transform?.type === 'javascript'
													? transform.expr
													: undefined
										])
									)}
								>
									{#snippet innerBottomSnippet()}
										{#if shouldShowS3ArrayHelper}
											<S3ArrayHelperButton
												{connecting}
												onClick={() =>
													switchToJsAndConnect((path) =>
														appendPathToArrayExpr(arg?.type === 'javascript' ? arg.expr : '', path)
													)}
											/>
										{/if}
									{/snippet}
								</ArgInput>
							{:else if argKind === 'javascript' && arg.expr != undefined}
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<!-- Reaching for the editor reveals the properties beside it. On pointerdown,
								     not focus: an editor that was never blurred emits no focus event, so a
								     column dismissed while it kept the caret could not be brought back. -->
								<div
									onpointerdown={() => openPicker?.()}
									class={`bg-surface-input rounded-md flex flex-col pl-2 overflow-auto ${inputBorderClass({ forceFocus: focused, error: !!error })}`}
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
										on:focus={() => {
											focused = true
											updatePropsBeingEdited(true)
											openPicker?.()
										}}
										on:blur={() => {
											focused = false
											updatePropsBeingEdited(false)
										}}
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
											focusProp?.(argName, (path) => {
												appendPathToArrayExpr(arg.expr, path)
												return true
											})}
									/>
								{/if}

								{#if fieldDescription}
									<div class="text-xs italic py-1 text-secondary">
										<pre class="font-main whitespace-normal">{fieldDescription}</pre>
									</div>
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
		{/if}
	</div>
{/if}
