<script lang="ts">
	import type { Schema } from '$lib/common'
	import { InputTransform } from '$lib/gen'
	import { allTrue } from '$lib/utils'
	import ArgInput from './ArgInput.svelte'
	import Editor from './Editor.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import DynamicInputHelpBox from './flows/DynamicInputHelpBox.svelte'
	import { getCodeInjectionExpr, getDefaultExpr, isCodeInjection } from './flows/utils'
	import OverlayPropertyPicker from './propertyPicker/OverlayPropertyPicker.svelte'
	import Toggle from './Toggle.svelte'

	export let inputTransform = false
	export let schema: Schema
	export let args: Record<string, any> = {}
	export let editableSchema = false
	export let extraLib: string
	export let isValid: boolean = true

	export let i: number
	export let previousSchema: Object

	let inputCheck: { [id: string]: boolean } = {}

	$: isValid = allTrue(inputCheck) ?? false
	$: types = Object.keys(schema?.properties ?? {}).map((prop, index) => {
		const arg = args[prop]

		if (isCodeInjection(arg.value)) {
			args[prop].expr = getCodeInjectionExpr(arg.value)
			args[prop].type = InputTransform.type.JAVASCRIPT
			return InputTransform.type.STATIC
		} else {
			if (
				args[prop].type === InputTransform.type.JAVASCRIPT &&
				types?.at(index) === InputTransform.type.STATIC
			) {
				args[prop].type = InputTransform.type.STATIC
			}
		}

		return arg.type
	})
</script>

<div class="w-full">
	{#if Object.keys(schema?.properties ?? {}).length > 0}
		{#each Object.keys(schema?.properties ?? {}) as argName, index}
			{#if inputTransform && args[argName] != undefined}
				<div class={index > 0 ? 'mt-8' : ''} />
				<div class="flex justify-between items-center">
					<div class="flex items-center">
						<FieldHeader
							label={argName}
							format={schema.properties[argName].format}
							contentEncoding={schema.properties[argName].contentEncoding}
							required={schema.required.includes(argName)}
							type={schema.properties[argName].type}
							itemsType={schema.properties[argName].items}
						/>
						{#if types[index] === 'static' && args[argName].type === InputTransform.type.JAVASCRIPT}
							<span
								class="bg-blue-100 text-blue-800 text-sm font-medium mr-2 px-2.5 py-0.5 rounded ml-2"
							>
								{'${...}'}
							</span>
						{/if}
					</div>
					<Toggle
						options={{
							left: { label: '', value: InputTransform.type.STATIC },
							right: { label: 'Code editor', value: InputTransform.type.JAVASCRIPT }
						}}
						bind:value={types[index]}
						on:change={(e) => {
							if (e.detail === InputTransform.type.JAVASCRIPT) {
								args[argName].expr = getDefaultExpr(i)
								args[argName].value = undefined
							} else {
								args[argName].expr = undefined
								args[argName].value = undefined
							}

							args[argName].type = e.detail
						}}
					/>
				</div>
				<div class="max-w-xs" />

				{#if types[index] === 'static'}
					<OverlayPropertyPicker
						{previousSchema}
						{index}
						on:select={(event) => {
							args[argName].value = `\$\{previous_result.${event.detail}}`
						}}
					>
						<ArgInput
							label={argName}
							bind:description={schema.properties[argName].description}
							bind:value={args[argName].value}
							type={schema.properties[argName].type}
							required={schema.required.includes(argName)}
							bind:pattern={schema.properties[argName].pattern}
							bind:valid={inputCheck[argName]}
							defaultValue={schema.properties[argName].default}
							bind:enum_={schema.properties[argName].enum}
							bind:format={schema.properties[argName].format}
							contentEncoding={schema.properties[argName].contentEncoding}
							bind:itemsType={schema.properties[argName].items}
							displayHeader={false}
						/>
					</OverlayPropertyPicker>
				{:else if types[index] === InputTransform.type.JAVASCRIPT}
					{#if args[argName].expr != undefined}
						<div class="border rounded p-2 mt-2 border-gray-300">
							<Editor
								bind:code={args[argName].expr}
								lang="typescript"
								class="few-lines-editor"
								{extraLib}
								extraLibPath="file:///node_modules/@types/windmill@{i}/index.d.ts"
							/>
						</div>

						<DynamicInputHelpBox />
					{/if}
				{:else}
					<p>Not recognized arg type {args[argName].type}</p>
				{/if}
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
