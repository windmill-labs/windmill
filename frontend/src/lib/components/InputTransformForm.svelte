<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { InputTransform } from '$lib/gen'
	import type { InputCat } from '$lib/utils'
	import { faChain } from '@fortawesome/free-solid-svg-icons'
	import { Button, Tooltip } from 'flowbite-svelte'
	import Icon from 'svelte-awesome'
	import ArgInput from './ArgInput.svelte'
	import Editor from './Editor.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import DynamicInputHelpBox from './flows/DynamicInputHelpBox.svelte'
	import { codeToStaticTemplate } from './flows/flowStore'
	import { getCodeInjectionExpr, isCodeInjection } from './flows/utils'
	import ClickablePropertyPicker from './propertyPicker/ClickablePropertyPicker.svelte'
	import OverlayPropertyPicker from './propertyPicker/OverlayPropertyPicker.svelte'
	import Toggle from './Toggle.svelte'

	export let schema: Schema
	export let arg: InputTransform | any
	export let argName: string
	export let extraLib: string = 'missing extraLib'
	export let inputCheck: { [id: string]: boolean }

	export let i: number | undefined = undefined
	export let pickableProperties: Object | undefined = undefined

	let overlays: { [id: string]: OverlayPropertyPicker } = {}
	let monacos: { [id: string]: Editor } = {}

	let inputCats: { [id: string]: InputCat } = {}

	function getPropertyType(arg: InputTransform | any): 'static' | 'javascript' {
		let type: 'static' | 'javascript' = arg.type
		if (type == 'javascript') {
			if (codeToStaticTemplate(arg.expr)) {
				type = 'static'
			}
		}
		return type
	}

	$: propertyType = getPropertyType(arg)

	function setPropertyType(id: string, rawValue: string, isRaw: boolean) {
		if (!arg) {
			return
		}

		if (isCodeInjection(rawValue)) {
			arg.expr = getCodeInjectionExpr(rawValue, isRaw)
			arg.type = 'javascript'
			propertyType = 'static'
		} else {
			if (arg.type === 'javascript' && propertyType === 'static') {
				arg.type = 'static'
				if (inputCats[id] == 'number') {
					arg.value = Number(arg.value)
				}
			}
			if (arg.type) {
				propertyType = arg.type
			}
		}
	}

	function isStaticTemplate(inputCat: InputCat) {
		return inputCat === 'string' || inputCat === 'number' || inputCat === 'sql'
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

	function onPropertyLink(argName: string, rawValue: string) {
		if (isStaticTemplate(inputCats[argName])) {
			arg.value = `\$\{${rawValue}}`
			setPropertyType(argName, arg.value, false)
		} else {
			arg.expr = getCodeInjectionExpr(`\$\{${rawValue}}`, false)
			arg.type = 'javascript'
			checked = true
		}

		if (monacos[argName]) {
			monacos[argName].setCode(arg.value)
		}
	}

	$: checked = propertyType === 'javascript'
</script>

{#if arg != undefined}
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

			{#if !checked && arg.type === 'javascript'}
				<span class="bg-blue-100 text-blue-800 text-sm font-medium mr-2 px-2.5 py-0.5 rounded ml-2">
					{'${...}'}
				</span>
			{/if}
		</div>
		<Toggle
			bind:checked
			options={{
				right: 'Raw Javascript Editor'
			}}
		/>
	</div>
	<div class="max-w-xs" />

	{#if propertyType === undefined || !checked}
		<OverlayPropertyPicker
			bind:this={overlays[argName]}
			{pickableProperties}
			disabled={!isStaticTemplate(inputCats[argName])}
			on:select={(event) => {
				const toAppend = `\$\{${event.detail}}`
				arg.value = `${arg.value ?? ''}${toAppend}`
				if (monacos[argName]) {
					monacos[argName].setCode(arg.value)
				}
				setPropertyType(argName, arg.value, false)
			}}
		>
			<ArgInput
				on:focus={() => focusProp(argName)}
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
				displayHeader={false}
				bind:inputCat={inputCats[argName]}
				numberAsString={true}
				on:input={(e) => {
					if (isStaticTemplate(inputCats[argName])) {
						setPropertyType(argName, e.detail.rawValue, e.detail.isRaw)
					}
				}}
			>
				<div slot="actions">
					<ClickablePropertyPicker
						bind:pickableProperties
						on:select={(event) => onPropertyLink(argName, event.detail)}
					>
						<Tooltip placement="bottom" content="Input connect">
							<Button color="blue" size="sm" class="h-8">
								<Icon data={faChain} />
							</Button>
						</Tooltip>
					</ClickablePropertyPicker>
				</div>
			</ArgInput>
		</OverlayPropertyPicker>
	{:else if checked}
		{#if arg.expr != undefined}
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
						bind:code={arg.expr}
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
		<p>Not recognized arg type {arg.type}</p>
	{/if}
{:else}
	<p>Arg at {argName} is undefined</p>
{/if}
