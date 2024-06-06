<script lang="ts">
	import type { Schema } from '$lib/common'
	import { VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'

	import { Plus, X } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { twMerge } from 'tailwind-merge'
	import Section from './Section.svelte'
	import FlowPropertyEditor from './schema/FlowPropertyEditor.svelte'
	import PropertyEditor from './schema/PropertyEditor.svelte'

	import { createEventDispatcher } from 'svelte'

	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Label from './Label.svelte'

	export let schema: Schema | any
	export let schemaSkippedValues: string[] = []
	export let args: Record<string, any> = {}

	export let shouldHideNoInputs: boolean = false
	export let noVariablePicker = false
	export let flexWrap = false
	export let noDelete = false
	// 48: Drawer header, 31: 1st Tab header, 31: 2nd Tab header, 16: mt-4, 1: border
	export let offset = 48 + 31 + 31 + 16 + 1
	export let uiOnly: boolean = false
	export let isFlowInput: boolean = false
	export let noPreview: boolean = false

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

	let wrapperHeight: number | undefined = undefined

	$: opened = keys?.[0] as string | undefined

	let selected = ''
</script>

<div style={`height: calc(100vh - ${offset}px);`} bind:clientHeight={wrapperHeight}>
	<Splitpanes>
		{#if !noPreview}
			<Pane size={50} minSize={20}>
				<div class="p-4" style={`height: ${wrapperHeight}px; overflow-y: auto;`}>
					<Section
						label="Form preview"
						tooltip={'Preview of the form that will be rendered based on the schema. Drag and drop to reorder the fields.'}
					>
						<SchemaForm {schema} bind:args dndEnabled />
					</Section>
				</div>
			</Pane>
		{/if}
		<Pane size={noPreview ? 100 : 50} minSize={noPreview ? 100 : 20}>
			<div
				class="w-full {clazz} {flexWrap
					? 'flex flex-row flex-wrap gap-x-6 '
					: ''} divide-y overflow-y-auto"
				style={`height: ${wrapperHeight}px;`}
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
								{argName}
								{#if schema.required?.includes(argName)}
									<span class="text-red-500 text-xs"> Required </span>
								{/if}
								{#if !uiOnly}
									<div class="flex flex-row gap-1 items-center justify-center">
										<button
											class="rounded-full p-1 text-gray-500 bg-white
			duration-200 hover:bg-gray-600 focus:bg-gray-600 hover:text-white"
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
												editableSchema={{ i, total: keys.length }}
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
