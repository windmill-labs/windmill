<script lang="ts">
	import { setInputCat as computeInputCat } from '$lib/utils'
	import { Badge, Button } from './common'
	import { createEventDispatcher } from 'svelte'
	import FieldHeader from './FieldHeader.svelte'
	import type { SchemaProperty } from '$lib/common'
	import autosize from 'svelte-autosize'
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

	export let css: ComponentCustomCSS<'schemaformcomponent'> | undefined = undefined
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
	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object'
				contentEncoding?: 'base64'
				enum?: string[]
		  }
		| undefined = undefined
	export let displayHeader = true
	export let properties: { [name: string]: SchemaProperty } | undefined = undefined
	export let extra: Record<string, any> = {}
	export let displayType: boolean = true

	const dispatch = createEventDispatcher()

	$: maxHeight = maxRows ? `${1 + maxRows * 1.2}em` : `auto`

	$: validateInput(pattern, value)

	export let error: string = ''

	let el: HTMLTextAreaElement | undefined = undefined

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
			error = 'Required'
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
			if (defaultValue === undefined || defaultValue === null) {
				if (inputCat === 'string') {
					value = ''
				} else if (inputCat == 'enum') {
					value = enum_?.[0]
				} else if (inputCat == 'boolean') {
					value = false
				}
			}
		}
	}

	$: inputCat = computeInputCat(type, format, itemsType?.type, enum_, contentEncoding)
</script>

<div class="flex flex-col w-full min-w-[250px]">
	<div>
		{#if displayHeader}
			<FieldHeader
				prettify
				{label}
				{required}
				{type}
				{contentEncoding}
				{format}
				{displayType}
				labelClass={css?.label?.class}
			/>
		{/if}

		{#if description}
			<div class={twMerge('text-sm italic pb-1', css?.description?.class)}>
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
					class={valid && error == ''
						? ''
						: 'border !border-red-700 !border-opacity-70 focus:!border-red-700 focus:!border-opacity-30'}
					bind:checked={value}
				/>
				{#if type == 'boolean' && value == undefined}
					<span>&nbsp; Not set</span>
				{/if}
			{:else if inputCat == 'list'}
				<div class="w-full">
					<div class="w-full">
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
						{(value ?? []).length} item{(value ?? []).length > 1 ? 's' : ''}
					</span>
				</div>
			{:else if inputCat == 'resource-object'}
				<LightweightObjectResourceInput {format} bind:value />
			{:else if inputCat == 'object'}
				{#if properties && Object.keys(properties).length > 0}
					<div class="p-4 pl-8 border rounded w-full">
						<LightweightSchemaForm
							schema={{ properties, $schema: '', required: [], type: 'object' }}
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
						style="max-height: {maxHeight}"
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
						<option>{e}</option>
					{/each}
				</select>
			{:else if inputCat == 'date'}
				<DateTimeInput bind:value />
			{:else if inputCat == 'base64'}
				<input
					type="file"
					class="my-6"
					on:change={(x) => fileChanged(x, (val) => (value = val))}
					multiple={false}
				/>
			{:else if inputCat == 'resource-string'}
				<div class="flex flex-row gap-x-1 w-full">
					<LightweightResourcePicker
						bind:value
						resourceType={format.split('-').length > 1
							? format.substring('resource-'.length)
							: undefined}
					/>
				</div>
			{:else if inputCat == 'string'}
				<div class="flex flex-col w-full">
					<div class="flex flex-row w-full items-center justify-between">
						<textarea
							rows="1"
							bind:this={el}
							on:focus={(e) => {
								dispatch('focus')
							}}
							use:autosize
							type="text"
							class="col-span-10 {valid && error == ''
								? ''
								: 'border !border-red-700 !border-opacity-70 focus:!border-red-700 focus:!border-opacity-30'}"
							placeholder={defaultValue ?? ''}
							bind:value
							on:pointerdown|stopPropagation={(e) => {
								dispatch('inputClicked', e)
							}}
						/>
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
