<script lang="ts">
	import type { Schema } from '$lib/common'
	import { VariableService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { allTrue } from '$lib/utils'
	import { slide } from 'svelte/transition'
	import ArgInput from './ArgInput.svelte'
	import { Button } from './common'
	import InputTransformForm from './InputTransformForm.svelte'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'

	export let inputTransform = false
	export let schema: Schema
	export let args: Record<string, InputTransform | any> = {}
	export let disabledArgs: string[] = []
	export let disabled = false

	export let editableSchema = false
	export let isValid: boolean = true
	export let extraLib: string = 'missing extraLib'
	export let autofocus = false
	export let previousModuleId: string | undefined = undefined

	export let shouldHideNoInputs: boolean = false
	export let compact = false
	export let password: string | undefined = undefined

	let clazz: string = ''
	export { clazz as class }

	let inputCheck: { [id: string]: boolean } = {}
	$: isValid = allTrue(inputCheck) ?? false

	$: if (args == undefined || typeof args !== 'object') {
		args = {}
	}

	function removeExtraKey() {
		Object.keys(args ?? {}).forEach((key) => {
			if (!Object.keys(schema?.properties ?? {}).includes(key)) {
				delete args[key]
				delete inputCheck[key]
			}
		})
	}

	$: schema?.properties && removeExtraKey()

	let pickForField: string | undefined
	let itemPicker: ItemPicker
	let variableEditor: VariableEditor
</script>

<div class="w-full {clazz}">
	{#if Object.keys(schema?.properties ?? {}).length > 0}
		{#each Object.keys(schema?.properties ?? {}) as argName, i (argName)}
			<div transition:slide|local>
				{#if inputTransform}
					<InputTransformForm
						{previousModuleId}
						bind:arg={args[argName]}
						bind:schema
						bind:argName
						bind:inputCheck={inputCheck[argName]}
						bind:extraLib
						{variableEditor}
						{itemPicker}
						bind:pickForField
					/>
				{:else if typeof args == 'object'}
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
					/>
				{:else}
					Expected args to be an object, got {JSON.stringify(args)} instead
				{/if}
			</div>
		{/each}
	{:else if !shouldHideNoInputs}
		<div class="text-gray-500 text-sm">No inputs</div>
	{/if}
</div>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, _) => {
		if (pickForField) {
			if (inputTransform) {
				args[pickForField].value = '$var:' + path
			} else {
				args[pickForField] = '$var:' + path
			}
		}
	}}
	itemName="Variable"
	extraField="name"
	loadItems={async () =>
		(await VariableService.listVariable({ workspace: $workspaceStore ?? '' })).map((x) => ({
			name: x.path,
			...x
		}))}
>
	<div
		slot="submission"
		class="flex flex-row-reverse w-full p-5 bg-white border-t border-gray-200 rounded-bl-lg rounded-br-lg"
	>
		<Button
			variant="border"
			color="blue"
			size="sm"
			on:click={() => {
				variableEditor?.initNew?.()
			}}
		>
			Create a new variable
		</Button>
	</div>
</ItemPicker>

<VariableEditor bind:this={variableEditor} on:create={itemPicker.openDrawer} />
