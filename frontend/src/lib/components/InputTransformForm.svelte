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
	import { getDefaultExpr, isCodeInjection } from './flows/utils'
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
	let propertyType = getPropertyType(arg)

	function getPropertyType(arg: InputTransform | any): 'static' | 'javascript' {
		let type: 'static' | 'javascript' = arg.type
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
			arg.expr = getDefaultExpr(i!, argName, `\`${rawValue}\``)
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

	function focusProp(argName: string, monacoEditor: boolean = false) {
		Object.keys(overlays).forEach((k) => {
			if (k == argName && (isStaticTemplate(inputCats[argName]) || monacoEditor)) {
				overlays[k].focus()
			} else {
				overlays[k].unfocus()
			}
		})
	}

	function connectProperty(argName: string, rawValue: string) {
		if (isStaticTemplate(inputCats[argName])) {
			arg.value = `\$\{${rawValue}}`
			setPropertyType(arg.value)
		} else {
			arg.expr = getDefaultExpr(i ?? -1, undefined, rawValue)
			arg.type = 'javascript'
			propertyType = 'javascript'
		}

		if (monacos[argName]) {
			monacos[argName].setCode(arg.value)
		}
	}

	$: checked = propertyType == 'javascript'
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
			on:change={(e) => {
				const type = e.detail ? 'javascript' : 'static'
				const staticTemplate = isStaticTemplate(inputCats[argName])
				if (type === 'javascript') {
					arg.expr = getDefaultExpr(
						i ?? -1,
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
	</div>
	<div class="max-w-xs" />

	{#if propertyType === undefined || !checked}
		<OverlayPropertyPicker
			{pickableProperties}
			bind:this={overlays[argName]}
			on:select={({ detail }) => {
				if (detail.pickerVariation === 'connect') {
					connectProperty(argName, detail.propPath)
				} else {
					const toAppend = `\$\{${detail.propPath}}`
					arg.value = `${arg.value ?? ''}${toAppend}`
					if (monacos[argName]) {
						monacos[argName].setCode(arg.value)
					}
					if (isStaticTemplate(inputCats[argName])) {
						setPropertyType(arg.value)
					}
				}
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
				on:input={(e) => {
					if (isStaticTemplate(inputCats[argName])) {
						setPropertyType(e.detail.rawValue)
					}
				}}
			>
				<div slot="actions">
					<div on:click={() => overlays[argName]?.focus('connect')}>
						<Tooltip placement="bottom" content="Input connect">
							<Button size="sm" class="blue-button h-8">
								<Icon data={faChain} />
							</Button>
						</Tooltip>
					</div>
				</div>
			</ArgInput>
		</OverlayPropertyPicker>
	{:else if checked}
		{#if arg.expr != undefined}
			<OverlayPropertyPicker
				bind:this={overlays[argName]}
				{pickableProperties}
				on:select={(event) => {
					monacos[argName].insertAtCursor(event.detail.propPath)
				}}
			>
				<div class="border rounded p-2 mt-2 border-gray-300">
					<Editor
						bind:this={monacos[argName]}
						on:focus={() => focusProp(argName, true)}
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
