<script lang="ts">
	import { setInputCat as computeInputCat, emptyString } from '$lib/utils'
	import { Button } from './common'
	import { createEventDispatcher, tick } from 'svelte'
	import FieldHeader from './FieldHeader.svelte'
	import type { EnumType, SchemaProperty } from '$lib/common'
	import autosize from '$lib/autosize'
	import Toggle from './Toggle.svelte'
	import Range from './Range.svelte'
	import LightweightSchemaForm from './LightweightSchemaForm.svelte'
	import type { ComponentCustomCSS } from './apps/types'
	import { twMerge } from 'tailwind-merge'
	import { fade } from 'svelte/transition'
	import { Plus, X } from 'lucide-svelte'
	import LightweightResourcePicker from './LightweightResourcePicker.svelte'
	import LightweightObjectResourceInput from './LightweightObjectResourceInput.svelte'
	import DateTimeInput from './DateTimeInput.svelte'
	import DateInput from './DateInput.svelte'
	import CurrencyInput from './apps/components/inputs/currency/CurrencyInput.svelte'
	import Multiselect from 'svelte-multiselect'
	import Password from './Password.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'

	export let css: ComponentCustomCSS<'schemaformcomponent'> | undefined = undefined
	export let label: string = ''
	export let value: any

	export let defaultValue: any = undefined

	export let description: string = ''
	export let format: string = ''
	export let contentEncoding: 'base64' | 'binary' | undefined = undefined
	export let type: string | undefined = undefined
	export let oneOf: SchemaProperty[] | undefined = undefined
	export let required = false
	export let pattern: undefined | string = undefined
	export let valid = required ? false : true
	export let enum_: EnumType = undefined
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
	export let extra: Record<string, any> = {}
	export let displayType: boolean = true
	export let customErrorMessage: string | undefined = undefined
	export let render = true
	export let title: string | undefined = undefined
	export let placeholder: string | undefined = undefined

	let oneOfSelected: string | undefined = undefined
	async function updateOneOfSelected(oneOf: SchemaProperty[] | undefined) {
		if (
			oneOf &&
			oneOf.length >= 2 &&
			(!oneOfSelected || !oneOf.some((o) => o.title === oneOfSelected))
		) {
			if (value && value['label'] && oneOf.some((o) => o.title === value['label'])) {
				const existingValue = JSON.parse(JSON.stringify(value))
				oneOfSelected = value['label']
				await tick()
				value = existingValue
			} else {
				oneOfSelected = oneOf[0]['title']
			}
		}
	}
	$: updateOneOfSelected(oneOf)
	function updateOneOfSelectedValue(oneOfSelected) {
		if (oneOfSelected) {
			value = { label: oneOfSelected }
		}
	}
	$: updateOneOfSelectedValue(oneOfSelected)

	const dispatch = createEventDispatcher()

	export let error: string = ''

	let el: HTMLTextAreaElement | undefined = undefined

	let rawValue: string | undefined = undefined

	$: inputCat = computeInputCat(type, format, itemsType?.type, enum_, contentEncoding)

	$: render && changeDefaultValue(inputCat, defaultValue)

	$: rawValue && evalRawValueToValue()

	$: validateInput(pattern, value, required)

	$: {
		if (inputCat === 'object') {
			evalValueToRaw()
		}
	}

	function evalRawValueToValue() {
		if (rawValue) {
			try {
				value = JSON.parse(rawValue)
				error = ''
			} catch (err) {
				error = err.toString()
			}
		}
	}

	export function evalValueToRaw() {
		if (value) {
			rawValue = JSON.stringify(value, null, 4)
		}
	}

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

	function validateInput(pattern: string | undefined, v: any, required: boolean): void {
		if (!render) {
			error = ''
			value = true
		}
		if (required && (v == undefined || v == null || v === '')) {
			error = 'Required'
			valid && (valid = false)
		} else {
			if (pattern && !testRegex(pattern, v)) {
				if (!emptyString(customErrorMessage)) {
					error = customErrorMessage ?? ''
				} else if (format == 'email') {
					error = 'invalid email address'
				} else {
					error = `should match ${pattern}`
				}
				valid && (valid = false)
			} else {
				error = ''
				!valid && (valid = true)
			}
		}
	}

	function testRegex(pattern: string, value: any): boolean {
		try {
			const regex = new RegExp(pattern)
			return regex.test(value)
		} catch (err) {
			return false
		}
	}

	async function changeDefaultValue(inputCat, defaultValue) {
		value = defaultValue
		if (value == null || value == undefined) {
			if (defaultValue === undefined || defaultValue === null) {
				if (inputCat === 'string') {
					value = ''
				} else if (inputCat == 'enum' && required) {
					value = enum_?.[0]
				} else if (inputCat == 'boolean') {
					value = false
				}
			}
		}

		if (inputCat === 'object') {
			evalValueToRaw()
		}
	}

	$: render == false && onRenderFalse()

	function onRenderFalse() {
		value = undefined
		valid = true
		error = ''
	}
