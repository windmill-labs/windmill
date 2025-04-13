<script lang="ts">
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
	import { createEventDispatcher, tick } from 'svelte'
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
	import type { SchemaDiff } from '$lib/components/schema/schemaUtils'
	import type { EditableSchemaFormUi } from '$lib/components/custom_ui'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	export let schema: Schema | any
	export let hiddenArgs: string[] = []
	export let args: Record<string, any> = {}
	export let shouldHideNoInputs: boolean = false
	export let noVariablePicker = false
	export let flexWrap = false
	export let uiOnly: boolean = false
	export let isFlowInput: boolean = false
	export let noPreview: boolean = false
	export let jsonEnabled: boolean = true
	export let isAppInput: boolean = false
	export let displayWebhookWarning: boolean = false
	export let onlyMaskPassword: boolean = false
	export let dndType: string | undefined = undefined
	export let editTab:
		| 'inputEditor'
		| 'history'
		| 'savedInputs'
		| 'json'
		| 'captures'
		| 'firstStepInputs'
		| undefined
	export let previewSchema: Record<string, any> | undefined = undefined
	export let editPanelInitialSize: number | undefined = undefined
	export let editPanelSize = 0
	export let diff: Record<string, SchemaDiff> = {}
	export let disableDnd: boolean = false
	export let shouldDispatchChanges: boolean = false
	export let isValid: boolean = true
	export let customUi: EditableSchemaFormUi | undefined = undefined

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	let clazz: string = ''
	export { clazz as class }

	$: if (args == undefined || typeof args !== 'object') {
		args = {}
	}

	export function setDefaults() {
		const nargs = {}

		Object.keys(schema?.properties ?? {}).forEach((key) => {
			if (schema?.properties[key].default != undefined && args[key] == undefined) {
				let value = schema?.properties[key].default
				nargs[key] = value === 'object' ? structuredClone(value) : value
			}
		})
		args = nargs
	}

	let pickForField: string | undefined
	let itemPicker: ItemPicker | undefined = undefined
	let variableEditor: VariableEditor | undefined = undefined

	let keys: string[] = Array.isArray(schema?.order)
		? [...schema.order]
		: (Object.keys(schema?.properties ?? {}) ?? Object.keys(schema?.properties ?? {}))

	$: schema && onSchemaChange()

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

	let opened: string | undefined = keys[0]
	let selected = ''

	export function openField(key: string) {
		opened = key
	}

	export function deleteField(key: string) {
		delete args[key]
		delete schema.properties[key]
		if (schema.required?.includes(key)) {
			schema.required = schema.required?.filter((x) => x !== key)
		}
		if (schema.order) {
			schema.order = schema.order.filter((x) => x !== key)
		}
	}

	$: opened && updateSelected(schema.properties[opened])
	function updateSelected(property: any) {
		if (!property) return
		selected = opened
			? property.type !== 'object'
				? property.type
				: property.format === 'resource-s3_object'
					? 'S3'
					: property.oneOf && property.oneOf.length >= 2
						? 'oneOf'
						: 'object'
			: ''
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
			args[newName] = args[oldName]
			delete args[oldName]

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

			schema = schema
			dispatch('change', schema)
			sendUserToast('Argument renamed successfully')
		}
	}

	let jsonView: boolean = customUi?.jsonOnly == true
	let schemaString: string = JSON.stringify(schema, null, '\t')
	let error: string | undefined = undefined
	let editor: SimpleEditor | undefined = undefined

	const editTabDefaultSize = noPreview ? 100 : 50
	editPanelSize = editTab ? (editPanelInitialSize ?? editTabDefaultSize) : 0
	let inputPanelSize = 100 - editPanelSize
	let editPanelSizeSmooth = tweened(editPanelSize, {
		duration: 150
	})
	let inputPanelSizeSmooth = tweened(inputPanelSize, { duration: 150 })

	function openEditTab() {
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
		dispatchIfMounted('editPanelSizeChanged', editSize)
	}
	$: updatePanelSizes($editPanelSizeSmooth, $inputPanelSizeSmooth)

	$: !!editTab ? openEditTab() : closeEditTab()

	let panelButtonWidth: number = 0
	export let pannelExtraButtonWidth: number = 0

	export function updateJson() {
		schemaString = JSON.stringify(schema, null, '\t')
		editor?.setCode(schemaString)
	}
