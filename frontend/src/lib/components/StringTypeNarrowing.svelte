<script lang="ts">
	import type { EnumType } from '$lib/common'
	import { computeKind } from '$lib/utils'
	import Label from './Label.svelte'
	import ResourceTypePicker from './ResourceTypePicker.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Button } from './common'
	import Alert from './common/alert/Alert.svelte'
	import ClearableInput from './common/clearableInput/ClearableInput.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import RegexGen from './copilot/RegexGen.svelte'
	import TextInput from './text_input/TextInput.svelte'

	interface Props {
		pattern: string | undefined
		enum_: EnumType
		format: string | undefined
		contentEncoding: 'base64' | 'binary' | undefined
		customErrorMessage: string | undefined
		minRows?: number | undefined
		disableCreate?: boolean | undefined
		disableVariablePicker?: boolean | undefined
		password?: boolean | undefined
		noExtra?: boolean
		dateFormat: string | undefined
		enumLabels?: Record<string, string> | undefined
		overrideAllowKindChange?: boolean
		originalType?: string | undefined
	}

	let {
		pattern = $bindable(),
		enum_ = $bindable(),
		format = $bindable(),
		contentEncoding = $bindable(),
		customErrorMessage = $bindable(),
		minRows = $bindable(undefined),
		disableCreate = $bindable(undefined),
		disableVariablePicker = $bindable(undefined),
		password = $bindable(undefined),
		noExtra = false,
		dateFormat = $bindable(),
		enumLabels = $bindable(undefined),
		overrideAllowKindChange = true,
		originalType = undefined
	}: Props = $props()

	let kind: 'none' | 'pattern' | 'enum' | 'resource' | 'format' | 'base64' | 'date-time' = $state(
		computeKind(enum_, contentEncoding, pattern, format)
	)

	const allowKindChange = overrideAllowKindChange || originalType === 'string'

	let patternStr: string = $state(pattern ?? '')
	let resource: string | undefined = $state()

	const FORMATS = [
		'email',
		'hostname',
		'uri',
		'uuid',
		'ipv4',
		'ipv6',
		'yaml',
		'sql',
		// 'time',
		'date-time',
		'date'
		// 'duration',
		// 'ipv6',
		// 'jsonpointer',
	]

	const FIELD_SETTINGS = [
		['None', 'none'],
		['File', 'base64', 'Encoded as Base 64'],
		['Enum', 'enum'],
		['Datetime', 'date-time'],
		['Format', 'format'],
		['Pattern', 'pattern']
	]

	$effect(() => {
		format =
			kind == 'resource' ? (resource != undefined ? `resource-${resource}` : 'resource') : format
	})
	$effect(() => {
		pattern = patternStr == '' ? undefined : patternStr
	})
	$effect(() => {
		contentEncoding = kind == 'base64' ? 'base64' : undefined
	})

	function add() {
		if (enumLabels === undefined) {
			enumLabels = {}
		}

		let choice = `choice ${enum_?.length ? enum_?.length + 1 : 1}`
		enum_ = enum_ ? (enum_.concat(choice) as EnumType) : [choice]
	}

	function remove(item: string) {
		enum_ = (enum_ || []).filter((el) => el !== item) as EnumType
		if (enum_?.length == 0) {
			enum_ = undefined
		}

		if (enumLabels !== undefined) {
			delete enumLabels[item]
		}
	}

	const presetOptions = [
		{ label: 'ISO Format', format: 'yyyy-MM-dd' },
		{ label: 'US Format', format: 'MM/dd/yyyy' },
		{ label: 'EU Format', format: 'dd/MM/yyyy' }
	]

	function onEnumKeyChange(oldKey: string, newKey: string) {
		if (enumLabels === undefined) {
			enumLabels = {}
		}
		if (oldKey !== newKey) {
			enumLabels[newKey] = enumLabels[oldKey]
			delete enumLabels[oldKey]
		}
	}
</script>

