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

	export let pattern: string | undefined
	export let enum_: EnumType
	export let format: string | undefined
	export let contentEncoding: 'base64' | 'binary' | undefined
	export let customErrorMessage: string | undefined
	export let minRows: number | undefined = undefined
	export let disableCreate: boolean | undefined = undefined
	export let disableVariablePicker: boolean | undefined = undefined
	export let password: boolean | undefined = undefined
	export let noExtra = false
	export let dateFormat: string | undefined
	export let enumLabels: Record<string, string> | undefined = undefined
	export let overrideAllowKindChange: boolean = true
	export let originalType: string | undefined = undefined

	let kind: 'none' | 'pattern' | 'enum' | 'resource' | 'format' | 'base64' | 'date-time' =
		computeKind(enum_, contentEncoding, pattern, format)

	const allowKindChange = overrideAllowKindChange || originalType === 'string'

	let patternStr: string = pattern ?? ''
	let resource: string | undefined

	const FORMATS = [
		'email',
		'hostname',
		'uri',
		'uuid',
		'ipv4',
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

	$: format =
		kind == 'resource' ? (resource != undefined ? `resource-${resource}` : 'resource') : format
	$: pattern = patternStr == '' ? undefined : patternStr
	$: contentEncoding = kind == 'base64' ? 'base64' : undefined

	$: {
		if (format == 'email') {
			pattern = '^[\\w-+.]+@([\\w-]+\\.)+[\\w-]{2,63}$'
		}
	}

	function add() {
		if (enumLabels === undefined) {
			enumLabels = {}
		}

		let choice = `choice ${enum_?.length ? enum_?.length + 1 : 1}`
		enum_ = enum_ ? enum_.concat(choice) : [choice]
	}

	function remove(item: string) {
		enum_ = (enum_ || []).filter((el) => el !== item)
		if (enum_.length == 0) {
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
			let:item
		>
			{#each FIELD_SETTINGS as x}
				<ToggleButton
					value={x[1]}
					label={x[0]}
					tooltip={x[2]}
					showTooltipIcon={Boolean(x[2])}
					{item}
				/>
			{/each}
		</ToggleButtonGroup>
	{/if}
	{#if kind == 'pattern'}
		<Label label="Pattern (Regex)">
			<svelte:fragment slot="header">
				<Tooltip light>
					Setting a pattern will allow you to specify a regular expression that the input should
					adhere to. You can use the regex generator to help you create a pattern.
				</Tooltip>
			</svelte:fragment>
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
			<svelte:fragment slot="header">
				<Tooltip light>
					Setting a custom error message will allow you to specify a message that will be shown when
					the input does not match the pattern.
				</Tooltip>
			</svelte:fragment>
			<svelte:fragment slot="action">
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
			</svelte:fragment>
			<input
				type="text"
				bind:value={customErrorMessage}
				disabled={customErrorMessage == undefined || customErrorMessage == ''}
			/>
		</Label>
	{:else if kind == 'enum'}
		<Label label="Enums">
			<svelte:fragment slot="header">
				<Tooltip light>
					Setting enums will allow you to specify a list of values that the input can take. If you
					want to allow custom values, you can disable the option below.
				</Tooltip>
			</svelte:fragment>
			<div class="flex flex-col gap-1">
				{#each enum_ || [] as e}
					<div class="flex flex-row w-full gap-2 pt-2">
						<input
							id="input"
							type="text"
							bind:value={e}
							on:input={(event) => onEnumKeyChange(event?.currentTarget.value, e)}
						/>
						{#if enumLabels !== undefined}
							<input
								id="input"
								type="text"
								bind:value={enumLabels[e]}
								placeholder="Optional title..."
								on:input={(event) => {
									if (event?.currentTarget.value === '') {
										if (enumLabels === undefined) {
											enumLabels = {}
										}
										delete enumLabels[e]
									}
								}}
							/>
						{/if}

						{#if allowKindChange}
							<Button size="sm" on:click={() => remove(e)}>-</Button>
						{/if}
					</div>
				{/each}
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
			<svelte:fragment slot="header">
				<Tooltip light>
					Setting the format will allow you to specify a format that the input should adhere to.
				</Tooltip>
			</svelte:fragment>
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
					<svelte:fragment slot="header">
						<Tooltip light>
							Setting the date output format will allow you to specify how the date will be passed
							to the script.
						</Tooltip>
					</svelte:fragment>
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
			<Label label="Min textarea rows">
				<input type="number" bind:value={minRows} />
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

	{#if kind == 'none' || kind == 'pattern'}
		<Toggle
			size="xs"
			options={{ right: 'Is Password' }}
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
