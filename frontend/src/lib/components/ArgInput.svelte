<script lang="ts">
	import type { SchemaProperty } from '$lib/common'
	import { setInputCat as computeInputCat } from '$lib/utils'
	import { ChevronDown, DollarSign, Plus, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import autosize from 'svelte-autosize'
	import Multiselect from 'svelte-multiselect'
	import { fade } from 'svelte/transition'
	import JsonEditor from './apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'
	import { Badge, Button, SecondsInput } from './common'
	import FieldHeader from './FieldHeader.svelte'
	import type ItemPicker from './ItemPicker.svelte'
	import NumberTypeNarrowing from './NumberTypeNarrowing.svelte'
	import ObjectResourceInput from './ObjectResourceInput.svelte'
	import ObjectTypeNarrowing from './ObjectTypeNarrowing.svelte'
	import Password from './Password.svelte'
	import Range from './Range.svelte'
	import ResourcePicker from './ResourcePicker.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import StringTypeNarrowing from './StringTypeNarrowing.svelte'
	import Toggle from './Toggle.svelte'
	import type VariableEditor from './VariableEditor.svelte'
	import { twMerge } from 'tailwind-merge'
	import ArgEnum from './ArgEnum.svelte'
	import ArrayTypeNarrowing from './ArrayTypeNarrowing.svelte'
	import DateTimeInput from './DateTimeInput.svelte'

	export let label: string = ''
	export let value: any

	export let defaultValue: any = undefined

	export let description: string = ''
	export let format: string = ''
	export let contentEncoding: 'base64' | 'binary' | undefined = undefined
	export let type: string | undefined = undefined
	export let required = false
	export let pattern: undefined | string = undefined
	export let valid = true
	export let enum_: string[] | undefined = undefined
	export let disabled = false
	export let editableSchema = false
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
	export let autofocus = false
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

	let seeEditable: boolean = enum_ != undefined || pattern != undefined
	const dispatch = createEventDispatcher()

	let error: string = ''

	let el: HTMLTextAreaElement | undefined = undefined

	export let editor: SimpleEditor | undefined = undefined

	let inputCat = computeInputCat(type, format, itemsType?.type, enum_, contentEncoding)

	$: inputCat = computeInputCat(type, format, itemsType?.type, enum_, contentEncoding)
	let rawValue: string | undefined = undefined

	function computeDefaultValue(nvalue?: any, inputCat?: string, defaultValue?: any) {
		if (value == undefined || value == null) {
			value = defaultValue
			if (defaultValue === undefined || defaultValue === null) {
				if (inputCat === 'string') {
					value = ''
				} else if (inputCat == 'enum') {
					value = enum_?.[0]
				} else if (inputCat == 'boolean') {
					value = false
				} else if (inputCat == 'list') {
					value = []
				}
			} else if (inputCat === 'object') {
				evalValueToRaw()
			}
		}
	}

	computeDefaultValue()

	$: computeDefaultValue(value, inputCat, defaultValue)

	$: defaultValue != undefined && handleDefaultValueChange()

	let oldDefaultValue = defaultValue
	function handleDefaultValueChange() {
		if (value == oldDefaultValue) {
			value = defaultValue
		}
		oldDefaultValue = defaultValue
	}

	function evalValueToRaw() {
		rawValue =
			inputCat === 'object' || inputCat === 'resource-object'
				? JSON.stringify(value, null, 2)
				: undefined
	}

	evalValueToRaw()

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
		if (required && (v == undefined || v == null || v === '')) {
			error = 'Required'
			valid && (valid = false)
		} else {
			if (pattern && !testRegex(pattern, v)) {
				error = `Should match ${pattern}`
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

	function onKeyDown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key == 'Enter') {
			return
		}
		e.stopPropagation()
	}

	let redraw = 0

	let itemsLimit = 50

	let customValue = false

	$: validateInput(pattern, value, required)
</script>

<!-- svelte-ignore a11y-autofocus -->
<div class="flex flex-col w-full {minW ? 'min-w-[250px]' : ''}">
	<div>
		{#if displayHeader}
			<FieldHeader
				prettify={prettifyHeader}
				{label}
				{disabled}
				{required}
				{type}
				{contentEncoding}
				{format}
				{simpleTooltip}
			/>
		{/if}
		{#if editableSchema}
			<label class="text-secondary">
				<textarea
					class="mb-1"
					use:autosize
					rows="1"
					bind:value={description}
					on:keydown={onKeyDown}
					placeholder="Field description"
				/>
			</label>

			{#if type == 'array'}
				<ArrayTypeNarrowing bind:itemsType />
			{:else if (type == 'string' && format != 'date-time') || ['number', 'object'].includes(type ?? '')}
				<div class="p-2 my-1 text-xs border-solid border border-gray-200 rounded-lg">
					<div class="w-min">
						<Button
							on:click={() => {
								seeEditable = !seeEditable
							}}
							endIcon={{
								icon: ChevronDown,
								classes: twMerge('rotate-0 duration-300', seeEditable ? '!rotate-180' : '')
							}}
							color="light"
							size="xs"
						>
							Customize
						</Button>
					</div>

					{#if seeEditable}
						<div class="mt-2">
							{#if type == 'string' && format != 'date-time'}
								<StringTypeNarrowing bind:format bind:pattern bind:enum_ bind:contentEncoding />
							{:else if type == 'number'}
								<NumberTypeNarrowing bind:min={extra['min']} bind:max={extra['max']} />
							{:else if type == 'object'}
								<ObjectTypeNarrowing bind:format />
							{/if}
						</div>
					{/if}
				</div>
			{/if}
			<span class="text-2xs font-semibold">Preview:</span>
		{/if}

		{#if description}
			<div class="text-sm italic pb-1 text-secondary">
				{description}
			</div>
		{/if}

		<div class="flex space-x-1">
			{#if inputCat == 'number'}
				{#if extra['min'] != undefined && extra['max'] != undefined}
					<div class="flex w-full gap-1">
						<span>{extra['min']}</span>
						<div class="grow">
							<Range bind:value min={extra['min']} max={extra['max']} />
						</div>
						<span>{extra['max']}</span>
						<span class="mx-2"><Badge large color="blue">{value}</Badge></span>
					</div>
				{:else if extra['seconds'] !== undefined}
					<SecondsInput bind:seconds={value} on:focus />
				{:else}
					<input
						{autofocus}
						on:focus
						{disabled}
						type="number"
						class={valid
							? ''
							: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'}
						placeholder={defaultValue ?? ''}
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
					{disabled}
					class={valid
						? ''
						: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'}
					bind:checked={value}
				/>
				{#if type == 'boolean' && value == undefined}
					<span>&nbsp; Not set</span>
				{/if}
			{:else if inputCat == 'list'}
				<div class="w-full">
					{#if Array.isArray(itemsType?.multiselect)}
						<div class="items-start">
							<Multiselect
								{disabled}
								bind:selected={value}
								options={itemsType?.multiselect ?? []}
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
						<div class="flex mt-2">
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
			{:else if inputCat == 'resource-object' && resourceTypes == undefined}
				<span class="text-2xs text-tertiary">Loading resource types...</span>
			{:else if inputCat == 'resource-object' && (resourceTypes == undefined || (format.split('-').length > 1 && resourceTypes.includes(format.substring('resource-'.length))))}
				<ObjectResourceInput {disablePortal} {format} bind:value {showSchemaExplorer} />
			{:else if inputCat == 'object' || inputCat == 'resource-object'}
				{#if properties && Object.keys(properties).length > 0}
					<div class="p-4 pl-8 border rounded w-full">
						<SchemaForm
							{disablePortal}
							{disabled}
							schema={{ properties, $schema: '', required: [], type: 'object' }}
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
						code={rawValue}
						bind:value
					/>
				{/if}
			{:else if inputCat == 'enum'}
				<div class="flex flex-row w-full gap-1">
					<ArgEnum {defaultValue} {valid} {customValue} {disabled} bind:value {enum_} {autofocus} />
				</div>
			{:else if inputCat == 'date'}
				<DateTimeInput {autofocus} bind:value />
			{:else if inputCat == 'sql' || inputCat == 'yaml'}
				<div class="border my-1 mb-4 w-full border-gray-400">
					<SimpleEditor
						on:focus={(e) => {
							dispatch('focus')
						}}
						bind:this={editor}
						lang={inputCat}
						bind:code={value}
						autoHeight
					/>
				</div>
			{:else if inputCat == 'base64'}
				<input
					{autofocus}
					type="file"
					class="my-6"
					on:change={(x) => fileChanged(x, (val) => (value = val))}
					multiple={false}
				/>
			{:else if inputCat == 'resource-string'}
				<ResourcePicker
					{disablePortal}
					bind:value
					resourceType={format && format.split('-').length > 1
						? format.substring('resource-'.length)
						: undefined}
					{showSchemaExplorer}
				/>
			{:else if inputCat == 'string'}
				<div class="flex flex-col w-full">
					<div class="flex flex-row w-full items-center justify-between relative">
						{#if password}
							<Password {disabled} bind:password={value} />
						{:else}
							<textarea
								{autofocus}
								rows="1"
								bind:this={el}
								on:focus={(e) => {
									dispatch('focus')
								}}
								use:autosize
								on:keydown={onKeyDown}
								type="text"
								{disabled}
								class={twMerge(
									'w-full',
									valid
										? ''
										: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'
								)}
								placeholder={defaultValue ?? ''}
								bind:value
							/>
							{#if !disabled && itemPicker}
								<!-- svelte-ignore a11y-click-events-have-key-events -->
								<button
									class="absolute right-1 top-1 py-1 min-w-min !px-2 items-center text-gray-800 bg-gray-100 border rounded center-center hover:bg-gray-300 transition-all cursor-pointer"
									on:click={() => {
										pickForField = label
										itemPicker?.openDrawer?.()
									}}
									title="Insert a Variable"
								>
									<DollarSign size={14} />
								</button>
							{/if}
						{/if}
					</div>
					{#if variableEditor}
						<div class="text-sm text-tertiary">
							{#if value && typeof value == 'string' && value?.startsWith('$var:')}
								Linked to variable <button
									class="text-blue-500 underline"
									on:click={() => variableEditor?.editVariable?.(value.slice(5))}
									>{value.slice(5)}</button
								>
							{/if}
						</div>
					{/if}
				</div>
			{/if}
			<slot name="actions" />
		</div>
		{#if !compact || (error && error != '')}
			<div class="text-right text-xs text-red-600 dark:text-red-400">
				{#if disabled || error === ''}
					&nbsp;
				{:else}
					{error}
				{/if}
			</div>
		{:else if !noMargin}
			<div class="mb-2" />
		{/if}
	</div>
</div>
