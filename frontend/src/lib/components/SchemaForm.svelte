<script lang="ts">
	import type { Schema } from '$lib/common'
	import { VariableService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { allTrue } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import ArgInput from './ArgInput.svelte'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'

	export let schema: Schema
	export let args: Record<string, InputTransform | any> = {}
	export let disabledArgs: string[] = []
	export let disabled = false

	export let editableSchema = false
	export let isValid: boolean = true
	export let autofocus = false

	export let shouldHideNoInputs: boolean = false
	export let compact = false
	export let password: string | undefined = undefined
	export let noVariablePicker = false
	export let flexWrap = false
	export let noDelete = false

	let clazz: string = ''
	export { clazz as class }

	let inputCheck: { [id: string]: boolean } = {}
	$: isValid = allTrue(inputCheck) ?? false

	$: if (args == undefined || typeof args !== 'object') {
		args = {}
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
</script>

<div class="w-full {clazz} {flexWrap ? 'flex flex-row flex-wrap gap-x-6 gap-y-2' : ''}">
	{#if keys.length > 0}
		{#each keys as argName, i (argName)}
			{#if Object.keys(schema.properties ?? {}).includes(argName)}
				<div>
					{#if typeof args == 'object' && schema?.properties[argName]}
						{#if editableSchema}
							<ArgInput
								autofocus={i == 0 && autofocus}
								label={argName}
								bind:description={schema.properties[argName].description}
								bind:value={args[argName]}
								type={schema.properties[argName].type}
								required={schema.required.includes(argName)}
								bind:pattern={schema.properties[argName].pattern}
								bind:valid={inputCheck[argName]}
								defaultValue={schema.properties[argName].default}
								bind:enum_={schema.properties[argName].enum}
								bind:format={schema.properties[argName].format}
								contentEncoding={schema.properties[argName].contentEncoding}
								properties={schema.properties[argName].properties}
								bind:itemsType={schema.properties[argName].items}
								disabled={disabledArgs.includes(argName) || disabled}
								{editableSchema}
								{compact}
								password={argName == password}
								{variableEditor}
								{itemPicker}
								bind:pickForField
								bind:extra={schema.properties[argName]}
							/>
						{:else}
							<ArgInput
								autofocus={i == 0 && autofocus}
								label={argName}
								description={schema.properties[argName].description}
								bind:value={args[argName]}
								type={schema.properties[argName].type}
								required={schema.required.includes(argName)}
								pattern={schema.properties[argName].pattern}
								bind:valid={inputCheck[argName]}
								defaultValue={schema.properties[argName].default}
								enum_={schema.properties[argName].enum}
								format={schema.properties[argName].format}
								contentEncoding={schema.properties[argName].contentEncoding}
								properties={schema.properties[argName].properties}
								itemsType={schema.properties[argName].items}
								disabled={disabledArgs.includes(argName) || disabled}
								{editableSchema}
								{compact}
								password={argName == password}
								{variableEditor}
								{itemPicker}
								bind:pickForField
								extra={schema.properties[argName]}
							/>
						{/if}
					{/if}
				</div>
			{/if}
		{/each}
	{:else if !shouldHideNoInputs}
		<div class="text-gray-500 text-sm">No inputs</div>
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
		extraField="path"
		loadItems={async () =>
			(await VariableService.listVariable({ workspace: $workspaceStore ?? '' })).map((x) => ({
				name: x.path,
				...x
			}))}
	>
		<div
			slot="submission"
			class="flex flex-row-reverse w-full bg-white border-t border-gray-200 rounded-bl-lg rounded-br-lg"
		>
			<Button
				variant="border"
				color="blue"
				size="sm"
				startIcon={{ icon: faPlus }}
				on:click={() => {
					variableEditor?.initNew?.()
				}}
			>
				New variable
			</Button>
		</div>
	</ItemPicker>

	<VariableEditor bind:this={variableEditor} />
{/if}
