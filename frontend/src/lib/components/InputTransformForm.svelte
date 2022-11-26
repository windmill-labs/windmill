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
	import { Button, ToggleButton, ToggleButtonGroup } from './common'
	import { faCode } from '@fortawesome/free-solid-svg-icons'

	export let schema: Schema
	export let arg: InputTransform | any
	export let argName: string
	export let extraLib: string = 'missing extraLib'
	export let inputCheck: boolean = true
	export let previousModuleId: string | undefined

	export let monaco: SimpleEditor | undefined = undefined
	let argInput: ArgInput | undefined = undefined

	let inputCat: InputCat = 'object'
	let propertyType = getPropertyType(arg)

	function getPropertyType(arg: InputTransform | any): 'static' | 'javascript' {
		let type: 'static' | 'javascript' = arg?.type ?? 'static'
		if (type == 'javascript') {
			const newValue = codeToStaticTemplate(arg.expr)
			if (newValue) {
				type = 'static'
				arg.value = newValue
			}
		}
		return type
	}

	function setPropertyType(rawValue: string) {
		if (!arg) {
			return
		}

		if (isCodeInjection(rawValue)) {
			arg.expr = getDefaultExpr(argName, previousModuleId, `\`${rawValue}\``)
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

	function connectProperty(rawValue: string) {
		if (isStaticTemplate(inputCat)) {
			arg.value = `\$\{${rawValue}}`
			setPropertyType(arg.value)
		} else {
			arg.expr = getDefaultExpr(undefined, previousModuleId, rawValue)
			arg.type = 'javascript'
			propertyType = 'javascript'
			monaco?.setCode(arg.expr)
		}
	}

	function onFocus() {
		if (isStaticTemplate(inputCat)) {
			focusProp(argName, 'append', (path) => {
				const toAppend = `\$\{${path}}`
				arg.value = `${arg.value ?? ''}${toAppend}`
				setPropertyType(arg.value)
				argInput?.focus()
				return false
			})
		} else {
			focusProp(argName, 'insert', (path) => {
				arg.expr = path
				arg.type = 'javascript'
				propertyType = 'javascript'
				return true
			})
		}
	}
	const { focusProp } = getContext<PropPickerWrapperContext>('PropPickerWrapper')
</script>

{#if arg != undefined}
	<div class="flex flex-row justify-between gap-1 mb-1">
		<div class="flex items-center flex-wrap grow">
			<FieldHeader
				label={argName}
				format={schema.properties[argName].format}
				contentEncoding={schema.properties[argName].contentEncoding}
				required={schema.required.includes(argName)}
				type={schema.properties[argName].type}
				itemsType={schema.properties[argName].items}
			/>

			{#if isStaticTemplate(inputCat)}
				<span
					class="bg-blue-100 text-blue-800 text-sm font-medium mr-2 px-2.5 py-0.5 rounded ml-2 {propertyType ==
						'static' && arg.type === 'javascript'
						? 'visible'
						: 'invisible'}"
				>
					{'${...}'}
				</span>
			{/if}
		</div>
		<div class="flex flex-row gap-1 flex-wrap">
			<div>
				<ToggleButtonGroup
					bind:selected={propertyType}
					on:selected={(e) => {
						const staticTemplate = isStaticTemplate(inputCat)
						if (e.detail === 'javascript') {
							arg.expr = getDefaultExpr(
								argName,
								previousModuleId,
								staticTemplate
									? `\`${arg.value ?? ''}\``
									: arg.value
									? JSON.stringify(arg.value, null, 4)
									: ''
							)

							arg.value = undefined
							propertyType = 'javascript'
							arg.type = 'javascript'
						} else {
							if (staticTemplate) {
								arg.value = codeToStaticTemplate(arg.expr)
								setPropertyType(arg.value)
							} else {
								arg.type = 'static'
								arg.value = undefined
								arg.expr = undefined
							}
							propertyType = 'static'
						}
					}}
				>
					{#if isStaticTemplate(inputCat)}
						<ToggleButton light position="left" value="static" size="xs">Template</ToggleButton>
					{:else}
						<ToggleButton light position="left" value="static" size="xs">Static</ToggleButton>
					{/if}

					<ToggleButton
						light
						position="right"
						value="javascript"
						startIcon={{ icon: faCode }}
						size="xs"
					>
						Dynamic (JS)
					</ToggleButton>
				</ToggleButtonGroup>
			</div>
			<div>
				<Button
					variant="contained"
					color="blue"
					size="xs"
					on:click={() => {
						focusProp(argName, 'connect', (path) => {
							connectProperty(path)
							return false
						})
					}}>Connect &rightarrow;</Button
				>
			</div>
		</div>
	</div>
	<div class="max-w-xs" />

	{#if propertyType === undefined || propertyType == 'static'}
		<ArgInput
			bind:this={argInput}
			on:focus={onFocus}
			label={argName}
			bind:editor={monaco}
			bind:description={schema.properties[argName].description}
			bind:value={arg.value}
			type={schema.properties[argName].type}
			required={schema.required.includes(argName)}
			bind:pattern={schema.properties[argName].pattern}
			bind:valid={inputCheck}
			defaultValue={schema.properties[argName].default}
			bind:enum_={schema.properties[argName].enum}
			bind:format={schema.properties[argName].format}
			contentEncoding={schema.properties[argName].contentEncoding}
			bind:itemsType={schema.properties[argName].items}
			properties={schema.properties[argName].properties}
			displayHeader={false}
			bind:inputCat
			on:input={(e) => {
				if (isStaticTemplate(inputCat)) {
					setPropertyType(e.detail.rawValue)
				}
			}}
		/>
	{:else if arg.expr != undefined}
		<div class="border rounded p-2 mt-2 border-gray-300">
			<SimpleEditor
				bind:this={monaco}
				bind:code={arg.expr}
				{extraLib}
				lang="javascript"
				shouldBindKey={false}
				on:focus={() => {
					focusProp(argName, 'insert', (path) => {
						monaco?.insertAtCursor(path)
						return false
					})
				}}
				autoHeight
			/>
		</div>
		<DynamicInputHelpBox />
		<div class="mb-2" />
	{:else}
		Not recognized input type {argName}
	{/if}
{:else}
	<p class="text-sm text-gray-700">Arg at {argName} is undefined</p>
{/if}
