<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { InputTransform } from '$lib/gen'
	import type { InputCat } from '$lib/utils'
	import { getContext } from 'svelte'

	import ArgInput from './ArgInput.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import DynamicInputHelpBox from './flows/content/DynamicInputHelpBox.svelte'
	import type { PropPickerWrapperContext } from './flows/propPicker/PropPickerWrapper.svelte'
	import { codeToStaticTemplate, getDefaultExpr, isCodeInjection } from './flows/utils'
	import SimpleEditor from './SimpleEditor.svelte'
	import Toggle from './Toggle.svelte'
	import { Button } from './common'
	import Icon from 'svelte-awesome'
	import { faChain } from '@fortawesome/free-solid-svg-icons'

	export let schema: Schema
	export let arg: InputTransform | any
	export let argName: string
	export let extraLib: string = 'missing extraLib'
	export let inputCheck: { [id: string]: boolean }
	export let importPath: string | undefined = undefined

	let monacos: { [id: string]: SimpleEditor } = {}

	let inputCats: { [id: string]: InputCat } = {}
	let propertyType = getPropertyType(arg)

	function getPropertyType(arg: InputTransform | any): 'static' | 'javascript' {
		let type: 'static' | 'javascript' = arg?.type ?? 'static'
		if (type == 'javascript') {
			if (codeToStaticTemplate(arg.expr)) {
				type = 'static'
			}
		}
		return type
	}

	function setPropertyType(rawValue: string) {
		if (!arg) {
			return
		}

		if (isCodeInjection(rawValue)) {
			arg.expr = getDefaultExpr(importPath, argName, `\`${rawValue}\``)
			arg.type = 'javascript'
			propertyType = 'static'
		} else {
			if (arg.type === 'javascript' && propertyType === 'static') {
				arg.type = 'static'
			}
			if (arg.type) {
				propertyType = arg.type
			}
		}
	}

	function isStaticTemplate(inputCat: InputCat) {
		return inputCat === 'string' || inputCat === 'sql'
	}

	function connectProperty(argName: string, rawValue: string) {
		if (isStaticTemplate(inputCats[argName])) {
			arg.value = `\$\{${rawValue}}`
			setPropertyType(arg.value)
		} else {
			arg.expr = getDefaultExpr(importPath, undefined, rawValue)
			arg.type = 'javascript'
			propertyType = 'javascript'
		}
	}

	$: checked = propertyType == 'javascript'

	const { focusProp } = getContext<PropPickerWrapperContext>('PropPickerWrapper')
</script>

{#if arg != undefined}
	<div class="flex justify-between items-center mb-2">
		<div class="flex items-center">
			<FieldHeader
				label={argName}
				format={schema.properties[argName].format}
				contentEncoding={schema.properties[argName].contentEncoding}
				required={schema.required.includes(argName)}
				type={schema.properties[argName].type}
				itemsType={schema.properties[argName].items}
			/>

			{#if !checked && arg.type === 'javascript'}
				<span class="bg-blue-100 text-blue-800 text-sm font-medium mr-2 px-2.5 py-0.5 rounded ml-2">
					{'${...}'}
				</span>
			{/if}
		</div>
		<div class="flex flex-row space-x-2 items-center">
			<Toggle
				bind:checked
				options={{
					right: 'Raw Javascript Editor'
				}}
				on:change={(e) => {
					const type = e.detail ? 'javascript' : 'static'
					const staticTemplate = isStaticTemplate(inputCats[argName])
					if (type === 'javascript') {
						arg.expr = getDefaultExpr(
							importPath,
							argName,
							staticTemplate ? `\`${arg.value ?? ''}\`` : arg.value
						)

						arg.value = undefined
						propertyType = 'javascript'
					} else {
						arg.value = staticTemplate ? codeToStaticTemplate(arg.expr) : undefined

						arg.expr = undefined
						propertyType = 'static'
					}

					arg.type = type
				}}
			/>
			<div
				on:click={() => {
					focusProp(argName, 'connect', (path) => {
						connectProperty(argName, path)
					})
				}}
			>
				<Button variant="contained" color="blue" size="md">
					<Icon data={faChain} />
				</Button>
			</div>
		</div>
	</div>
	<div class="max-w-xs" />

	{#if propertyType === undefined || !checked}
		<ArgInput
			on:focus={() => {
				if (isStaticTemplate(inputCats[argName])) {
					focusProp(argName, 'append', (path) => {
						const toAppend = `\$\{${path}}`
						arg.value = `${arg.value ?? ''}${toAppend}`
						setPropertyType(arg.value)
					})
				} else {
					focusProp(argName, 'insert', (path) => {
						console.log('path', path)
						arg.expr = path
						arg.type = 'javascript'
						propertyType = 'javascript'
					})
				}
			}}
			label={argName}
			bind:editor={monacos[argName]}
			bind:description={schema.properties[argName].description}
			bind:value={arg.value}
			type={schema.properties[argName].type}
			required={schema.required.includes(argName)}
			bind:pattern={schema.properties[argName].pattern}
			bind:valid={inputCheck[argName]}
			defaultValue={schema.properties[argName].default}
			bind:enum_={schema.properties[argName].enum}
			bind:format={schema.properties[argName].format}
			contentEncoding={schema.properties[argName].contentEncoding}
			bind:itemsType={schema.properties[argName].items}
			properties={schema.properties[argName].properties}
			displayHeader={false}
			bind:inputCat={inputCats[argName]}
			on:input={(e) => {
				if (isStaticTemplate(inputCats[argName])) {
					setPropertyType(e.detail.rawValue)
				}
			}}
		/>
	{:else if checked}
		{#if arg.expr != undefined}
			<div class="border rounded p-2 mt-2 border-gray-300">
				<SimpleEditor
					bind:this={monacos[argName]}
					on:focus={() => {
						focusProp(argName, 'insert', (path) => {
							monacos[argName].insertAtCursor(path)
						})
					}}
					bind:code={arg.expr}
					lang="javascript"
					class="few-lines-editor"
					{extraLib}
					extraLibPath="file:///node_modules/@types/windmill@{importPath}/index.d.ts"
					shouldBindKey={false}
				/>
			</div>
			<DynamicInputHelpBox {importPath} />
			<div class="mb-2" />
		{/if}
	{:else}
		<p>Not recognized arg type {arg.type}</p>
	{/if}
{:else}
	<p>Arg at {argName} is undefined</p>
{/if}
