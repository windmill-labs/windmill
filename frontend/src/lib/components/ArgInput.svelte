<script lang="ts">
	import type { EnumType, SchemaProperty } from '$lib/common'
	import {
		setInputCat as computeInputCat,
		debounce,
		emptyString,
		getSchemaFromProperties
	} from '$lib/utils'
	import { DollarSign, Pipette, Plus, X } from 'lucide-svelte'
	import { createEventDispatcher, tick } from 'svelte'
	import Multiselect from 'svelte-multiselect'
	import { fade } from 'svelte/transition'
	import JsonEditor from './apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'
	import { Button, SecondsInput } from './common'
	import FieldHeader from './FieldHeader.svelte'
	import type ItemPicker from './ItemPicker.svelte'
	import ObjectResourceInput from './ObjectResourceInput.svelte'
	import Range from './Range.svelte'
	import ResourcePicker from './ResourcePicker.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import Toggle from './Toggle.svelte'
	import type VariableEditor from './VariableEditor.svelte'
	import { twMerge } from 'tailwind-merge'
	import ArgEnum from './ArgEnum.svelte'
	import DateTimeInput from './DateTimeInput.svelte'
	import DateInput from './DateInput.svelte'
	import S3FilePicker from './S3FilePicker.svelte'
	import CurrencyInput from './apps/components/inputs/currency/CurrencyInput.svelte'
	import FileUpload from './common/fileUpload/FileUpload.svelte'
	import autosize from '$lib/autosize'
	import PasswordArgInput from './PasswordArgInput.svelte'
	import Password from './Password.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import SchemaFormDnd from './schema/SchemaFormDND.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import { deepEqual } from 'fast-equals'
	import DynSelect from './DynSelect.svelte'
	import type { Script } from '$lib/gen'

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
	export let valid = true
	export let enum_: EnumType = undefined
	export let disabled = false
	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object' | 'resource'
				contentEncoding?: 'base64'
				enum?: string[]
				multiselect?: string[]
				resourceType?: string
				properties?: { [name: string]: SchemaProperty }
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
	export let order: string[] | undefined = undefined
	export let editor: SimpleEditor | undefined = undefined
	export let orderEditable = false
	export let shouldDispatchChanges: boolean = false
	export let noDefaultOnSelectFirst: boolean = false
	export let helperScript:
		| { type: 'inline'; path?: string; lang: Script['language']; code: string }
		| { type: 'hash'; hash: string }
		| undefined = undefined
	export let otherArgs: Record<string, any> = {}

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
	function updateOneOfSelectedValue(oneOfSelected: string | undefined) {
		if (oneOfSelected) {
			value = { label: oneOfSelected }
		}
	}
	$: updateOneOfSelectedValue(oneOfSelected)

	const dispatch = createEventDispatcher()

	let ignoreValueUndefined = false
	let error: string = ''
	let s3FilePicker: S3FilePicker
	let s3FileUploadRawMode: false
	let isListJson = false

	let el: HTMLTextAreaElement | undefined = undefined
	let inputCat = computeInputCat(type, format, itemsType?.type, enum_, contentEncoding)

	$: inputCat = computeInputCat(type, format, itemsType?.type, enum_, contentEncoding)
	let rawValue: string | undefined = undefined

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
				evalValueToRaw()
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

	function evalValueToRaw() {
		rawValue =
			inputCat === 'object' || inputCat === 'resource-object' || (inputCat == 'list' && !isListJson)
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
		if (nullable && emptyString(v)) {
			error = ''
			valid && (valid = true)
		} else if (required && (v == undefined || v == null || v === '')) {
			error = 'Required'
			valid && (valid = false)
		} else {
			if (inputCat == 'number' && typeof v === 'number') {
				let min = extra['min']
				let max = extra['max']
				if (min != undefined && typeof min == 'number' && v < min) {
					error = `Should be greater than or equal to ${min}`
					valid && (valid = false)
				} else if (max != undefined && typeof max == 'number' && v > max) {
					error = `Should be less than or equal to ${max}`
					valid && (valid = false)
				} else {
					error = ''
					!valid && (valid = true)
				}
			} else if (pattern && !testRegex(pattern, v)) {
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

	function testRegex(pattern: string, value: any): boolean {
		try {
			const regex = new RegExp(pattern)
			return regex.test(value)
		} catch (err) {
			return false
		}
	}

	function onKeyDown(e: KeyboardEvent) {
		if (
			(e.ctrlKey || e.metaKey) &&
			(e.key == 'Enter' || e.key == 'c' || e.key == 'v' || e.key == 'x')
		) {
			return
		}
		e.stopPropagation()
	}

	let redraw = 0
	let itemsLimit = 50

	$: validateInput(pattern, value, required)

	let oldValue = value

	function compareValues(value) {
		if (!deepEqual(oldValue, value)) {
			oldValue = value
			dispatch('change')
		}
	}

	let debounced = debounce(() => compareValues(value), 50)
	$: shouldDispatchChanges && debounced(value)
</script>

<S3FilePicker
	bind:this={s3FilePicker}
	bind:selectedFileKey={value}
	on:close={() => {
		rawValue = JSON.stringify(value, null, 2)
		editor?.setCode(rawValue)
	}}
	readOnlyMode={false}
/>
<!-- svelte-ignore a11y-autofocus -->
<div class="flex flex-col w-full {minW ? 'min-w-[250px]' : ''}">
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
			<div class="text-xs italic pb-1 text-secondary">
				<pre class="font-main whitespace-normal">{description}</pre>
			</div>
		{/if}

		<div class="flex space-x-1">
			{#if inputCat == 'number'}
				{#if extra['min'] != undefined && extra['max'] != undefined}
					<Range bind:value min={extra['min']} max={extra['max']} {defaultValue} />
				{:else if extra['seconds'] !== undefined}
					<SecondsInput bind:seconds={value} on:focus />
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
					<div class="relative w-full">
						<input
							{autofocus}
							on:focus
							on:blur
							{disabled}
							type="number"
							on:keydown={() => {
								ignoreValueUndefined = true
							}}
							class={valid
								? ''
								: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'}
							placeholder={placeholder ?? defaultValue ?? ''}
							bind:value
							min={extra['min']}
							max={extra['max']}
						/>
					</div>
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
													{:else if itemsType?.type == 'object' && itemsType?.resourceType === undefined && itemsType?.properties === undefined}
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
															enumLabels={extra['enumLabels']}
														/>
													{:else if itemsType?.type == 'resource' && itemsType?.resourceType && resourceTypes?.includes(itemsType.resourceType)}
														<ObjectResourceInput
															value={v ? `$res:${v}` : undefined}
															bind:path={v}
															format={'resource-' + itemsType?.resourceType}
															defaultValue={undefined}
														/>
													{:else if itemsType?.type == 'resource'}
														<JsonEditor
															bind:editor
															on:focus={(e) => {
																dispatch('focus')
															}}
															on:blur={(e) => {
																dispatch('blur')
															}}
															code={JSON.stringify(v, null, 2)}
															bind:value={v}
														/>
													{:else if itemsType?.type === 'object' && itemsType?.properties}
														<div class="p-8 border rounded-md w-full">
															<SchemaForm
																{onlyMaskPassword}
																{disablePortal}
																{disabled}
																noDelete
																schema={getSchemaFromProperties(itemsType?.properties)}
																bind:args={v}
															/>
														</div>
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
										if (itemsType?.type == 'number') {
											value = value.concat(0)
										} else if (
											itemsType?.type == 'object' ||
											(itemsType?.type == 'resource' &&
												!(
													itemsType?.resourceType &&
													resourceTypes?.includes(itemsType?.resourceType)
												))
										) {
											value = value.concat({})
										} else {
											value = value.concat('')
										}
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
								evalValueToRaw()
								isListJson = !isListJson
							}}
							checked={isListJson}
							textClass="text-secondary"
							size="xs"
							options={{ right: 'json' }}
						/>
					</div>
				</div>
			{:else if inputCat == 'dynselect'}
				<DynSelect
					name={label}
					args={otherArgs}
					{helperScript}
					bind:value
					entrypoint={format.substring('dynselect_'.length)}
				/>
			{:else if inputCat == 'resource-object' && resourceTypes == undefined}
				<span class="text-2xs text-tertiary">Loading resource types...</span>
			{:else if inputCat == 'resource-object' && (resourceTypes == undefined || (format.split('-').length > 1 && resourceTypes.includes(format.substring('resource-'.length))))}
				<ObjectResourceInput
					{defaultValue}
					selectFirst={!noDefaultOnSelectFirst}
					{disablePortal}
					{format}
					bind:value
					bind:editor
					on:clear={() => {
						defaultValue = null
					}}
					{showSchemaExplorer}
				/>
			{:else if inputCat == 'resource-object' && format.split('-').length > 1 && format
					.replace('resource-', '')
					.replace('_', '')
					.toLowerCase() == 's3object'}
				<div class="flex flex-col w-full gap-1">
					<Toggle
						class="flex justify-end"
						bind:checked={s3FileUploadRawMode}
						size="xs"
						options={{ left: 'Raw S3 object input' }}
					/>
					{#if s3FileUploadRawMode}
						<JsonEditor
							bind:editor
							on:focus={(e) => {
								dispatch('focus')
							}}
							on:blur={(e) => {
								dispatch('blur')
							}}
							code={JSON.stringify(value ?? defaultValue ?? { s3: '' }, null, 2)}
							bind:value
						/>
						<Button
							variant="border"
							color="light"
							size="xs"
							btnClasses="mt-1"
							on:click={() => {
								s3FilePicker?.open?.(value)
							}}
							startIcon={{ icon: Pipette }}
						>
							Choose an object from the catalog
						</Button>
					{:else}
						<FileUpload
							allowMultiple={false}
							randomFileKey={true}
							on:addition={(evt) => {
								value = {
									s3: evt.detail?.path ?? '',
									filename: evt.detail?.filename ?? ''
								}
							}}
							on:deletion={(evt) => {
								value = {
									s3: ''
								}
							}}
							defaultValue={defaultValue?.s3}
						/>
					{/if}
				</div>
			{:else if inputCat == 'object' || inputCat == 'resource-object' || isListJson}
				{#if oneOf && oneOf.length >= 2}
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
										{#if orderEditable}
											<SchemaFormDnd
												{onlyMaskPassword}
												{disablePortal}
												{disabled}
												schema={{
													properties: Object.fromEntries(
														Object.entries(obj.properties).filter(([k, v]) => k !== 'label')
													),
													order: obj.order?.filter((k) => k !== 'label') ?? undefined,
													$schema: '',
													required: obj.required ?? [],
													type: 'object'
												}}
												args={value}
												dndType={`nested-${title}`}
												on:reorder={(e) => {
													if (oneOf && oneOf[objIdx]) {
														const keys = e.detail
														oneOf[objIdx].order = keys
													}
												}}
												on:change
											/>
										{:else}
											<SchemaForm
												{onlyMaskPassword}
												{disablePortal}
												{disabled}
												noDelete
												schema={{
													properties: Object.fromEntries(
														Object.entries(obj.properties).filter(([k, v]) => k !== 'label')
													),
													order: obj.order?.filter((k) => k !== 'label') ?? undefined,
													$schema: '',
													required: obj.required ?? [],
													type: 'object'
												}}
												bind:args={value}
											/>
										{/if}
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
							{/if}
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
					</div>
				{:else if properties && Object.keys(properties).length > 0 && inputCat !== 'list'}
					<div class="p-4 pl-8 border rounded-md w-full">
						{#if orderEditable}
							<SchemaFormDnd
								{onlyMaskPassword}
								{disablePortal}
								{disabled}
								schema={{
									properties,
									$schema: '',
									required: nestedRequired ?? [],
									type: 'object',
									order
								}}
								args={value}
								dndType={`nested-${title}`}
								on:reorder={(e) => {
									const keys = e.detail
									order = keys
								}}
								on:change
							/>
						{:else}
							<SchemaForm
								{onlyMaskPassword}
								{disablePortal}
								{disabled}
								schema={{
									properties,
									order,
									$schema: '',
									required: nestedRequired ?? [],
									type: 'object'
								}}
								bind:args={value}
							/>
						{/if}
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
						enumLabels={extra['enumLabels']}
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
				<div class="flex flex-col my-6 w-full">
					<input
						{autofocus}
						type="file"
						on:change={(x) => fileChanged(x, (val) => (value = val))}
						multiple={false}
					/>
					{#if value?.length}
						<div class="text-2xs text-tertiary mt-1"
							>File length: {value.length} base64 chars ({(value.length / 1024 / 1024).toFixed(
								2
							)}MB)</div
						>
					{/if}
				</div>
			{:else if inputCat == 'resource-string'}
				<ResourcePicker
					selectFirst={noDefaultOnSelectFirst}
					{disablePortal}
					bind:value
					initialValue={defaultValue}
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
				<div class="flex flex-col w-full">
					<div class="flex flex-row w-full items-center justify-between relative">
						{#if password || extra?.['password'] == true}
							{#if value && typeof value == 'string' && value?.startsWith('$var:')}
								<input type="text" bind:value />
							{:else if onlyMaskPassword}
								<Password
									{disabled}
									bind:password={value}
									placeholder={placeholder ?? defaultValue ?? ''}
								/>
							{:else}
								<PasswordArgInput {disabled} bind:value />
							{/if}
						{:else}
							{#key extra?.['minRows']}
								<textarea
									{autofocus}
									rows={extra?.['minRows'] ? extra['minRows']?.toString() : '1'}
									bind:this={el}
									on:focus={(e) => {
										dispatch('focus')
									}}
									on:blur={(e) => {
										dispatch('blur')
									}}
									use:autosize
									on:keydown={onKeyDown}
									{disabled}
									class={twMerge(
										'w-full',
										valid
											? ''
											: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-3'
									)}
									placeholder={placeholder ?? defaultValue ?? ''}
									bind:value
								/>
							{/key}
							{#if !disabled && itemPicker && extra?.['disableVariablePicker'] != true}
								<!-- svelte-ignore a11y-click-events-have-key-events -->
								<button
									class="absolute right-1 top-1 py-1 min-w-min !px-2 items-center text-gray-800 bg-surface-secondary border rounded center-center hover:bg-gray-300 transition-all cursor-pointer"
									on:click={() => {
										pickForField = label
										itemPicker?.openDrawer?.()
									}}
									title="Insert a Variable"
								>
									<DollarSign class="!text-tertiary" size={14} />
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

<style>
	input::-webkit-outer-spin-button,
	input::-webkit-inner-spin-button {
		-webkit-appearance: none !important;
		margin: 0;
	}

	/* Firefox */
	input[type='number'] {
		-moz-appearance: textfield !important;
		appearance: textfield !important;
	}
</style>
