<script lang="ts">
	import type { Schema } from '$lib/common'
	import { VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { allTrue, computeShow, generateRandomString } from '$lib/utils'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { getResourceTypes } from './resourceTypesStore'
	import { GripVertical, Plus } from 'lucide-svelte'
	import ArgInput from './ArgInput.svelte'
	import { SOURCES, TRIGGERS, dndzone } from 'svelte-dnd-action'
	import { flip } from 'svelte/animate'
	import { createEventDispatcher } from 'svelte'

	export let schema: Schema | any
	export let schemaSkippedValues: string[] = []
	export let schemaFieldTooltip: Record<string, string> = {}
	export let args: Record<string, any> = {}
	export let disabledArgs: string[] = []
	export let disabled = false

	export let isValid: boolean = true
	export let autofocus = false

	export let shouldHideNoInputs: boolean = false
	export let compact = false
	export let linkedSecret: string | undefined = undefined
	export let linkedSecretCandidates: string[] | undefined = undefined
	export let noVariablePicker = false
	export let flexWrap = false
	export let noDelete = false
	export let prettifyHeader = false
	export let disablePortal = false
	export let showSchemaExplorer = false
	export let showReset = false
	export let onlyMaskPassword = false
	export let dndType: string | undefined = undefined
	export let dndEnabled: boolean = false

	let dragDisabled: boolean = false

	const flipDurationMs = 200
	const dispatch = createEventDispatcher()

	let clazz: string = ''
	export { clazz as class }

	let inputCheck: { [id: string]: boolean } = {}

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

	let keys: string[] = []

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
	let items: Array<{
		value: string
		id: string
	}> = []

	$: {
		let lkeys = schema?.order ?? Object.keys(schema?.properties ?? {})
		if (schema?.properties && JSON.stringify(lkeys) != JSON.stringify(keys)) {
			keys = lkeys

			items = keys.map((item, index) => {
				return { value: item, id: generateRandomString() }
			})
			if (!noDelete) {
				removeExtraKey()
			}
		}
	}

	$: isValid = allTrue(inputCheck ?? {})

	let resourceTypes: string[] | undefined = undefined

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

	loadResourceTypes()

	$: schema && reorder()

	// Reorder the schema properties according to the order array
	function reorder() {
		if (schema?.order && Array.isArray(schema.order)) {
			const n = {}

			;(schema.order as string[]).forEach((x) => {
				if (schema.properties && schema.properties[x] != undefined) {
					n[x] = schema.properties[x]
				}
			})

			Object.keys(schema.properties ?? {})
				.filter((x) => !schema.order?.includes(x))
				.forEach((x) => {
					n[x] = schema.properties[x]
				})
			schema.properties = n
			keys = Object.keys(schema.properties ?? {})
		}
	}

	function handleConsider(e) {
		const {
			items: newItems,
			info: { source, trigger }
		} = e.detail

		items = newItems
		// Ensure dragging is stopped on drag finish via keyboard
		if (source === SOURCES.KEYBOARD && trigger === TRIGGERS.DRAG_STOPPED) {
			dragDisabled = true
		}
	}

	function handleFinalize(e) {
		const {
			items: newItems,
			info: { source }
		} = e.detail

		items = newItems

		if (source === SOURCES.POINTER) {
			dragDisabled = true
		}

		keys = items.map((item) => item.value)

		schema.properties = keys.reduce((acc, key) => {
			acc[key] = schema.properties[key]
			return acc
		}, {})

		schema.order = keys

		schema = { ...schema }

		dispatch('reorder', keys)
	}

	function startDrag(e) {
		// preventing default to prevent lag on touch devices (because of the browser checking for screen scrolling)
		e.preventDefault()
		dragDisabled = false
	}

	function handleKeyDown(e) {
		if ((e.key === 'Enter' || e.key === ' ') && dragDisabled) dragDisabled = false
	}
</script>

{#if showReset}
	<div class="flex flex-row-reverse w-full">
		<Button size="xs" color="light" on:click={() => setDefaults()}
			>Reset args to runnable's defaults</Button
		>
	</div>
{/if}

<div
	class="w-full {clazz} {flexWrap ? 'flex flex-row flex-wrap gap-x-6 ' : ''}"
	use:dndzone={{
		items,
		dragDisabled: dragDisabled || !dndEnabled,
		flipDurationMs,
		dropTargetStyle: {},
		type: dndEnabled ? dndType ?? 'top-level' : 'dnd-disabled'
	}}
	on:consider={handleConsider}
	on:finalize={handleFinalize}
>
	{#if items.length > 0}
		{#each items as item, i (item.id)}
			{@const argName = item.value}

			<div animate:flip={{ duration: 200 }}>
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				{#if !schemaSkippedValues.includes(argName) && Object.keys(schema?.properties ?? {}).includes(argName)}
					<!-- svelte-ignore a11y-no-static-element-interactions -->
					<div
						class="flex flex-row items-center bg-surface"
						on:click={() => {
							dispatch('click', argName)
						}}
					>
						{#if typeof args == 'object' && schema?.properties[argName]}
							{#if computeShow(argName, schema?.properties[argName].showExpr, args)}
								<ArgInput
									{disablePortal}
									{resourceTypes}
									{prettifyHeader}
									autofocus={i == 0 && autofocus ? true : null}
									label={argName}
									description={schema.properties[argName].description}
									bind:value={args[argName]}
									type={schema.properties[argName].type}
									required={schema.required?.includes(argName)}
									pattern={schema.properties[argName].pattern}
									bind:valid={inputCheck[argName]}
									defaultValue={schema.properties[argName].default}
									enum_={schema.properties[argName].enum}
									format={schema.properties[argName].format}
									contentEncoding={schema.properties[argName].contentEncoding}
									customErrorMessage={schema.properties[argName].customErrorMessage}
									bind:properties={schema.properties[argName].properties}
									bind:order={schema.properties[argName].order}
									nestedRequired={schema.properties[argName].required}
									itemsType={schema.properties[argName].items}
									disabled={disabledArgs.includes(argName) || disabled}
									{compact}
									{variableEditor}
									{itemPicker}
									bind:pickForField
									password={linkedSecret == argName}
									extra={schema.properties[argName]}
									{showSchemaExplorer}
									simpleTooltip={schemaFieldTooltip[argName]}
									{onlyMaskPassword}
									nullable={schema.properties[argName].nullable}
									title={schema.properties[argName].title}
									placeholder={schema.properties[argName].placeholder}
									{dndEnabled}
								>
									<svelte:fragment slot="actions">
										{#if linkedSecretCandidates?.includes(argName)}
											<div>
												<ToggleButtonGroup
													selected={linkedSecret == argName}
													on:selected={(e) => {
														if (e.detail) {
															linkedSecret = argName
														} else if (linkedSecret == argName) {
															linkedSecret = undefined
														}
													}}
												>
													<ToggleButton
														value={false}
														size="sm"
														label="Inlined"
														tooltip="The value is inlined in the resource and thus has no special treatment."
													/>
													<ToggleButton
														position="right"
														value={true}
														size="sm"
														label="Secret"
														tooltip="The value will be stored in a newly created linked secret variable at the same path. That variable can be permissioned differently, will be treated as a secret the UI, operators will not be able to load it and every access will generate a corresponding audit log."
													/>
												</ToggleButtonGroup>
											</div>{/if}</svelte:fragment
									>
								</ArgInput>
								<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
								<!-- svelte-ignore a11y-no-static-element-interactions -->
								{#if dndEnabled}
									<div
										tabindex={dragDisabled ? 0 : -1}
										class="w-4 h-4 cursor-move ml-2"
										on:mousedown={startDrag}
										on:touchstart={startDrag}
										on:keydown={handleKeyDown}
									>
										<GripVertical size={16} />
									</div>
								{/if}
							{/if}
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	{:else if !shouldHideNoInputs}
		<div class="text-secondary text-sm">No inputs</div>
	{/if}
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
