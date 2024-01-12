<script lang="ts">
	import type { Schema } from '$lib/common'
	import { VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import ArgInput from './ArgInput.svelte'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'

	import { getResourceTypes } from './resourceTypesStore'
	import { Plus } from 'lucide-svelte'
	import { flip } from 'svelte/animate'

	export let schema: Schema | any
	export let schemaSkippedValues: string[] = []
	export let schemaFieldTooltip: Record<string, string> = {}
	export let args: Record<string, any> = {}

	export let autofocus = false

	export let shouldHideNoInputs: boolean = false
	export let compact = false
	export let noVariablePicker = false
	export let flexWrap = false
	export let noDelete = false
	export let prettifyHeader = false
	export let disablePortal = false

	const moveAnimationDuration = 1000

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

	let resourceTypes: string[] | undefined = undefined

	async function loadResourceTypes() {
		resourceTypes = await getResourceTypes()
	}

	reorder()
	loadResourceTypes()

	function reorder() {
		if (schema?.order && Array.isArray(schema.order)) {
			const n = {}

			;(schema.order as string[]).forEach((x) => {
				n[x] = schema.properties[x]
			})

			Object.keys(schema.properties ?? {})
				.filter((x) => !schema.order?.includes(x))
				.forEach((x) => {
					n[x] = schema.properties[x]
				})
			schema.properties = n
		}
	}
</script>

<div class="w-full {clazz} {flexWrap ? 'flex flex-row flex-wrap gap-x-6 ' : ''}">
	{#if keys.length > 0}
		{#each keys as argName, i (argName)}
			<div animate:flip={{ duration: moveAnimationDuration }} class={'pb-6'}>
				{#if !schemaSkippedValues.includes(argName) && Object.keys(schema?.properties ?? {}).includes(argName)}
					{#if typeof args == 'object' && schema?.properties[argName]}
						<ArgInput
							{disablePortal}
							{resourceTypes}
							{prettifyHeader}
							autofocus={i == 0 && autofocus}
							label={argName}
							bind:description={schema.properties[argName].description}
							bind:value={args[argName]}
							type={schema.properties[argName].type}
							required={schema.required?.includes(argName) ?? false}
							bind:pattern={schema.properties[argName].pattern}
							bind:valid={inputCheck[argName]}
							defaultValue={schema.properties[argName].default}
							bind:enum_={schema.properties[argName].enum}
							bind:format={schema.properties[argName].format}
							bind:contentEncoding={schema.properties[argName].contentEncoding}
							bind:customErrorMessage={schema.properties[argName].customErrorMessage}
							properties={schema.properties[argName].properties}
							nestedRequired={schema.properties[argName].required}
							bind:itemsType={schema.properties[argName].items}
							editableSchema={{ i, total: keys.length }}
							on:changePosition={(event) => changePosition(event.detail.i, event.detail.up)}
							{compact}
							{variableEditor}
							{itemPicker}
							bind:pickForField
							bind:extra={schema.properties[argName]}
							simpleTooltip={schemaFieldTooltip[argName]}
						/>
					{/if}
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
