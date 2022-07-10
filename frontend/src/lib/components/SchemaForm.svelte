<script lang="ts">
	import type { Schema } from '$lib/common'
	import { allTrue } from '$lib/utils'
	import ArgInput from './ArgInput.svelte'
	import Editor from './Editor.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import DynamicInputHelpBox from './flows/DynamicInputHelpBox.svelte'
	import PropPicker from './flows/PropPicker.svelte'
	import RadioButton from './RadioButton.svelte'

	export let inputTransform = false
	export let schema: Schema
	export let args: Record<string, any> = {}
	export let editableSchema = false
	export let extraLib: string
	export let isValid: boolean = true

	export let i: number
	export let previousSchema: Object

	let inputCheck: { [id: string]: boolean } = {}
	let editor: Editor

	function getDefaultExpr(i: number, key: string = 'myfield') {
		return `import { previous_result, flow_input, step, variable, resource, params } from 'windmill@${i}'

previous_result.${key}`
	}

	$: isValid = allTrue(inputCheck) ?? false
</script>

<div class="w-full">
	{#if Object.keys(schema?.properties ?? {}).length > 0}
		{#each Object.keys(schema?.properties ?? {}) as argName}
			{#if inputTransform && args[argName] != undefined}
				<div class="mt-10" />
				<FieldHeader
					label={argName}
					format={schema.properties[argName].format}
					contentEncoding={schema.properties[argName].contentEncoding}
					required={schema.required.includes(argName)}
					type={schema.properties[argName].type}
					itemsType={schema.properties[argName].items}
				/>
				<div class="max-w-xs">
					<RadioButton
						options={[
							['Static', 'static'],
							['Dynamic (JS)', 'javascript']
						]}
						small={true}
						bind:value={args[argName].type}
						on:change={(e) => {
							args[argName].expr = e.detail == 'javascript' ? getDefaultExpr(i) : undefined
						}}
					/>
				</div>
				{#if args[argName].type == 'static'}
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
				{:else if args[argName].type == 'javascript'}
					{#if args[argName].expr != undefined}
						<div class="border rounded p-2 mt-2 border-gray-300">
							<Editor
								bind:this={editor}
								bind:code={args[argName].expr}
								lang="typescript"
								class="few-lines-editor"
								{extraLib}
								extraLibPath="file:///node_modules/@types/windmill@{i}/index.d.ts"
							/>
						</div>
						<div class="mt-4">
							{#if Boolean(previousSchema)}
								<PropPicker
									props={previousSchema}
									on:select={(event) => {
										editor.setCode(getDefaultExpr(i, event.detail))
									}}
								/>
							{:else}
								<div
									class="flex p-4 mb-4 bg-yellow-100 border-t-4 border-yellow-500 dark:bg-yellow-200"
									role="alert"
								>
									<div class="ml-3 text-sm font-medium text-yellow-700">
										Previous results are not avaiable. The property picker and type inference are
										not avaiable.
									</div>
								</div>
							{/if}
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
