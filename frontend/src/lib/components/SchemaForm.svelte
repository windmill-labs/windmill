<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { InputTransform } from '$lib/gen'
	import { allTrue } from '$lib/utils'
	import { getContext, hasContext } from 'svelte'
	import ArgInput from './ArgInput.svelte'
	import type { PropPickerWrapperContext } from './flows/propPicker/PropPickerWrapper.svelte'
	import InputTransformForm from './InputTransformForm.svelte'

	export let inputTransform = false
	export let schema: Schema
	export let args: Record<string, InputTransform | any> = {}
	export let editableSchema = false
	export let isValid: boolean = true
	export let extraLib: string = 'missing extraLib'
	export let importPath: string | undefined = undefined

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

	function getFocusFunction() {
		if (hasContext('PropPickerWrapper')) {
			const { focus } = getContext<PropPickerWrapperContext>('PropPickerWrapper')
			return focus
		}
		return (value: string | undefined) => {}
	}

	const focus = getFocusFunction()
</script>

<div class="w-full">
	{#if Object.keys(schema?.properties ?? {}).length > 0}
		{#each Object.keys(schema?.properties ?? {}) as argName}
			{#if inputTransform}
				<InputTransformForm
					bind:arg={args[argName]}
					bind:schema
					bind:argName
					bind:inputCheck
					bind:extraLib
					bind:importPath
				/>
			{:else}
				<ArgInput
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
					on:focus={() => focus && focus(argName)}
					{editableSchema}
				/>
			{/if}
		{/each}
	{:else}
		<p class="italic text-sm">No settable input</p>
	{/if}
</div>