<div class="flex flex-col gap-2 w-full">
	{#if allowKindChange}
		<ToggleButtonGroup
			tabListClass="flex-wrap"
			class="h-auto"
			bind:selected={kind}
			on:selected={(e) => {
				if (e.detail != 'enum') {
					enum_ = undefined
				}
				if (e.detail == 'date-time') {
					format = 'date-time'
				} else if (format) {
					format = undefined
				}
				if (e.detail == 'none') {
					pattern = undefined
					format = undefined
					contentEncoding = undefined
					customErrorMessage = undefined
					minRows = undefined
					disableCreate = undefined
					disableVariablePicker = undefined
				}
			}}
		>
			{#snippet children({ item })}
				{#each FIELD_SETTINGS as x}
					<ToggleButton
						value={x[1]}
						label={x[0]}
						tooltip={x[2]}
						showTooltipIcon={Boolean(x[2])}
						{item}
					/>
				{/each}
			{/snippet}
		</ToggleButtonGroup>
	{/if}
	{#if kind == 'pattern'}
		<Label label="Pattern (Regex)">
			{#snippet header()}
				<Tooltip light>
					Setting a pattern will allow you to specify a regular expression that the input should
					adhere to. You can use the regex generator to help you create a pattern.
				</Tooltip>
			{/snippet}
			<div class="flex flex-row gap-1">
				<ClearableInput
					id="input"
					type="text"
					placeholder="^(\\([0-9]{3}\\))?[0-9]{3}-[0-9]{4}$"
					bind:value={patternStr}
				/>
				<RegexGen
					on:gen={(e) => {
						const { res, prompt } = e.detail
						patternStr = res
						customErrorMessage = 'does not match: ' + prompt
					}}
				/>
			</div>
		</Label>
		<Label label="Custom error message" class="w-full">
			{#snippet header()}
				<Tooltip light>
					Setting a custom error message will allow you to specify a message that will be shown when
					the input does not match the pattern.
				</Tooltip>
			{/snippet}
			{#snippet action()}
				<Toggle
					size="xs"
					options={{ right: 'Enable' }}
					checked={customErrorMessage != undefined && customErrorMessage != ''}
					on:change={(e) => {
						if (e.detail) {
							customErrorMessage = 'Custom error message'
						} else {
							customErrorMessage = undefined
						}
					}}
				/>
			{/snippet}
			<input
				type="text"
				bind:value={customErrorMessage}
				disabled={customErrorMessage == undefined || customErrorMessage == ''}
			/>
		</Label>
	{:else if kind == 'enum'}
		<Label label="Enums">
			{#snippet header()}
				<Tooltip light>
					Setting enums will allow you to specify a list of values that the input can take. If you
					want to allow custom values, you can disable the option below.
				</Tooltip>
			{/snippet}
			<div class="flex flex-col gap-1">
				{#if enum_}
					{#each enum_ as _, i}
						{#if typeof enum_[i] === 'string'}
							<div class="flex flex-row w-full gap-2 pt-2">
								<input
									id="input"
									type="text"
									bind:value={enum_[i]}
									oninput={(event) =>
										enum_ &&
										typeof enum_[i] === 'string' &&
										onEnumKeyChange(event?.currentTarget.value, enum_[i])}
								/>
								{#if enumLabels !== undefined}
									<input
										id="input"
										type="text"
										bind:value={enumLabels[enum_[i]]}
										placeholder="Optional title..."
										oninput={(event) => {
											if (event?.currentTarget.value === '') {
												if (enumLabels === undefined) {
													enumLabels = {}
												}
												if (typeof enum_?.[i] === 'string') {
													enum_ && delete enumLabels[enum_[i]]
												}
											}
										}}
									/>
								{/if}

								{#if allowKindChange}
									<Button
										size="sm"
										on:click={() => enum_ && typeof enum_[i] === 'string' && remove(enum_[i])}
										>-</Button
									>
								{/if}
							</div>
						{:else}
							<div class="flex flex-row w-full gap-2 pt-2">
								{JSON.stringify(enum_[i])} is not a string, remove it
							</div>
						{/if}
					{/each}
				{/if}
			</div>
			{#if allowKindChange}
				<div class="flex flex-row my-1">
					<Button color="light" size="sm" on:click={add}>+</Button>
				</div>
			{/if}
		</Label>
		{#if !noExtra}
			<Toggle
				size="xs"
				options={{ right: 'Disallow creating custom values' }}
				checked={disableCreate}
				on:change={(e) => {
					if (e.detail) {
						disableCreate = true
					} else {
						disableCreate = undefined
					}
				}}
			/>
		{/if}
	{:else if kind == 'resource'}
		<div class="mt-1"></div>
		<ResourceTypePicker bind:value={resource} />
	{:else if kind == 'format'}
		<Label label="Format">
			{#snippet header()}
				<Tooltip light>
					Setting the format will allow you to specify a format that the input should adhere to.
				</Tooltip>
			{/snippet}
			<select bind:value={format}>
				<option value={undefined}></option>
				{#each FORMATS as f}
					<option value={f}>{f}</option>
				{/each}
			</select>
		</Label>
		{#if format == 'date'}
			<div class="mt-1"></div>

			<div class="grid grid-cols-3 gap-2">
				<Label label="Date format passed to script" class="col-span-2">
					{#snippet header()}
						<Tooltip light>
							Setting the date output format will allow you to specify how the date will be passed
							to the script.
						</Tooltip>
					{/snippet}
					<ClearableInput type="text" bind:value={dateFormat} placeholder="yyyy-MM-dd" />
				</Label>
				<Label label="Presets">
					<select
						bind:value={dateFormat}
						disabled={dateFormat ? !presetOptions.map((f) => f.format).includes(dateFormat) : false}
					>
						{#each presetOptions as f}
							<option value={f.format}>{f.label}</option>
						{/each}
					</select>
				</Label>
			</div>
		{/if}
	{:else if kind == 'none'}
		{#if !noExtra}
			<div class="mt-2"></div>
			<Label label="Min textarea rows">
				<TextInput
					inputProps={{ type: 'number' }}
					bind:value={() => minRows?.toString(), (v) => (minRows = v ? parseInt(v) : undefined)}
				/>
			</Label>
		{/if}
	{:else if kind === 'base64'}
		<Alert
			type="warning"
			title="S3 Object recommended"
			size="xs"
			tooltip="Check out the documentation for more information:"
			documentationLink="https://www.windmill.dev/docs/core_concepts/persistent_storage#large-data-files-s3-r2-minio-azure-blob"
		>
			For large files, we recommend using the S3 Object type instead of the base64 string type.
		</Alert>
	{/if}
	{#if (kind == 'none' || kind == 'pattern' || kind == 'format') && !noExtra}
		<Toggle
			size="xs"
			options={{ right: 'Disable variable picker' }}
			checked={disableVariablePicker}
			on:change={(e) => {
				if (e.detail) {
					disableVariablePicker = true
				} else {
					disableVariablePicker = undefined
				}
			}}
			disabled={['email', 'yaml', 'sql', 'date-time', 'date'].includes(format ?? '')}
		/>
	{/if}

	{#if kind == 'none' || kind == 'pattern' || kind == 'format'}
		<Toggle
			size="xs"
			options={{
				right: 'Is Password/Sensitive',
				rightTooltip:
					'The value will be stored as an ephemeral secret variable in the user space of the caller of the job, only viewable by him.'
			}}
			checked={password}
			on:change={(e) => {
				if (e.detail) {
					password = true
				} else {
					password = undefined
				}
			}}
		/>
	{/if}
</div>