</script>

<div class="w-full h-full">
	<div class="relative z-[100000]">
		<div
			class="absolute"
			style="right: calc({editPanelSize}% - 1px - {pannelExtraButtonWidth}px); top: 0px;"
			bind:clientWidth={panelButtonWidth}
		>
			<slot name="openEditTab" />
		</div>
	</div>
	<Splitpanes class="splitter-hidden w-full">
		{#if !noPreview}
			<Pane bind:size={inputPanelSize} minSize={20}>
				<div
					class="h-full flex flex-col gap-2 {$$slots.openEditTab && editPanelSize > 0
						? 'pr-[38px]'
						: 'pr-2'}"
				>
					{#if $$slots.addProperty}
						<div class="w-full justify-left pr-2 grow-0">
							<div
								style={editPanelSize > 0
									? `width: 100%;`
									: `width: calc(100% - ${panelButtonWidth - pannelExtraButtonWidth}px);`}
							>
								<slot name="addProperty" />
							</div>
						</div>
					{/if}

					<div
						class="min-h-0 overflow-y-auto grow rounded-md {$$slots.runButton
							? 'flex flex-col gap-2'
							: ''}"
					>
						<SchemaFormDnd
							nestedClasses={'flex flex-col gap-1'}
							schema={previewSchema ? previewSchema : schema}
							{dndType}
							{disableDnd}
							{onlyMaskPassword}
							bind:args
							on:click={(e) => {
								opened = e.detail
							}}
							on:reorder={(e) => {
								schema.order = e.detail
								schema = schema
								tick().then(() => dispatch('change', schema))
							}}
							on:change={() => {
								schema = schema
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

						<slot name="runButton" />
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
					<slot name="extraTab" />
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
										<!-- svelte-ignore a11y-click-events-have-key-events -->
										<!-- svelte-ignore a11y-no-static-element-interactions -->
										<div
											class={twMerge(
												'w-full flex bg-gray-50 dark:bg-gray-800 px-4 py-1 justify-between items-center hover:bg-gray-100 cursor-pointer',
												opened === argName ? 'bg-gray-100 hover:bg-gray-200' : ''
											)}
											on:click={() => {
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
													<div on:click|stopPropagation|preventDefault>
														<Popover placement="bottom-end" containerClasses="p-4" closeButton>
															<svelte:fragment slot="trigger">
																<Button
																	color="light"
																	size="xs2"
																	nonCaptureEvent
																	startIcon={{ icon: Pen }}
																	iconOnly
																/>
															</svelte:fragment>
															<svelte:fragment slot="content" let:close>
																<Label label="Name" class="p-4">
																	<div class="flex flex-col gap-2">
																		<input
																			type="text"
																			class="w-full !bg-surface"
																			value={argName}
																			id={argName + i}
																			on:keydown={(event) => {
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
															</svelte:fragment>
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
													on:click={() => {
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
																schema = schema
																dispatch('change', schema)
															}}
														>
															<svelte:fragment slot="typeeditor">
																{#if isFlowInput || isAppInput}
																	<Label label="Type">
																		<ToggleButtonGroup
																			tabListClass="flex-wrap"
																			class="h-auto"
																			let:item
																			bind:selected
																			on:selected={(e) => {
																				const isS3 = e.detail == 'S3'
																				const isOneOf = e.detail == 'oneOf'

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
																					...(e.detail == 'array'
																						? { items: { type: 'string' } }
																						: {}),
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
																						type: e.detail
																					}
																				}
																				// No better solution than this, needs future rework
																				setTimeout(() => {
																					schema = schema
																					dispatch('change', schema)
																				}, 100)
																				dispatch('schemaChange')
																			}}
																		>
																			{#each [['String', 'string'], ['Number', 'number'], ['Integer', 'integer'], ['Object', 'object'], ['OneOf', 'oneOf'], ['Array', 'array'], ['Boolean', 'boolean'], ['S3 Object', 'S3']] as x}
																				<ToggleButton value={x[1]} label={x[0]} {item} />
																			{/each}
																		</ToggleButtonGroup>
																	</Label>
																{/if}
															</svelte:fragment>

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
																		schema = schema
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
				args[pickForField] = '$var:' + path
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
		<div slot="submission">
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
