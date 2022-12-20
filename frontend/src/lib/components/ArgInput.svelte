<script lang="ts">
	import {
		faChevronDown,
		faChevronUp,
		faDollarSign,
		faMinus,
		faPlus
	} from '@fortawesome/free-solid-svg-icons'

	import { setInputCat as computeInputCat, type InputCat } from '$lib/utils'
	import { Button } from './common'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import FieldHeader from './FieldHeader.svelte'
	import ObjectResourceInput from './ObjectResourceInput.svelte'
	import ObjectTypeNarrowing from './ObjectTypeNarrowing.svelte'
	import ResourcePicker from './ResourcePicker.svelte'
	import StringTypeNarrowing from './StringTypeNarrowing.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import type { SchemaProperty } from '$lib/common'
	import SimpleEditor from './SimpleEditor.svelte'
	import autosize from 'svelte-autosize'
	import Toggle from './Toggle.svelte'
	import Password from './Password.svelte'
	import type VariableEditor from './VariableEditor.svelte'
	import type ItemPicker from './ItemPicker.svelte'

	export let label: string = ''
	export let value: any

	export let defaultValue: any = undefined

	export let description: string = ''
	export let format: string = ''
	export let contentEncoding: 'base64' | 'binary' | undefined = undefined
	export let type: string | undefined = undefined
	export let required = false
	export let pattern: undefined | string = undefined
	export let valid = required ? false : true
	export let maxRows = 10
	export let enum_: string[] | undefined = undefined
	export let disabled = false
	export let editableSchema = false
	export let itemsType:
		| { type?: 'string' | 'number' | 'bytes'; contentEncoding?: 'base64' }
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

	let seeEditable: boolean = enum_ != undefined || pattern != undefined
	const dispatch = createEventDispatcher()

	$: maxHeight = maxRows ? `${1 + maxRows * 1.2}em` : `auto`

	$: validateInput(pattern, value)

	let error: string = ''

	let el: HTMLTextAreaElement | undefined = undefined

	export let editor: SimpleEditor | undefined = undefined

	let rawValue: string | undefined = undefined

	$: {
		if (rawValue) {
			try {
				value = JSON.parse(rawValue)
				error = ''
			} catch (err) {
				error = err.toString()
			}
		}
	}

	$: {
		error = ''
		if (inputCat === 'object') {
			evalValueToRaw()
			validateInput(pattern, value)
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

	function validateInput(pattern: string | undefined, v: any): void {
		if (required && (v == undefined || v == null || v === '')) {
			error = 'This field is required'
			valid = false
		} else {
			if (pattern && !testRegex(pattern, v)) {
				error = `Should match ${pattern}`
				valid = false
			} else {
				error = ''
				valid = true
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

	$: {
		if (value == undefined || value == null) {
			value = defaultValue
			if (defaultValue === undefined || defaultValue === null)
				if (inputCat === 'string') {
					value = ''
				} else if (inputCat == 'enum') {
					value = enum_?.[0]
				}
		}
	}

	export let inputCat: InputCat = 'string'
	$: inputCat = computeInputCat(type, format, itemsType?.type, enum_, contentEncoding)
</script>

<div class="flex flex-col w-full">
	<div>
		{#if displayHeader}
			<FieldHeader {label} {required} {type} {contentEncoding} {format} {itemsType} />
		{/if}
		{#if editableSchema}
			<div class="my-1 text-xs border-solid border border-gray-400 rounded p-2">
				<span
					class="underline"
					on:click={() => {
						seeEditable = !seeEditable
					}}
				>
					Customize property
					<Icon class="ml-2" data={seeEditable ? faChevronUp : faChevronDown} scale={0.7} />
				</span>

				{#if seeEditable}
					<div class="mt-2">
						<label class="text-gray-700">
							Description
							<textarea
								class="mb-1"
								use:autosize
								rows="1"
								bind:value={description}
								placeholder="Field description"
							/>
							{#if type == 'string' && !contentEncoding && format != 'date-time'}
								<StringTypeNarrowing bind:format bind:pattern bind:enum_ bind:contentEncoding />
							{:else if type == 'object'}
								<ObjectTypeNarrowing bind:format />
							{:else if type == 'array'}
								<select bind:value={itemsType}>
									<option value={undefined}>No specific item type</option>
									<option value={{ type: 'string' }}> Items are strings</option>
									<option value={{ type: 'number' }}>Items are numbers</option>
									<option value={{ type: 'string', contentEncoding: 'base64' }}
										>Items are bytes</option
									>
								</select>
							{/if}
						</label>
					</div>
				{/if}
			</div>
			<span class="text-2xs">Input preview:</span>
		{/if}

		{#if description}
			<div class="text-sm italic pb-1">
				{description}
			</div>
		{/if}

		<div class="flex space-x-1">
			{#if inputCat == 'number'}
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
					on:input={() => dispatch('input', { value, isRaw: true })}
				/>
			{:else if inputCat == 'boolean'}
				<Toggle
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
				<div>
					<div>
						{#each value ?? [] as v, i}
							<div class="flex flex-row max-w-md mt-1">
								{#if itemsType?.type == 'number'}
									<input autofocus={autofocus && i == 0} type="number" bind:value={v} />
								{:else if itemsType?.type == 'string' && itemsType?.contentEncoding == 'base64'}
									<input
										autofocus={autofocus && i == 0}
										type="file"
										class="my-6"
										on:change={(x) => fileChanged(x, (val) => (value[i] = val))}
										multiple={false}
									/>
								{:else}
									<input autofocus={autofocus && i == 0} type="text" bind:value={v} />
								{/if}
								<Button
									variant="border"
									color="red"
									size="sm"
									btnClasses="mx-6"
									on:click={() => {
										value = value.filter((el) => el != v)
										if (value.length == 0) {
											value = undefined
										}
									}}
								>
									<Icon data={faMinus} />
								</Button>
							</div>
						{/each}
					</div>
					<Button
						variant="border"
						color="blue"
						size="sm"
						btnClasses="mt-1"
						on:click={() => {
							if (value == undefined || !Array.isArray(value)) {
								value = []
							}
							value = value.concat('')
						}}
					>
						<Icon data={faPlus} class="mr-2" />
						Add item
					</Button>
					<span class="ml-2">
						{(value ?? []).length} item{(value ?? []).length > 1 ? 's' : ''}
					</span>
				</div>
			{:else if inputCat == 'resource-object'}
				<ObjectResourceInput {format} bind:value />
			{:else if inputCat == 'object'}
				{#if properties && Object.keys(properties).length > 0}
					<div class="p-4 pl-8 border rounded w-full">
						<SchemaForm
							{disabled}
							schema={{ properties, $schema: '', required: [], type: 'object' }}
							bind:args={value}
						/>
					</div>
				{:else}
					<textarea
						bind:this={el}
						on:focus
						{autofocus}
						{disabled}
						use:autosize
						style="max-height: {maxHeight}"
						on:input={() => {
							dispatch('input', { rawValue: value, isRaw: false })
						}}
						class="col-span-10 {valid
							? ''
							: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'}"
						placeholder={defaultValue ? JSON.stringify(defaultValue, null, 4) : ''}
						bind:value={rawValue}
					/>
				{/if}
			{:else if inputCat == 'enum'}
				<select {disabled} class="px-6" bind:value>
					{#each enum_ ?? [] as e}
						<option>{e}</option>
					{/each}
				</select>
			{:else if inputCat == 'date'}
				<input {autofocus} class="inline-block" type="datetime-local" bind:value />
			{:else if inputCat == 'sql'}
				<div class="border rounded mb-4 w-full border-gray-700">
					<SimpleEditor
						on:focus={() => dispatch('focus')}
						on:blur={() => dispatch('blur')}
						bind:this={editor}
						lang="sql"
						bind:code={value}
						class="few-lines-editor"
						on:change={async () => {
							dispatch('input', { rawValue: value, isRaw: false })
						}}
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
					bind:value
					resourceType={format.split('-').length > 1
						? format.substring('resource-'.length)
						: undefined}
				/>
			{:else if inputCat == 'string'}
				<div class="flex flex-col w-full">
					<div class="flex flex-row w-full items- justify-between">
						{#if password}
							<Password {disabled} bind:password={value} />
						{:else}
							<textarea
								{autofocus}
								rows="1"
								bind:this={el}
								on:focus={() => dispatch('focus')}
								on:blur={() => dispatch('blur')}
								use:autosize
								type="text"
								{disabled}
								class="col-span-10 {valid
									? ''
									: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'}"
								placeholder={defaultValue ?? ''}
								bind:value
								on:input={() => {
									dispatch('input', { rawValue: value, isRaw: false })
								}}
							/>
							{#if itemPicker}
								<div class="ml-1 relative">
									<Button
										{disabled}
										variant="border"
										color="blue"
										size="sm"
										btnClasses="min-w-min items-center leading-4 py-0"
										on:click={() => {
											pickForField = label
											itemPicker?.openDrawer?.()
										}}><Icon data={faDollarSign} /></Button
									>
								</div>
							{/if}
						{/if}
					</div>
					{#if variableEditor}
						<div class="text-sm text-gray-600">
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
			{#if !required && inputCat != 'resource-object'}
				<!-- <Tooltip placement="bottom" content="Reset to default value">
					<Button
						on:click={() => (value = undefined)}
						{disabled}
						color="alternative"
						size="sm"
						class="h-8"
					>
						<Icon data={faArrowRotateLeft} />
					</Button>
				</Tooltip> -->
			{/if}
			<slot name="actions" />
		</div>
		{#if !compact || (error && error != '')}
			<div class="text-right text-xs text-red-600">
				{#if error === ''}
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
