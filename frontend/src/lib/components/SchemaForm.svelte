<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { InputTransform } from '$lib/gen'
	import { allTrue, type InputCat } from '$lib/utils'
	import ArgInput from './ArgInput.svelte'
	import Editor from './Editor.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import DynamicInputHelpBox from './flows/DynamicInputHelpBox.svelte'
	import { codeToStaticTemplate } from './flows/flowStore'
	import { getCodeInjectionExpr, getDefaultExpr, isCodeInjection } from './flows/utils'
	import OverlayPropertyPicker from './propertyPicker/OverlayPropertyPicker.svelte'
	import Toggle from './Toggle.svelte'

	export let inputTransform = false
	export let schema: Schema
	export let args: Record<string, InputTransform | any> = {}
	export let editableSchema = false
	export let extraLib: string = 'missing extraLib'
	export let isValid: boolean = true

	export let i: number | undefined = undefined
	export let pickableProperties: Object | undefined = undefined

	let inputCheck: { [id: string]: boolean } = {}
	let overlays: { [id: string]: OverlayPropertyPicker } = {}
	let monacos: { [id: string]: Editor } = {}

	$: isValid = allTrue(inputCheck) ?? false

	let propertiesTypes: { [id: string]: 'javascript' | 'static' } = inputTransform
		? Object.keys(args).reduce((acc, key) => {
				let type = args[key].type
				if (type == 'javascript') {
					if (codeToStaticTemplate(args[key].expr)) {
						type = 'static'
					}
				}
				acc[key] = type
				return acc
		  }, {})
		: {}

	let inputCats: { [id: string]: InputCat } = {}

	function setPropertyType(id: string, rawValue: string, isRaw: boolean) {
		const arg = args[id]
		if (!arg) {
			return
		}

		if (isCodeInjection(rawValue)) {
			args[id].expr = getCodeInjectionExpr(rawValue, isRaw)
			args[id].type = 'javascript'
			propertiesTypes[id] = 'static'
		} else {
			if (args[id].type === 'javascript' && propertiesTypes[id] === 'static') {
				args[id].type = 'static'
				if (inputCats[id] == 'number') {
					args[id].value = Number(args[id].value)
				}
			}
			if (arg.type) {
				propertiesTypes[id] = arg.type
			}
		}
	}

	function isStaticTemplate(inputCat: InputCat) {
		return inputCat === 'string' || inputCat === 'number'
	}

	function focusProp(argName: string) {
		Object.keys(overlays).forEach((k) => {
			if (k == argName) {
				overlays[k].focus()
			} else {
				overlays[k].unfocus()
			}
		})
	}
</script>

<div class="w-full">
	{#if Object.keys(schema?.properties ?? {}).length > 0}
		{#each Object.keys(schema?.properties ?? {}) as argName, index}
			{#if inputTransform}
				{#if args[argName] != undefined}
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
							{#if propertiesTypes[argName] === 'static' && args[argName].type === 'javascript'}
								<span
									class="bg-blue-100 text-blue-800 text-sm font-medium mr-2 px-2.5 py-0.5 rounded ml-2"
								>
									{'${...}'}
								</span>
							{/if}
						</div>
						<Toggle
							options={{
								left: { label: '', value: 'static' },
								right: { label: 'Raw Javascript Editor', value: 'javascript' }
							}}
							bind:value={propertiesTypes[argName]}
							on:change={(e) => {
								const staticTemplate = isStaticTemplate(inputCats[argName])
								if (e.detail === 'javascript') {
									args[argName].expr = getDefaultExpr(
										i ?? -1,
										argName,
										staticTemplate ? args[argName].value : undefined
									)
									args[argName].value = undefined
								} else {
									args[argName].value = staticTemplate
										? codeToStaticTemplate(args[argName].expr)
										: undefined
									args[argName].expr = undefined
								}

								args[argName].type = e.detail
							}}
						/>
					</div>
					<div class="max-w-xs" />

					{#if propertiesTypes[argName] === undefined || propertiesTypes[argName] === 'static'}
						<OverlayPropertyPicker
							bind:this={overlays[argName]}
							{pickableProperties}
							disabled={!isStaticTemplate(inputCats[argName])}
							on:select={(event) => {
								const toAppend = `\$\{${event.detail}}`
								args[argName].value = `${args[argName].value ?? ''}${toAppend}`
								setPropertyType(argName, args[argName].value, false)
							}}
						>
							<ArgInput
								on:focus={() => focusProp(argName)}
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
								bind:inputCat={inputCats[argName]}
								numberAsString={true}
								on:input={(e) => {
									if (isStaticTemplate(inputCats[argName])) {
										setPropertyType(argName, e.detail.rawValue, e.detail.isRaw)
									}
								}}
							/>
						</OverlayPropertyPicker>
					{:else if propertiesTypes[argName] === 'javascript'}
						{#if args[argName].expr != undefined}
							<OverlayPropertyPicker
								bind:this={overlays[argName]}
								{pickableProperties}
								on:select={(event) => {
									monacos[argName].insertAtCursor(event.detail)
								}}
							>
								<div class="border rounded p-2 mt-2 border-gray-300">
									<Editor
										bind:this={monacos[argName]}
										on:focus={() => focusProp(argName)}
										bind:code={args[argName].expr}
										lang="javascript"
										class="few-lines-editor"
										{extraLib}
										extraLibPath="file:///node_modules/@types/windmill@{i}/index.d.ts"
									/>
								</div>
							</OverlayPropertyPicker>
							<DynamicInputHelpBox />
						{/if}
					{:else}
						<p>Not recognized arg type {args[argName].type}</p>
					{/if}
				{:else}
					<p>Arg at {argName} is undefined</p>
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
