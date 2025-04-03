<script lang="ts">
	import type { Schema } from '$lib/common'
	import { VariableService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { allTrue } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { Button } from './common'
	import StepInputsGen from './copilot/StepInputsGen.svelte'
	import type { PickableProperties } from './flows/previousResults'
	import InputTransformForm from './InputTransformForm.svelte'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import { Plus } from 'lucide-svelte'

	export let schema: Schema | { properties?: Record<string, any> }
	export let args: Record<string, InputTransform | any> = {}

	export let isValid: boolean = true
	export let extraLib: string = 'missing extraLib'
	export let previousModuleId: string | undefined = undefined

	export let filter: string[] | undefined = undefined
	export let noDynamicToggle = false
	export let pickableProperties: PickableProperties | undefined = undefined
	export let enableAi = false

	let clazz: string = ''
	export { clazz as class }

	let inputCheck: { [id: string]: boolean } = {}

	const dispatch = createEventDispatcher()

	$: isValid = allTrue(inputCheck) ?? false

	$: if (args == undefined || typeof args !== 'object') {
		args = {}
	}

	export function setArgs(nargs: Record<string, InputTransform | any>) {
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
			removeExtraKey()
		}
	}
</script>

<div class="w-full {clazz}">
	{#if enableAi}
		<div class="px-0.5 pt-0.5">
			<StepInputsGen
				{pickableProperties}
				argNames={keys
					? keys.filter(
							(argName) =>
								Object.keys(schema.properties ?? {}).includes(argName) &&
								Object.keys(args ?? {}).includes(argName) &&
								((args[argName].type === 'static' && !args[argName].value) ||
									(args[argName].type === 'javascript' && !args[argName].expr))
						)
					: []}
				{schema}
			/>
		</div>
	{/if}
	{#if keys.length > 0}
		{#each keys as argName (argName)}
			{#if (!filter || filter.includes(argName)) && Object.keys(schema.properties ?? {}).includes(argName)}
				<div class="pt-2 relative">
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
						{pickableProperties}
						{enableAi}
						on:change={(e) => {
							const { argName } = e.detail
							dispatch('changeArg', { argName })
						}}
					/>
				</div>
			{/if}
		{/each}
	{:else}
		<div class="text-tertiary text-sm">No inputs</div>
	{/if}
</div>

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
		class="flex flex-row-reverse w-full border-t border-gray-200 rounded-bl-lg rounded-br-lg"
	>
		<Button
			variant="border"
			color="blue"
			size="sm"
			startIcon={{ icon: Plus }}
			on:click={() => {
				variableEditor?.initNew?.()
			}}
		>
			New variable
		</Button>
	</div>
</ItemPicker>

<VariableEditor bind:this={variableEditor} />
