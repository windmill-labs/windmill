<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { InputTransform } from '$lib/gen'
	import { allTrue } from '$lib/utils'
	import { slide } from 'svelte/transition'
	import ArgInput from './ArgInput.svelte'
	import InputTransformForm from './InputTransformForm.svelte'

	export let inputTransform = false
	export let schema: Schema
	export let args: Record<string, InputTransform | any> = {}
	export let disabledArgs: string[] = []

	export let editableSchema = false
	export let isValid: boolean = true
	export let extraLib: string = 'missing extraLib'
	export let importPath: string | undefined = undefined
	export let autofocus = false
	export let animateNew = false

	let clazz: string = ''
	export { clazz as class }

	let inputCheck: { [id: string]: boolean } = {}
	$: isValid = allTrue(inputCheck) ?? false

	$: if (args == undefined) {
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
</script>

<div class="w-full {clazz}">
	{#if Object.keys(schema?.properties ?? {}).length > 0}
		{#each Object.keys(schema?.properties ?? {}) as argName, i (argName)}
			<div transition:slide|local>
				{#if inputTransform}
					<InputTransformForm
						bind:arg={args[argName]}
						bind:schema
						bind:argName
						bind:inputCheck={inputCheck[argName]}
						bind:extraLib
						bind:importPath
					/>
				{:else}
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
						disabled={disabledArgs.includes(argName)}
						{editableSchema}
					/>
				{/if}
			</div>
		{/each}
	{:else}
		<p class="italic text-sm">No inputs</p>
	{/if}
</div>