</script>

{#if render}
	<div class="flex flex-col w-full min-w-[250px]">
		<div>
			{#if displayHeader}
				<FieldHeader
					prettify={emptyString(title)}
					label={title && !emptyString(title) ? title : label}
					{required}
					{type}
					{contentEncoding}
					{format}
					{displayType}
					labelClass={css?.label?.class}
				/>
			{/if}

			{#if description}
				<div class={twMerge('text-xs italic pb-1', css?.description?.class)}>
					<pre class="font-main whitespace-normal">{description}</pre>
				</div>
			{/if}

			<div class="flex space-x-1">
				{#if inputCat == 'number'}
					{#if extra['min'] != undefined && extra['max'] != undefined}
						<Range bind:value min={extra['min']} max={extra['max']} {defaultValue} />
					{:else if extra?.currency}
						<CurrencyInput
							inputClasses={{
								formatted: 'px-2 w-full py-1.5 text-black dark:text-white',
								wrapper: 'w-full windmillapp',
								formattedZero: 'text-black dark:text-white'
							}}
							noColor
							bind:value
							currency={extra?.currency}
							locale={extra?.currencyLocale ?? 'en-US'}
						/>
					{:else}
						<input
							on:focus={(e) => {
								dispatch('focus')
							}}
							type="number"
							class={twMerge(
								valid && error == ''
									? ''
									: 'border !border-red-700 !border-opacity-70 focus:!border-red-700 focus:!border-opacity-30'
							)}
							placeholder={placeholder ?? defaultValue ?? ''}
							bind:value
							min={extra['min']}
							max={extra['max']}
						/>
					{/if}
				{:else if inputCat == 'boolean'}
					<Toggle
						on:pointerdown={(e) => {
							e?.stopPropagation()
						}}
						class={twMerge(
							valid && error == ''
								? ''
								: 'border !border-red-700 !border-opacity-70 focus:!border-red-700 focus:!border-opacity-30',
							'w-full'
						)}
						bind:checked={value}
					/>
					{#if type == 'boolean' && value == undefined}
						<span>&nbsp; Not set</span>
					{/if}
				{:else if inputCat == 'list'}
					<div class="w-full">
						{#if Array.isArray(itemsType?.multiselect) && Array.isArray(value)}
							<div class="items-start">
								<Multiselect
									ulOptionsClass={'!bg-surface-secondary'}
									bind:selected={value}
									options={itemsType?.multiselect ?? []}
									selectedOptionsDraggable={true}
								/>
							</div>
						{:else if Array.isArray(itemsType?.enum) && Array.isArray(value)}
							<div class="items-start">
								<Multiselect
									ulOptionsClass={'!bg-surface-secondary'}
									bind:selected={value}
									options={itemsType?.enum ?? []}
									selectedOptionsDraggable={true}
								/>
							</div>
						{:else if Array.isArray(enum_) && Array.isArray(value)}
							<div class="items-start">
								<Multiselect
									ulOptionsClass={'!bg-surface-secondary'}
									bind:selected={value}
									options={enum_ ?? []}
									selectedOptionsDraggable={true}
								/>
							</div>
						{:else}
							<div class="w-full">
								{#if Array.isArray(value)}
									{#each value ?? [] as v, i}
										<div class="flex flex-row max-w-md mt-1 w-full">
											{#if itemsType?.type == 'number'}
												<input type="number" bind:value={v} />
											{:else if itemsType?.type == 'string' && itemsType?.contentEncoding == 'base64'}
												<input
													type="file"
													class="my-6"
													on:change={(x) => fileChanged(x, (val) => (value[i] = val))}
													multiple={false}
												/>
											{:else if Array.isArray(itemsType?.enum)}
												<select
													on:focus={(e) => {
														dispatch('focus')
													}}
													class="px-6"
													bind:value={v}
												>
													{#each itemsType?.enum ?? [] as e}
														<option>{e}</option>
													{/each}
												</select>
											{:else}
												<input type="text" bind:value={v} />
											{/if}
											<button
												transition:fade|local={{ duration: 100 }}
												class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover ml-2"
												aria-label="Clear"
												on:click={() => {
													value = value.filter((el) => el != v)
													if (value.length == 0) {
														value = undefined
													}
												}}
											>
												<X size={14} />
											</button>
										</div>
									{/each}
								{:else if value != undefined}
									List is not an array
								{/if}
							</div>
							<div class="flex my-2">
								<Button
									variant="border"
									color="light"
									size="sm"
									btnClasses="mt-1"
									on:click={() => {
										if (value == undefined || !Array.isArray(value)) {
											value = []
										}
										value = value.concat('')
									}}
									startIcon={{ icon: Plus }}
								>
									Add
								</Button>
							</div>
							<span class="ml-2">
								{(value ?? []).length} item{(value ?? []).length != 1 ? 's' : ''}
							</span>
						{/if}
					</div>
				{:else if inputCat == 'resource-object'}
					<LightweightObjectResourceInput {format} bind:value />
				{:else if inputCat == 'object' && oneOf && oneOf.length >= 2}
					<div class="flex flex-col gap-2 w-full">
						{#if oneOf && oneOf.length >= 2}
							<ToggleButtonGroup bind:selected={oneOfSelected}>
								{#each oneOf as obj}
									<ToggleButton value={obj.title} label={obj.title} />
								{/each}
							</ToggleButtonGroup>

							{#if oneOfSelected}
								{@const objIdx = oneOf.findIndex((o) => o.title === oneOfSelected)}
								{@const obj = oneOf[objIdx]}
								{#if obj && obj.properties && Object.keys(obj.properties).length > 0}
									<div class="p-4 pl-8 border rounded w-full">
										<LightweightSchemaForm
											schema={{
												properties: Object.fromEntries(
													Object.entries(obj.properties).filter(([k, v]) => k !== 'label')
												),
												$schema: '',
												required: obj.required ?? [],
												type: 'object'
											}}
											bind:args={value}
										/>
									</div>
								{/if}
							{/if}
						{/if}
					</div>
				{:else if inputCat == 'object'}
					{#if properties && Object.keys(properties).length > 0}
						<div class="p-4 pl-8 border rounded w-full">
							<LightweightSchemaForm
								schema={{
									properties,
									$schema: '',
									required: nestedRequired ?? [],
									type: 'object'
								}}
								bind:args={value}
							/>
						</div>
					{:else}
						<textarea
							bind:this={el}
							on:focus={(e) => {
								dispatch('focus')
							}}
							use:autosize
							style="min-height: 5px;"
							class="col-span-10 {valid && error == ''
								? ''
								: 'border !border-red-700 !border-opacity-70 focus:!border-red-700 focus:!border-opacity-30'}"
							placeholder={defaultValue ? JSON.stringify(defaultValue, null, 4) : ''}
							bind:value={rawValue}
						/>
					{/if}
				{:else if inputCat == 'enum'}
					<select
						on:focus={(e) => {
							dispatch('focus')
						}}
						class="px-6"
						bind:value
					>
						{#each enum_ ?? [] as e}
							<option value={e}>{extra?.['enumLabels']?.[e] ?? e}</option>
						{/each}
					</select>
				{:else if inputCat == 'date'}
					{#if format === 'date'}
						<DateInput bind:value dateFormat={extra['dateFormat']} />
					{:else}
						<DateTimeInput useDropdown bind:value />
					{/if}
				{:else if inputCat == 'base64'}
					<div class="flex flex-col my-6 w-full">
						<input
							type="file"
							on:change={(x) => fileChanged(x, (val) => (value = val))}
							multiple={false}
						/>
						{#if value?.length}
							<div class="text-2xs text-tertiary mt-1">File length: {value.length} base64 chars</div
							>
						{/if}
					</div>
				{:else if inputCat == 'resource-string'}
					<div class="flex flex-row gap-x-1 w-full">
						<LightweightResourcePicker
							bind:value
							resourceType={format.split('-').length > 1
								? format.substring('resource-'.length)
								: undefined}
						/>
					</div>
				{:else if inputCat == 'email'}
					<input
						on:focus
						type="email"
						class={valid
							? ''
							: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-3'}
						placeholder={placeholder ?? defaultValue ?? ''}
						bind:value
					/>
				{:else if inputCat == 'currency'}
					<input
						type="number"
						class={valid
							? ''
							: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-3'}
						placeholder={placeholder ?? defaultValue ?? ''}
						bind:value
					/>
				{:else if inputCat == 'string'}
					<div class="flex flex-col w-full">
						<div class="flex flex-row w-full items-center justify-between">
							{#if extra?.['password'] == true}
								<Password bind:password={value} />
							{:else}
								<textarea
									rows={extra?.['rows'] || 1}
									bind:this={el}
									on:focus={(e) => {
										dispatch('focus')
									}}
									use:autosize
									class="col-span-10 {valid && error == ''
										? ''
										: 'border !border-red-700 !border-opacity-70 focus:!border-red-700 focus:!border-opacity-30'}"
									placeholder={placeholder ?? defaultValue ?? ''}
									bind:value
									on:pointerdown|stopPropagation={(e) => {
										dispatch('inputClicked', e)
									}}
								/>
							{/if}
						</div>
					</div>
				{/if}
				<slot name="actions" />
			</div>
			{#if error && error != ''}
				<div class="text-right text-xs text-red-600 dark:text-red-400">
					{#if error === ''}
						&nbsp;
					{:else}
						{error}
					{/if}
				</div>
			{/if}
		</div>
	</div>
{/if}

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
