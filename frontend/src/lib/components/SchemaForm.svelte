<script lang="ts">
	import type { Schema } from '$lib/common'
	import { InputTransform } from '$lib/gen'
	import { allTrue } from '$lib/utils'
	import ArgInput from './ArgInput.svelte'
	import Editor from './Editor.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import DynamicInputHelpBox from './flows/DynamicInputHelpBox.svelte'
	import Toggle from './Toggle.svelte'

	export let inputTransform = false
	export let schema: Schema
	export let args: Record<string, InputTransform> = {}
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

	function wasPicked(expr: string | undefined): boolean {
		if (!expr) {
			return false
		}
		const lines = expr.split('\n')
		const [returnStatement] = lines.reverse()

		const returnStatementRegex = new RegExp(/\$\{(.*)\}/)
		if (returnStatementRegex.test(returnStatement)) {
			const [_, argName] = returnStatement.split(returnStatementRegex)

			return Boolean(argName)
		}
		return false
	}

	$: types = Object.keys(schema?.properties ?? {}).map((prop, index) => {
		const arg = args[prop]

		const displayed_expr = wasPicked(arg.value)

		if (!displayed_expr) {
			return arg.type
		}

		args[prop].expr = `\`${arg.value}\``
		args[prop].type = InputTransform.type.JAVASCRIPT
		return InputTransform.type.STATIC
	})

	$: console.log(Object.keys(schema?.properties ?? {}).map((p) => args[p]))
</script>

<div class="w-full">
	{#if Object.keys(schema?.properties ?? {}).length > 0}
		{#each Object.keys(schema?.properties ?? {}) as argName, index}
			{#if inputTransform && args[argName] != undefined}
				<div class={index > 0 ? 'mt-8' : ''} />
				<FieldHeader
					label={argName}
					format={schema.properties[argName].format}
					contentEncoding={schema.properties[argName].contentEncoding}
					required={schema.required.includes(argName)}
					type={schema.properties[argName].type}
					itemsType={schema.properties[argName].items}
				/>
				<span>
					<Toggle
						options={{
							left: { value: InputTransform.type.STATIC },
							right: { label: 'Code editor', value: InputTransform.type.JAVASCRIPT }
						}}
						bind:value={types[index]}
						on:change={(e) => {
							if (e.detail === InputTransform.type.JAVASCRIPT) {
								args[argName].expr = getDefaultExpr(i)
								args[argName].value = undefined
							} else {
								args[argName].expr = undefined
							}

							args[argName].type = e.detail
						}}
					/></span
				>
				<div class="max-w-xs" />
				{#if types[index] === 'static'}
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
				{:else if types[index] === InputTransform.type.JAVASCRIPT}
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
