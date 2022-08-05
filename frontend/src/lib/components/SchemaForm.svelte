<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { InputTransform } from '$lib/gen'
	import { allTrue } from '$lib/utils'
	import ArgInput from './ArgInput.svelte'
	import InputTransformForm from './InputTransformForm.svelte'

	export let inputTransform = false
	export let schema: Schema
	export let args: Record<string, InputTransform | any> = {}
	export let editableSchema = false
	export let isValid: boolean = true
	export let pickableProperties: Object | undefined = undefined
	export let extraLib: string = 'missing extraLib'
	export let i: number | undefined = undefined

	let inputCheck: { [id: string]: boolean } = {}
	$: isValid = allTrue(inputCheck) ?? false

	function removeExtraKey() {
		Object.keys(args).forEach((key) => {
			if (!Object.keys(schema?.properties ?? {}).includes(key)) {
				delete args[key]
			}
		})
	}

	$: Object.keys(schema?.properties ?? {}).length > 0 && removeExtraKey()
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
					bind:pickableProperties
					bind:extraLib
					bind:i
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
					bind:itemsType={schema.properties[argName].items}
					{editableSchema}
				/>
			{/if}
		{/each}
	{:else}
		<p class="italic text-sm">No settable input</p>
	{/if}
</div>
