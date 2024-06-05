<script lang="ts">
	import type { SchemaProperty } from '$lib/common'
	import { setInputCat as computeInputCat, emptyString } from '$lib/utils'
	import { GripVertical, Plus, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Multiselect from 'svelte-multiselect'
	import { fade } from 'svelte/transition'
	import JsonEditor from '../apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'
	import { Button } from '../common'
	import FieldHeader from '../FieldHeader.svelte'
	import type ItemPicker from '../ItemPicker.svelte'
	import ObjectResourceInput from '../ObjectResourceInput.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import Toggle from '../Toggle.svelte'
	import type VariableEditor from '../VariableEditor.svelte'
	import { twMerge } from 'tailwind-merge'
	import ArgEnum from '../ArgEnum.svelte'
	import DateTimeInput from '../DateTimeInput.svelte'
	import DateInput from '../DateInput.svelte'
	import { evalValueToRaw, testRegex } from '../schema/utils'
	import NumberProperty from './display/NumberProperty.svelte'
	import S3FileProperty from './display/S3FileProperty.svelte'
	import StringProperty from './display/StringProperty.svelte'
	import SchemaForm from '../SchemaForm.svelte'

	export let label: string = ''
	export let value: any

	export let description: string = ''
	export let format: string = ''
	export let contentEncoding: 'base64' | 'binary' | undefined = undefined
	export let type: string | undefined = undefined
	export let required = false
	export let pattern: undefined | string = undefined
	export let valid = true
	export let enum_: string[] | undefined = undefined
	export let disabled = false
	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object'
				contentEncoding?: 'base64'
				enum?: string[]
				multiselect?: string[]
		  }
		| undefined = undefined
	export let displayHeader = true
	export let properties: { [name: string]: SchemaProperty } | undefined = undefined
	export let nestedRequired: string[] | undefined = undefined
	export let autofocus: boolean | null = null
	export let compact = false
	export let password = false
	export let pickForField: string | undefined = undefined
	export let variableEditor: VariableEditor | undefined = undefined
	export let itemPicker: ItemPicker | undefined = undefined
	export let noMargin = false
	export let extra: Record<string, any> = {}
	export let minW = true
	export let prettifyHeader = false
	export let resourceTypes: string[] | undefined
	export let disablePortal = false
	export let showSchemaExplorer = false
	export let simpleTooltip: string | undefined = undefined
	export let customErrorMessage: string | undefined = undefined
	export let onlyMaskPassword = false
	export let nullable: boolean = false
	export let title: string | undefined = undefined
	export let placeholder: string | undefined = undefined
	export let editor: SimpleEditor | undefined = undefined
	export let dndEnabled: boolean = false
	export let defaultValue: any = undefined

	const dispatch = createEventDispatcher()

	let ignoreValueUndefined = false
	let error: string = ''
	let isListJson = false
	let el: HTMLTextAreaElement | undefined = undefined
	let rawValue: string | undefined = undefined
	let inputCat = computeInputCat(type, format, itemsType?.type, enum_, contentEncoding)

	$: inputCat = computeInputCat(type, format, itemsType?.type, enum_, contentEncoding)

	function computeDefaultValue(
		nvalue?: any,
		inputCat?: string,
		defaultValue?: any,
		nnullable?: boolean
	) {
		if ((value == undefined || value == null) && !ignoreValueUndefined) {
			value = defaultValue
			if (defaultValue === undefined || defaultValue === null) {
				if (inputCat === 'string') {
					value = nullable ? null : ''
				} else if (inputCat == 'enum' && required) {
					value = enum_?.[0]
				} else if (inputCat == 'boolean') {
					value = false
				} else if (inputCat == 'list') {
					value = []
				}
			} else if (inputCat === 'object') {
				evalValueToRaw(inputCat, value, isListJson)
			}
		}

		if (nnullable && type === 'string' && value === '') {
			value = null
		}
	}

	computeDefaultValue()

	$: computeDefaultValue(value, inputCat, defaultValue, nullable)

	$: defaultValue != undefined && handleDefaultValueChange()

	let oldDefaultValue = defaultValue
	function handleDefaultValueChange() {
		if (value == oldDefaultValue) {
			value = defaultValue
		}
		oldDefaultValue = defaultValue
	}

	evalValueToRaw(inputCat, value, isListJson)

	function fileChanged(e: any, cb: (v: string | undefined) => void) {
		let t = e.target
		if (t && 'files' in t && t.files.length > 0) {
			let reader = new FileReader()
			reader.onload = (e: any) => {
				cb(e.target.result.split('base64,')[1])
			}
			reader.readAsDataURL(t.files[0])
		} else {
			cb(undefined)
		}
	}

	export function focus() {
		el?.focus()
		if (el) {
			el.style.height = '5px'
			el.style.height = el.scrollHeight + 50 + 'px'
		}
	}

	let redraw: number = 0
	let itemsLimit: number = 50

	// Field validation

	function validateInput(pattern: string | undefined, v: any, required: boolean): void {
		if (nullable && emptyString(v)) {
			error = ''
			valid && (valid = true)
		} else if (required && (v == undefined || v == null || v === '')) {
			error = 'Required'
			valid && (valid = false)
		} else {
			if (pattern && !testRegex(pattern, v)) {
				if (!emptyString(customErrorMessage)) {
					error = customErrorMessage ?? ''
				} else if (format == 'email') {
					error = 'invalid email address'
				} else {
					error = `Should match ${pattern}`
				}
				valid && (valid = false)
			} else {
				error = ''
				!valid && (valid = true)
			}
		}
	}

	$: validateInput(pattern, value, required)

	$: isS3Resource =
		inputCat == 'resource-object' &&
		format.split('-').length > 1 &&
		format.replace('resource-', '').replace('_', '').toLowerCase() == 's3object'
