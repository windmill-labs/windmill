<script lang="ts">
	import type { Schema } from '$lib/common'
	import { VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'

	import { Pen, Plus, X } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { twMerge } from 'tailwind-merge'
	import Section from './Section.svelte'
	import FlowPropertyEditor from './schema/FlowPropertyEditor.svelte'
	import PropertyEditor from './schema/PropertyEditor.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import { createEventDispatcher } from 'svelte'

	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Label from './Label.svelte'
	import { sendUserToast } from '$lib/toast'
	import Toggle from './Toggle.svelte'
	import { emptyString } from '$lib/utils'
	import Popup from './common/popup/Popup.svelte'

	export let schema: Schema | any
	export let schemaSkippedValues: string[] = []
	export let args: Record<string, any> = {}

	export let shouldHideNoInputs: boolean = false
	export let noVariablePicker = false
	export let flexWrap = false
	export let noDelete = false
	export let uiOnly: boolean = false
	export let isFlowInput: boolean = false
	export let noPreview: boolean = false
	export let offset = 48 + 31 + 31 + 16 + 1

	const dispatch = createEventDispatcher()

	function changePosition(i: number, up: boolean): any {
		const entries = Object.entries(schema.properties)
		var element = entries[i]
		entries.splice(i, 1)
		entries.splice(up ? i - 1 : i + 1, 0, element)
		schema.properties = Object.fromEntries(entries)
		syncOrders()
	}

	function syncOrders() {
		if (schema) {
			schema.order = Object.keys(schema.properties ?? {})
		}
	}

	let clazz: string = ''
	export { clazz as class }

	$: if (args == undefined || typeof args !== 'object') {
		args = {}
	}

	export function setDefaults() {
		const nargs = {}

		Object.keys(schema?.properties ?? {}).forEach((key) => {
			if (schema?.properties[key].default != undefined && args[key] == undefined) {
				nargs[key] = schema?.properties[key].default
			}
		})
		args = nargs
	}

	function removeExtraKey() {
		const nargs = {}
		Object.keys(args ?? {}).forEach((key) => {
			if (keys.includes(key)) {
				nargs[key] = args[key]
			}
		})
		args = nargs
	}

	let pickForField: string | undefined
	let itemPicker: ItemPicker | undefined = undefined
	let variableEditor: VariableEditor | undefined = undefined

	let keys: string[] = []

	$: {
		let lkeys = Object.keys(schema?.properties ?? {})
		if (schema?.properties && JSON.stringify(lkeys) != JSON.stringify(keys)) {
			keys = lkeys
			if (!noDelete) {
				removeExtraKey()
			}
		}
	}

	reorder()

	function reorder() {
		if (schema?.order && Array.isArray(schema.order)) {
			const n = {}

			;(schema.order as string[]).forEach((x) => {
				if (schema.properties && schema.properties[x] != undefined) {
					n[x] = schema?.properties[x]
				}
			})

			Object.keys(schema.properties ?? {})
				.filter((x) => !schema.order?.includes(x))
				.forEach((x) => {
					n[x] = schema.properties[x]
				})
			schema.properties = n
		}
	}

	let opened: string | undefined = undefined

	let selected = ''

	export function openField(key: string) {
		opened = key
	}

	$: if (opened === undefined && keys.length > 0) {
		opened = keys[0]
	}

	$: if (opened && schema.properties[opened]) {
		selected = opened
			? schema.properties[opened].type !== 'object'
				? schema.properties[opened].type
				: schema.properties[opened].format === 'resource-s3_object'
				? 'S3'
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

			schema = { ...schema }
			sendUserToast('Argument renamed successfully')
		}
	}

	let jsonView: boolean = false
	let schemaString: string = JSON.stringify(schema, null, '\t')
	let error: string | undefined = undefined
	let editor: SimpleEditor | undefined = undefined
</script>

