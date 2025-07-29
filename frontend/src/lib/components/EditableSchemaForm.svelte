<script lang="ts">
	import { createBubbler, preventDefault, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import type { Schema } from '$lib/common'
	import { VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import { Pen, Plus, X } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { twMerge } from 'tailwind-merge'
	import FlowPropertyEditor from './schema/FlowPropertyEditor.svelte'
	import PropertyEditor from './schema/PropertyEditor.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import { createEventDispatcher, tick, untrack } from 'svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Label from './Label.svelte'
	import { sendUserToast } from '$lib/toast'
	import Toggle from './Toggle.svelte'
	import { emptyString } from '$lib/utils'
	import Popover from './meltComponents/Popover.svelte'
	import SchemaFormDnd from './schema/SchemaFormDND.svelte'
	import { deepEqual } from 'fast-equals'
	import { tweened } from 'svelte/motion'
	import type { SchemaDiff } from '$lib/components/schema/schemaUtils.svelte'
	import type { EditableSchemaFormUi } from '$lib/components/custom_ui'

	// export let openEditTab: () => void = () => {}
	const dispatch = createEventDispatcher()

	interface Props {
		schema: Schema | any
		hiddenArgs?: string[]
		args?: Record<string, any>
		shouldHideNoInputs?: boolean
		noVariablePicker?: boolean
		flexWrap?: boolean
		uiOnly?: boolean
		isFlowInput?: boolean
		noPreview?: boolean
		jsonEnabled?: boolean
		isAppInput?: boolean
		displayWebhookWarning?: boolean
		onlyMaskPassword?: boolean
		dndType?: string | undefined
		editTab:
			| 'inputEditor'
			| 'history'
			| 'savedInputs'
			| 'json'
			| 'captures'
			| 'firstStepInputs'
			| undefined
		previewSchema?: Record<string, any> | undefined
		editPanelInitialSize?: number | undefined
		editPanelSize?: number
		diff?: Record<string, SchemaDiff>
		disableDnd?: boolean
		shouldDispatchChanges?: boolean
		isValid?: boolean
		customUi?: EditableSchemaFormUi | undefined
		pannelExtraButtonWidth?: number
		class?: string
		openEditTab?: import('svelte').Snippet
		addProperty?: import('svelte').Snippet
		runButton?: import('svelte').Snippet
		extraTab?: import('svelte').Snippet
	}

	let {
		schema = $bindable(),
		hiddenArgs = [],
		args = $bindable(undefined),
		shouldHideNoInputs = false,
		noVariablePicker = false,
		flexWrap = false,
		uiOnly = false,
		isFlowInput = false,
		noPreview = false,
		jsonEnabled = true,
		isAppInput = false,
		displayWebhookWarning = false,
		onlyMaskPassword = false,
		dndType = undefined,
		editTab,
		previewSchema = undefined,
		editPanelInitialSize = undefined,
		editPanelSize = $bindable(0),
		diff = {},
		disableDnd = false,
		shouldDispatchChanges = false,
		isValid = $bindable(true),
		customUi = undefined,
		pannelExtraButtonWidth = 0,
		class: clazz = '',
		openEditTab,
		addProperty,
		runButton,
		extraTab
	}: Props = $props()

	$effect.pre(() => {
		if (args == undefined) {
			args = {}
		}
	})

	export function setDefaults() {
		const nargs = {}

		Object.keys(schema?.properties ?? {}).forEach((key) => {
			if (schema?.properties[key].default != undefined && args?.[key] == undefined) {
				let value = schema?.properties[key].default
				nargs[key] = value === 'object' ? structuredClone($state.snapshot(value)) : value
			}
		})
		args = nargs
	}

	let pickForField: string | undefined
	let itemPicker: ItemPicker | undefined = $state(undefined)
	let variableEditor: VariableEditor | undefined = $state(undefined)

	let keys: string[] = $state(
		Array.isArray(schema?.order)
			? [...schema.order]
			: (Object.keys(schema?.properties ?? {}) ?? Object.keys(schema?.properties ?? {}))
	)

	function alignOrderWithProperties(schema: {
		properties: Record<string, any>
		order: string[] | undefined
	}) {
		if (schema.order == undefined && !Array.isArray(schema.order)) {
			schema.order = []
		}
		let norder = [...schema.order]
		let properties = Object.keys(schema?.properties ?? {})
		let index = 0
		let hasChanged = false
		for (let k of properties) {
			if (schema.properties[k].type === 'object' && schema.properties[k].properties) {
				hasChanged = hasChanged || alignOrderWithProperties(schema.properties[k])
			}
			if (!norder.includes(k)) {
				norder = [...norder.slice(0, index), k, ...norder.slice(index)]
			}
			index += 1
		}
		norder = norder.filter((x) => properties.includes(x))
		if (!deepEqual(schema.order, norder)) {
			schema.order = norder
			return true
		}
		return hasChanged
	}
	function onSchemaChange() {
		let editSchema = false
		if (alignOrderWithProperties(schema)) {
			editSchema = true
		}
		let lkeys = schema?.order ?? Object.keys(schema?.properties ?? {})
		if (schema?.properties && !deepEqual(lkeys, keys)) {
			keys = [...lkeys]
			editSchema = true
			if (opened == undefined) {
				opened = keys[0]
			}
		}
		if (editSchema) {
			schema = schema
		}
	}

	let opened: string | undefined = $state(untrack(() => keys[0]))

	function computeSelected(property: any) {
		if (!opened) return ''
		return property.type !== 'object'
			? property.type
			: property.format === 'resource-s3_object'
				? 'S3'
				: property.format?.startsWith('dynselect-')
					? 'dynselect'
					: property.oneOf && property.oneOf.length >= 2
						? 'oneOf'
						: 'object'
	}

	export function openField(key: string) {
		opened = key
	}

	export function deleteField(key: string) {
		delete args?.[key]
		delete schema.properties[key]
		if (schema.required?.includes(key)) {
			schema.required = schema.required?.filter((x) => x !== key)
		}
		if (schema.order) {
			schema.order = schema.order.filter((x) => x !== key)
		}
	}

	function renameProperty(oldName: string, key: string) {
		const el = document.getElementById(key) as HTMLInputElement
		const newName = el.value
		if (oldName === newName) {
			return
		}

		if (Object.keys(schema.properties ?? {}).includes(newName)) {
			sendUserToast('There is already an argument with this name', true)

			// clear the input
			el.value = oldName
		} else {
			if (args) {
				args[newName] = args[oldName]
				delete args[oldName]
			}

			schema.properties[newName] = schema.properties[oldName]
			delete schema.properties[oldName]

			if (schema.required?.includes(oldName)) {
				schema.required = schema.required?.map((x) => (x === oldName ? newName : x))
			}

			// Replace the old name with the new name in the order array
			if (schema.order) {
				const index = schema.order.indexOf(oldName)
				if (index !== -1) {
					schema.order[index] = newName
				}
			}

			opened = newName

			schema = $state.snapshot(schema)
			dispatch('change', schema)
			sendUserToast('Argument renamed')
		}
	}

	let jsonView: boolean = $state(customUi?.jsonOnly == true)
	let schemaString: string = $state(JSON.stringify(schema, null, '\t'))
	let error: string | undefined = $state(undefined)
	let editor: SimpleEditor | undefined = $state(undefined)

	export function updateJson() {
		schemaString = JSON.stringify(schema, null, '\t')
		editor?.setCode(schemaString)
	}

	const editTabDefaultSize = noPreview ? 100 : 50
	editPanelSize = editTab ? (editPanelInitialSize ?? editTabDefaultSize) : 0
	let inputPanelSize = $state(100 - editPanelSize)
	let editPanelSizeSmooth = tweened(editPanelSize, {
		duration: 150
	})
	let inputPanelSizeSmooth = tweened(
		untrack(() => inputPanelSize),
		{ duration: 150 }
	)

	function openEditTabFn() {
		if (editPanelSize > 0) return
		editPanelSizeSmooth.set(editTabDefaultSize)
		inputPanelSizeSmooth.set(100 - editTabDefaultSize)
	}

	function closeEditTab() {
		editPanelSizeSmooth.set(0)
		inputPanelSizeSmooth.set(100)
	}

	function updatePanelSizes(editSize: number, inputSize: number) {
		editPanelSize = editSize
		inputPanelSize = inputSize
		dispatch('editPanelSizeChanged', editSize)
	}

	let panelButtonWidth: number = $state(0)
	$effect(() => {
		if (args == undefined || typeof args !== 'object') {
			args = {}
		}
	})
	$effect(() => {
		schema && untrack(() => onSchemaChange())
	})
	$effect(() => {
		updatePanelSizes($editPanelSizeSmooth, $inputPanelSizeSmooth)
	})
	$effect(() => {
		!!editTab ? openEditTabFn() : closeEditTab()
	})
</script>

<div class="w-full h-full">
	<div class="relative z-[100000]">
		<div
			class="absolute"
			style="right: calc({editPanelSize}% - 1px - {pannelExtraButtonWidth}px); top: 0px;"
			bind:clientWidth={panelButtonWidth}
		>
			{@render openEditTab?.()}
		</div>
	</div>
	<Splitpanes class="splitter-hidden w-full">
		{#if !noPreview}
			<Pane bind:size={inputPanelSize} minSize={20}>
				<div
					class="h-full flex flex-col gap-2 {openEditTab && editPanelSize > 0
						? 'pr-[38px]'
						: 'pr-2'}"
				>
					{#if addProperty}
						<div class="w-full justify-left pr-2 grow-0">
							<div
								style={editPanelSize > 0
									? `width: 100%;`
									: `width: calc(100% - ${panelButtonWidth - pannelExtraButtonWidth}px);`}
							>
								{@render addProperty?.()}
							</div>
						</div>
					{/if}

					<div
						class="min-h-0 overflow-y-auto grow rounded-md {runButton ? 'flex flex-col gap-2' : ''}"
					>
						<SchemaFormDnd
							nestedClasses={'flex flex-col gap-1'}
							bind:schema={
								() => (previewSchema ? previewSchema : schema),
								(newSchema) => {
									schema = newSchema
									tick().then(() => dispatch('change', schema))
								}
							}
							{dndType}
							{disableDnd}
							{onlyMaskPassword}
							bind:args
							on:click={(e) => {
								opened = e.detail
							}}
							on:reorder={(e) => {
								schema = {
									...schema,
									order: e.detail
								}
								tick().then(() => dispatch('change', schema))
							}}
							prettifyHeader={isAppInput}
							disabled={!!previewSchema}
							{diff}
							on:acceptChange
							on:rejectChange
							on:nestedChange={() => {
								dispatch('change', schema)
							}}
							{shouldDispatchChanges}
							bind:isValid
							noVariablePicker={noVariablePicker || customUi?.disableVariablePicker === true}
						/>

						{@render runButton?.()}
					</div>
				</div>
			</Pane>
		{/if}

		{#if editPanelSize > 0}
			<Pane
				bind:size={editPanelSize}
				minSize={noPreview ? 100 : 50}
				class={twMerge('border rounded-md', panelButtonWidth > 0 ? 'rounded-tl-none' : '')}
			>
				{#if editTab !== 'inputEditor'}
					{@render extraTab?.()}
				{:else}
					<!-- WIP -->
					{#if jsonEnabled && customUi?.jsonOnly != true}
						<div class="w-full p-3 flex justify-end">
							<Toggle
								bind:checked={jsonView}
								label="JSON View"
								size="xs"
								options={{
									right: 'JSON editor',
									rightTooltip:
										'Arguments can be edited either using the wizard, or by editing their JSON Schema.'
								}}
								lightMode
								on:change={() => {
									schemaString = JSON.stringify(schema, null, '\t')
									editor?.setCode(schemaString)
								}}
							/>
						</div>
					{/if}

					{#if !jsonView}
						<div
							class="w-full {clazz} {flexWrap ? 'flex flex-row flex-wrap gap-x-6 ' : ''} divide-y"
						>
							{#if keys.length > 0}
								{#each keys as argName, i (argName)}
									<div>
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											class={twMerge(
												'w-full flex bg-gray-50 dark:bg-gray-800 px-4 py-1 justify-between items-center hover:bg-gray-100 cursor-pointer',
												opened === argName ? 'bg-gray-100 hover:bg-gray-200' : ''
											)}
											onclick={() => {
												if (opened === argName) {
													opened = undefined
												} else {
													opened = argName
												}
											}}
										>
											<div class="flex flex-row gap-2">
												{argName}
												{#if !uiOnly}
													<div onclick={stopPropagation(preventDefault(bubble('click')))}>
														<Popover placement="bottom-end" containerClasses="p-4" closeButton>
															{#snippet trigger()}
																<Button
																	color="light"
																	size="xs2"
																	nonCaptureEvent
																	startIcon={{ icon: Pen }}
																	iconOnly
																/>
															{/snippet}
															{#snippet content({ close })}
																<Label label="Name" class="p-4">
																	<div class="flex flex-col gap-2">
																		<input
																			type="text"
																			class="w-full !bg-surface"
																			value={argName}
																			id={argName + i}
																			onkeydown={(event) => {
																				if (event.key === 'Enter') {
																					renameProperty(argName, argName + i)
																					close()
																				}
																			}}
																		/>
																		<Button
																			variant="border"
																			color="light"
																			size="xs"
																			on:click={() => {
																				renameProperty(argName, argName + i)
																				close()
																			}}
																		>
																			Rename
																		</Button>
																	</div>
																</Label>
															{/snippet}
														</Popover>
													</div>
												{/if}
											</div>

											{#if schema.required?.includes(argName)}
												<span class="text-red-500 text-xs"> Required </span>
											{/if}

											{#if !uiOnly}
												<button
													class="rounded-full p-1 text-gray-500 bg-white
				duration-200 hover:bg-gray-600 focus:bg-gray-600 hover:text-white dark:bg-gray-700 dark:text-white dark:hover:bg-gray-800"
													aria-label="Clear"
													onclick={() => {
														dispatch('delete', argName)
													}}
												>
													<X size={16} />
												</button>
											{/if}
										</div>
										{#if opened === argName}
											<div class="p-4 border-t">
												{#if !hiddenArgs.includes(argName) && Object.keys(schema?.properties ?? {}).includes(argName)}
													{#if typeof args == 'object' && schema?.properties[argName]}
														<PropertyEditor
															bind:description={schema.properties[argName].description}
															type={schema.properties[argName].type}
															bind:oneOf={schema.properties[argName].oneOf}
															bind:pattern={schema.properties[argName].pattern}
															bind:enum_={schema.properties[argName].enum}
															bind:format={schema.properties[argName].format}
															bind:contentEncoding={schema.properties[argName].contentEncoding}
															bind:customErrorMessage={
																schema.properties[argName].customErrorMessage
															}
															bind:itemsType={schema.properties[argName].items}
															bind:extra={schema.properties[argName]}
															bind:title={schema.properties[argName].title}
															bind:placeholder={schema.properties[argName].placeholder}
															bind:properties={schema.properties[argName].properties}
															bind:order={schema.properties[argName].order}
															{isFlowInput}
															{isAppInput}
															on:change={() => {
																schema = $state.snapshot(schema)
																dispatch('change', schema)
															}}
														>
															{#snippet typeeditor()}
																{#if isFlowInput || isAppInput}
																	<Label label="Type">
																		<ToggleButtonGroup
																			tabListClass="flex-wrap"
																			class="h-auto"
																			bind:selected={
																				() => computeSelected(schema.properties[opened ?? '']),
																				(v) => {
																					const isS3 = v == 'S3'
																					const isOneOf = v == 'oneOf'
																					const isDynSelect = v == 'dynselect'

																					const emptyProperty = {
																						contentEncoding: undefined,
																						enum_: undefined,
																						pattern: undefined,
																						default: undefined,
																						min: undefined,
																						max: undefined,
																						currency: undefined,
																						currencyLocale: undefined,
																						multiselect: undefined,
																						password: undefined,
																						dateFormat: undefined,
																						...(v == 'array' ? { items: { type: 'string' } } : {}),
																						showExpr: undefined,
																						nullable: undefined,
																						required: undefined
																					}

																					if (isS3) {
																						schema.properties[argName] = {
																							...emptyProperty,
																							type: 'object',
																							format: 'resource-s3_object'
																						}
																					} else if (isDynSelect) {
																						schema.properties[argName] = {
																							...emptyProperty,
																							type: 'object',
																							format: 'dynselect-main'
																						}
																					} else if (isOneOf) {
																						schema.properties[argName] = {
																							...emptyProperty,
																							type: 'object',
																							oneOf: [
																								{
																									title: 'Option 1',
																									type: 'object',
																									properties: {
																										label: {
																											type: 'string',
																											enum: ['Option 1']
																										},
																										property_1: {
																											type: 'string'
																										}
																									}
																								},
																								{
																									title: 'Option 2',
																									type: 'object',
																									properties: {
																										label: {
																											type: 'string',
																											enum: ['Option 2']
																										},
																										property_2: {
																											type: 'string'
																										}
																									}
																								}
																							]
																						}
																					} else {
																						schema.properties[argName] = {
																							...emptyProperty,
																							format: undefined,
																							type: v
																						}
																					}
																				}
																			}
																			on:selected={(e) => {
																				schema = schema
																				dispatch('change', schema)
																				dispatch('schemaChange')
																			}}
																		>
																			{#snippet children({ item })}
																				{#each [['String', 'string'], ['Number', 'number'], ['Integer', 'integer'], ['Object', 'object'], ['OneOf', 'oneOf'], ['Array', 'array'], ['Boolean', 'boolean'], ['S3 Object', 'S3'], ['DynSelect', 'dynselect']] as x}
																					<ToggleButton value={x[1]} label={x[0]} {item} />
																				{/each}
																			{/snippet}
																		</ToggleButtonGroup>
																	</Label>
																{/if}
															{/snippet}

															{#if isFlowInput || isAppInput}
																<FlowPropertyEditor
																	bind:defaultValue={schema.properties[argName].default}
																	{variableEditor}
																	{itemPicker}
																	bind:nullable={schema.properties[argName].nullable}
																	bind:disabled={schema.properties[argName].disabled}
																	type={schema.properties[argName].type}
																	bind:oneOf={schema.properties[argName].oneOf}
																	bind:format={schema.properties[argName].format}
																	contentEncoding={schema.properties[argName].contentEncoding}
																	required={schema.required?.includes(argName) ?? false}
																	pattern={schema.properties[argName].pattern}
																	password={schema.properties[argName].password}
																	propsNames={schema.properties[argName].propsNames}
																	bind:showExpr={schema.properties[argName].showExpr}
																	extra={schema.properties[argName]}
																	customErrorMessage={schema.properties[argName].customErrorMessage}
																	itemsType={schema.properties[argName].items}
																	bind:properties={schema.properties[argName].properties}
																	bind:order={schema.properties[argName].order}
																	bind:requiredProperty={schema.properties[argName].required}
																	{displayWebhookWarning}
																	on:requiredChange={(event) => {
																		if (event.detail.required) {
																			schema.required = schema.required ?? []
																			schema.required.push(argName)
																		} else {
																			schema.required = schema.required?.filter(
																				(x) => x !== argName
																			)
																		}
																		dispatch('change', schema)
																	}}
																	on:schemaChange={(e) => {
																		schema = $state.snapshot(schema)
																		dispatch('change', schema)
																		dispatch('schemaChange')
																	}}
																/>
															{/if}
														</PropertyEditor>
													{/if}
												{/if}
											</div>
										{/if}
									</div>
								{/each}
							{:else if !shouldHideNoInputs}
								<div class="text-secondary text-sm p-2">No inputs</div>
							{/if}
						</div>
					{:else}
						<div class="p-2">
							<div class="border rounded h-full">
								<SimpleEditor
									bind:this={editor}
									small
									fixedOverflowWidgets={false}
									on:change={() => {
										try {
											schema = JSON.parse(schemaString)
											dispatch('change', schema)
											error = ''
										} catch (err) {
											error = err.message
										}
									}}
									bind:code={schemaString}
									lang="json"
									autoHeight
									automaticLayout
								/>
							</div>
							{#if !emptyString(error)}
								<div class="text-red-400 text-xs">{error}</div>
							{:else}
								<div><br /> </div>
							{/if}
						</div>
					{/if}
				{/if}
			</Pane>
		{/if}
	</Splitpanes>
</div>

{#if !noVariablePicker}
	<ItemPicker
		bind:this={itemPicker}
		pickCallback={(path, _) => {
			if (pickForField) {
				if (args) {
					args[pickForField] = '$var:' + path
				}
			}
		}}
		itemName="Variable"
		tooltip="Variables are dynamic values that have a key associated to them and can be retrieved during the execution of a Script or Flow."
		documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets"
		extraField="path"
		loadItems={async () =>
			(await VariableService.listVariable({ workspace: $workspaceStore ?? '' })).map((x) => ({
				name: x.path,
				...x
			}))}
	>
		{#snippet submission()}
			<div>
				<Button
					variant="border"
					color="blue"
					size="sm"
					startIcon={{ icon: Plus }}
					on:click={() => variableEditor?.initNew?.()}
				>
					New variable
				</Button>
			</div>
		{/snippet}
	</ItemPicker>

	<VariableEditor bind:this={variableEditor} />
{/if}

<style>
	:global(.splitter-hidden .splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		opacity: 0 !important;
	}
</style>