</script>

<div class="flex flex-row items-center justify-between w-full gap-2">
	<!-- svelte-ignore a11y-autofocus -->
	<div class={twMerge('flex flex-col w-full', 'gap-1', minW ? 'min-w-[250px]' : '')}>
		<div>
			{#if displayHeader}
				<FieldHeader
					prettify={prettifyHeader}
					label={title && !emptyString(title) ? title : label}
					{disabled}
					{required}
					{type}
					{contentEncoding}
					{format}
					{simpleTooltip}
				/>
			{/if}
			{#if description}
				<div class="text-xs italic text-secondary">
					<pre class="font-main whitespace-normal">{description}</pre>
				</div>
			{/if}
		</div>

		<div class="flex space-x-1">
			{#if inputCat == 'number'}
				<NumberProperty
					{autofocus}
					{disabled}
					{valid}
					{defaultValue}
					{placeholder}
					seconds={extra?.['seconds']}
					currency={extra?.['currency']}
					currencyLocale={extra?.['currencyLocale']}
					min={extra?.['min']}
					max={extra?.['max']}
					bind:value
					bind:ignoreValueUndefined
				/>
			{:else if inputCat == 'boolean'}
				<Toggle
					on:pointerdown={(e) => {
						e?.stopPropagation()
					}}
					{disabled}
					class={valid
						? ''
						: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'}
					bind:checked={value}
				/>
				{#if type == 'boolean' && value == undefined}
					<span>&nbsp; Not set</span>
				{/if}
			{:else if inputCat == 'list' && !isListJson}
				<div class="w-full flex gap-4">
					<div class="w-full">
						{#if Array.isArray(itemsType?.multiselect) && Array.isArray(value)}
							<div class="items-start">
								<Multiselect
									ulOptionsClass={'p-2 !bg-surface-secondary'}
									{disabled}
									bind:selected={value}
									options={itemsType?.multiselect ?? []}
									selectedOptionsDraggable={true}
								/>
							</div>
						{:else if itemsType?.enum != undefined && Array.isArray(itemsType?.enum) && Array.isArray(value)}
							<div class="items-start">
								<Multiselect
									ulOptionsClass={'p-2 !bg-surface-secondary'}
									{disabled}
									bind:selected={value}
									options={itemsType?.enum ?? []}
									selectedOptionsDraggable={true}
								/>
							</div>
						{:else}
							<div class="w-full">
								{#key redraw}
									{#if Array.isArray(value)}
										{#each value ?? [] as v, i}
											{#if i < itemsLimit}
												<div class="flex max-w-md mt-1 w-full items-center">
													{#if itemsType?.type == 'number'}
														<input type="number" bind:value={v} id="arg-input-number-array" />
													{:else if itemsType?.type == 'string' && itemsType?.contentEncoding == 'base64'}
														<input
															type="file"
															class="my-6"
															on:change={(x) => fileChanged(x, (val) => (value[i] = val))}
															multiple={false}
														/>
													{:else if itemsType?.type == 'object'}
														<JsonEditor code={JSON.stringify(v, null, 2)} bind:value={v} />
													{:else if Array.isArray(itemsType?.enum)}
														<ArgEnum
															required
															create={extra['disableCreate'] != true}
															on:focus={() => {
																dispatch('focus')
															}}
															on:blur={(e) => {
																dispatch('blur')
															}}
															{defaultValue}
															{valid}
															{disabled}
															{autofocus}
															bind:value={v}
															enum_={itemsType?.enum ?? []}
														/>
													{:else}
														<input type="text" bind:value={v} id="arg-input-array" />
													{/if}
													<button
														transition:fade|local={{ duration: 100 }}
														class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover ml-2"
														aria-label="Clear"
														on:click={() => {
															value.splice(i, 1)
															redraw += 1
														}}
													>
														<X size={14} />
													</button>
												</div>
											{/if}
										{/each}
										{#if value.length > itemsLimit}
											<button on:click={() => (itemsLimit += 50)} class="text-xs py-2 text-blue-600"
												>{itemsLimit}/{value.length}: Load 50 more...</button
											>
										{/if}
									{/if}
								{/key}
							</div>
							<div class="flex mt-2 gap-20 items-baseline">
								<Button
									variant="border"
									color="light"
									size="xs"
									btnClasses="mt-1"
									on:click={() => {
										if (value == undefined || !Array.isArray(value)) {
											value = []
										}
										value = value.concat('')
									}}
									id="arg-input-add-item"
									startIcon={{ icon: Plus }}
								>
									Add item
								</Button>
							</div>
						{/if}
					</div>
					<div class="mt-2 mr-4">
						<Toggle
							on:change={(e) => {
								evalValueToRaw(inputCat, value, isListJson)
								isListJson = !isListJson
							}}
							checked={isListJson}
							textClass="text-secondary"
							size="xs"
							options={{ right: 'json' }}
						/>
					</div>
				</div>
			{:else if inputCat == 'resource-object' && resourceTypes == undefined}
				<span class="text-2xs text-tertiary">Loading resource types...</span>
			{:else if inputCat == 'resource-object' && (resourceTypes == undefined || (format.split('-').length > 1 && resourceTypes.includes(format.substring('resource-'.length))))}
				<ObjectResourceInput selectFirst {disablePortal} {format} bind:value {showSchemaExplorer} />
			{:else if isS3Resource}
				<S3FileProperty bind:value {editor} on:focus on:blur {defaultValue} />
			{:else if inputCat == 'object' || inputCat == 'resource-object' || isListJson}
				{#if properties && Object.keys(properties).length > 0}
					<div class="p-4 pl-8 border rounded w-full">
						<SchemaForm
							{onlyMaskPassword}
							{disablePortal}
							{disabled}
							{dndEnabled}
							schema={{ properties, $schema: '', required: nestedRequired ?? [], type: 'object' }}
							bind:args={value}
						/>
					</div>
				{:else if disabled}
					<textarea disabled />
				{:else}
					<JsonEditor
						bind:editor
						on:focus={(e) => {
							dispatch('focus')
						}}
						on:blur={(e) => {
							dispatch('blur')
						}}
						code={rawValue}
						bind:value
					/>
				{/if}
				{#if inputCat == 'list'}
					<div class="block">
						<Toggle
							on:change={(e) => {
								isListJson = !isListJson
							}}
							checked={isListJson}
							textClass="text-secondary"
							size="xs"
							options={{ right: 'json' }}
						/>
					</div>
				{/if}
			{:else if inputCat == 'enum'}
				<div class="flex flex-row w-full gap-1">
					<ArgEnum
						{required}
						create={extra['disableCreate'] != true}
						{defaultValue}
						{valid}
						{disabled}
						bind:value
						{enum_}
						{autofocus}
					/>
				</div>
			{:else if inputCat == 'date'}
				{#if format === 'date'}
					<DateInput {autofocus} bind:value dateFormat={extra?.['dateFormat']} />
				{:else}
					<DateTimeInput useDropdown {autofocus} bind:value />
				{/if}
			{:else if inputCat == 'sql' || inputCat == 'yaml'}
				<div class="border my-1 mb-4 w-full border-primary">
					<SimpleEditor
						on:focus={(e) => {
							dispatch('focus')
						}}
						on:blur={(e) => {
							dispatch('blur')
						}}
						bind:this={editor}
						lang={inputCat}
						bind:code={value}
						autoHeight
					/>
				</div>
			{:else if inputCat == 'base64'}
				<div class="flex flex-col w-full">
					<input
						{autofocus}
						type="file"
						on:change={(x) => fileChanged(x, (val) => (value = val))}
						multiple={false}
					/>
					{#if value?.length}
						<div class="text-2xs text-tertiary mt-1">File length: {value.length} base64 chars</div>
					{/if}
				</div>
			{:else if inputCat == 'resource-string'}
				<ResourcePicker
					selectFirst
					{disablePortal}
					bind:value
					resourceType={format && format.split('-').length > 1
						? format.substring('resource-'.length)
						: undefined}
					{showSchemaExplorer}
				/>
			{:else if inputCat == 'email'}
				<input
					{autofocus}
					on:focus
					on:blur
					{disabled}
					type="email"
					class={valid
						? ''
						: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-3'}
					placeholder={placeholder ?? defaultValue ?? ''}
					bind:value
				/>
			{:else if inputCat == 'string'}
				<StringProperty
					{label}
					bind:value
					bind:pickForField
					on:blur
					on:focus
					{autofocus}
					{placeholder}
					{variableEditor}
					{itemPicker}
					{disabled}
					{extra}
					{onlyMaskPassword}
					{valid}
					{password}
					{defaultValue}
				/>
			{/if}
			<slot name="actions" />
		</div>

		{#if !compact || (error && error != '')}
			{#if disabled || error === ''}
				<div class="text-right text-xs text-red-600 dark:text-red-400"> &nbsp; </div>
			{:else}
				<div class="text-right text-xs text-red-600 dark:text-red-400">{error}</div>
			{/if}
		{:else if !noMargin}
			<div class="mb-2" />
		{/if}
	</div>
	{#if dndEnabled}
		<GripVertical class="w-4 h-4 text-gray-500 cursor-move" />
	{/if}
</div>

<style>
	input::-webkit-outer-spin-button,
	input::-webkit-inner-spin-button {
		-webkit-appearance: none !important;
		margin: 0;
	}

	/* Firefox */
	input[type='number'] {
		-moz-appearance: textfield !important;
	}
</style>