<div style={offset ? `height: calc(100vh - ${offset}px);` : 'h-full'}>
	<Splitpanes>
		{#if !noPreview}
			<Pane size={50} minSize={20}>
				<div class="p-4">
					<Section
						label="Form preview"
						tooltip={'Preview of the form that will be rendered based on the schema. Drag and drop to reorder the fields.'}
					>
						<SchemaForm
							{schema}
							bind:args
							dndEnabled
							on:click={(e) => {
								opened = e.detail
							}}
							on:reorder={() => reorder()}
						/>
					</Section>
				</div>
			</Pane>
		{/if}
		<Pane size={noPreview ? 100 : 50} minSize={noPreview ? 100 : 20}>
			<div class="w-full p-2 flex justify-end">
				<Toggle
					bind:checked={jsonView}
					label="JSON View"
					size="xs"
					options={{
						left: 'Rich Editor',
						leftTooltip: 'View the schema in a rich editor',
						right: 'JSON Editor',
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

			{#if !jsonView}
				<div class="w-full {clazz} {flexWrap ? 'flex flex-row flex-wrap gap-x-6 ' : ''} divide-y">
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
									{argName}
									{#if schema.required?.includes(argName)}
										<span class="text-red-500 text-xs"> Required </span>
									{/if}

									{#if !uiOnly}
										<div class="flex flex-row gap-2 items-center justify-center">
											<Popup
												floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
												containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
												let:close
											>
												<svelte:fragment slot="button">
													<Button color="light" size="xs" nonCaptureEvent startIcon={{ icon: Pen }}>
														Rename
													</Button>
												</svelte:fragment>
												<Label label="Name">
													<div class="flex flex-col gap-2">
														<input
															type="text"
															class="w-full !bg-surface"
															value={argName}
															id={argName + i}
														/>
														<Button
															variant="border"
															color="light"
															size="xs"
															on:click={() => {
																renameProperty(argName, argName + i)
																close(null)
															}}
														>
															Rename
														</Button>
													</div>
												</Label>
											</Popup>
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
										</div>
									{/if}
								</div>
								{#if opened === argName}
									<div class="p-4 border-t">
										{#if !schemaSkippedValues.includes(argName) && Object.keys(schema?.properties ?? {}).includes(argName)}
											{#if typeof args == 'object' && schema?.properties[argName]}
												<PropertyEditor
													bind:description={schema.properties[argName].description}
													type={schema.properties[argName].type}
													bind:pattern={schema.properties[argName].pattern}
													bind:enum_={schema.properties[argName].enum}
													bind:format={schema.properties[argName].format}
													bind:contentEncoding={schema.properties[argName].contentEncoding}
													bind:customErrorMessage={schema.properties[argName].customErrorMessage}
													bind:itemsType={schema.properties[argName].items}
													on:changePosition={(event) =>
														changePosition(event.detail.i, event.detail.up)}
													bind:extra={schema.properties[argName]}
													bind:title={schema.properties[argName].title}
													bind:placeholder={schema.properties[argName].placeholder}
													bind:properties={schema.properties[argName].properties}
													{isFlowInput}
												>
													<svelte:fragment slot="typeeditor">
														{#if isFlowInput}
															<Label label="Type">
																<ToggleButtonGroup
																	bind:selected
																	on:selected={(e) => {
																		const isS3 = e.detail == 'S3'

																		selected = e.detail

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
																			...(e.detail == 'array' ? { items: { type: 'string' } } : {}),
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
																		} else {
																			schema.properties[argName] = {
																				...emptyProperty,
																				format: undefined,
																				type: e.detail
																			}
																		}
																	}}
																>
																	{#each [['String', 'string'], ['Number', 'number'], ['Integer', 'integer'], ['Object', 'object'], ['Array', 'array'], ['Boolean', 'boolean'], ['S3 Object', 'S3']] as x}
																		<ToggleButton value={x[1]} label={x[0]} />
																	{/each}
																</ToggleButtonGroup>
															</Label>
														{/if}
													</svelte:fragment>

													{#if isFlowInput}
														<FlowPropertyEditor
															bind:defaultValue={schema.properties[argName].default}
															{variableEditor}
															{itemPicker}
															bind:nullable={schema.properties[argName].nullable}
															type={schema.properties[argName].type}
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
															on:requiredChange={(event) => {
																if (event.detail.required) {
																	schema.required = schema.required ?? []
																	schema.required.push(argName)
																} else {
																	schema.required = schema.required?.filter((x) => x !== argName)
																}
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
		</Pane>
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
				New Variable
			</Button>
		</div>
	</ItemPicker>

	<VariableEditor bind:this={variableEditor} />
{/if}
