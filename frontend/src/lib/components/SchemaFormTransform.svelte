<script lang="ts">
	import type { Schema } from '$lib/common'
	import { VariableService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { allTrue } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { slide } from 'svelte/transition'
	import { Button } from './common'
	import InputTransformForm from './InputTransformForm.svelte'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'

	export let schema: Schema
	export let args: Record<string, InputTransform | any> = {}
	export let isValid: boolean = true
	export let extraLib: string = 'missing extraLib'
	export let previousModuleId: string | undefined = undefined

	export let shouldHideNoInputs: boolean = false
	export let noVariablePicker = false
	export let filter: string[] | undefined = undefined
	export let noDynamicToggle = false
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
		Object.keys(args ?? {}).forEach((key) => {
			if (!keys.includes(key)) {
				delete args[key]
				delete inputCheck[key]
			}
		})
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
			{#if !filter || filter.includes(argName)}
				<div transition:slide|local>
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
						{noDynamicToggle}
					/>
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
				args[pickForField].value = '$var:' + path
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
