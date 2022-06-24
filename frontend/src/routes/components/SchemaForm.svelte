<script lang="ts">
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { slide } from 'svelte/transition'
	import type { Schema } from '../../common'
	import { allTrue } from '../../utils'
	import ArgInput from './ArgInput.svelte'
	import Editor from './Editor.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import PropPicker from './flows/PropPicker.svelte'
	import RadioButton from './RadioButton.svelte'

	export let inputTransform = false
	export let schema: Schema
	export let args: Record<string, any> = {}

	export let isValid: boolean = true
	export let editableSchema = false

	export let extraLib: string
	export let i: number
	export let previousSchema: Object

	let inputCheck: { [id: string]: boolean } = {}
	let seeHelp: { [id: string]: boolean } = {}
	let editor: Editor

	export function setArgs(nargs: Record<string, any>) {
		args = nargs
	}

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
									on:change={(event) => {
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

						<div class="text-xs flex flex-row-reverse">
							<span
								class="underline mr-4"
								on:click={() => {
									seeHelp[argName] = seeHelp[argName] == undefined ? true : !seeHelp[argName]
								}}
								>Help<Icon
									class="ml-2"
									data={seeHelp[argName] ? faChevronUp : faChevronDown}
									scale={0.7}
								/></span
							>
						</div>
						{#if seeHelp[argName]}
							<div
								transition:slide
								class="bg-gray-100 border-l-4 border-gray-600 text-gray-700 p-4 m-4"
								role="alert"
							>
								<p class="font-bold">Dynamic arg help</p>
								<p>
									When a field is "dynamic", its value is computed dynamically as the evaluation of
									its corresponding typescript snippet.
								</p>
								That snippet can be single line:
								<pre><code>last_result.myarg</code></pre>
								or multiline:
								<pre><code
										>let x = 5;
x + 2</code
									></pre>
								<p>
									If it is multiline, the last statement before the final expression<b
										>MUST END WITH ; and a newline</b
									>
								</p>
								The snippet can also be a string template:
								<pre><code
										>`Hello $&#123;params.name&#125;, all your base $&#123;previous_result.base_name&#125;
belong to us`</code
									></pre>
								However, the last line must always be the final expression.
								<p>
									The snippet can use any typescript primitives, and the following flow specific
									objects and functions:
								</p>
								<ul class="ml-4">
									<li>
										<b>previous_result</b>: the object containing the result of the previous step
									</li>
									<li><b>flow_input</b>: the object containing the flow input arguments</li>
									<li><b>params</b>: the object containing the current step static values</li>
									<li>
										<b>step(n)</b>: the function returning the result of step number n. One can also
										use a negative n. step(0) == flow_input , step(-1) == previous_result
									</li>
									<li>
										<b>variable(path)</b>: the function returning the variable (including secrets)
										at given path as a string
									</li>
									<li>
										<b>resource(path)</b>: the function returning the resource at a given path as an
										object
									</li>
								</ul>
							</div>
						{/if}
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
				/>{/if}
		{/each}
	{:else}
		<p class="italic text-sm">No settable input</p>
	{/if}
</div>
