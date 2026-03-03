<script lang="ts">
	import { run } from 'svelte/legacy'

	import { addWhitespaceBeforeCapitals, capitalize, classNames } from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Loader2, Pen, User, Shield } from 'lucide-svelte'

	import Toggle from '$lib/components/Toggle.svelte'
	import StaticInputEditor from '../apps/editor/settingsPanel/inputEditor/StaticInputEditor.svelte'
	import { fieldTypeToTsType } from '../apps/utils'
	import type { InputType } from '../apps/inputType'
	import Select from '$lib/components/select/Select.svelte'
	import { userStore, workspaceStore } from '$lib/stores'

	// Build ctx properties with current user's actual values
	let ctxProperties = $derived([
		{ value: 'username', label: 'Username', subtitle: `string — "${$userStore?.username ?? 'unknown'}"` },
		{ value: 'email', label: 'Email', subtitle: `string — "${$userStore?.email ?? 'unknown'}"` },
		{ value: 'groups', label: 'Groups', subtitle: `string[] — ${JSON.stringify($userStore?.groups ?? [])}` },
		{ value: 'workspace', label: 'Workspace', subtitle: `string — "${$workspaceStore ?? 'unknown'}"` },
		{ value: 'author', label: 'Author', subtitle: `string — "${$userStore?.email ?? 'unknown'}"` }
	])

	// Raw app input type that includes ctx inputs
	// Using a looser type to avoid TypeScript narrowing issues in Svelte templates
	// The 'any' binding allows compatibility with both StaticInputEditor and raw app field types
	type RawAppInput = any

	interface Props {
		id: string
		componentInput: RawAppInput
		key: string
		shouldCapitalize?: boolean
		resourceOnly?: boolean
		tooltip?: string | undefined
		fieldType: InputType
		subFieldType: InputType | undefined
		format: string | undefined
		selectOptions: string[] | undefined
		placeholder: string | undefined
		customTitle?: string | undefined
		displayType?: boolean
		allowTypeChange?: boolean
		loading?: boolean
		documentationLink?: string | undefined
	}

	let {
		id,
		componentInput = $bindable(),
		key,
		shouldCapitalize = true,
		resourceOnly = false,
		tooltip = undefined,
		fieldType,
		subFieldType,
		format,
		selectOptions,
		placeholder,
		customTitle = undefined,
		displayType = false,
		allowTypeChange = true,
		loading = false,
		documentationLink = undefined
	}: Props = $props()

	run(() => {
		if (componentInput == undefined) {
			//@ts-ignore
			componentInput = {
				type: 'user',
				fieldType
			}
		} else if (componentInput.fieldType == undefined && fieldType) {
			// Ensure fieldType is set on existing inputs
			componentInput = { ...componentInput, fieldType }
		}
	})
</script>

{#if !(resourceOnly && (fieldType !== 'object' || !format?.startsWith('resource-')))}
	<div class={classNames('flex gap-1', 'flex-col')}>
		<div class="flex justify-between items-end">
			<div class="flex flex-row gap-4 items-center">
				<div class="flex items-center">
					<span class="!text-2xs font-semibold text-ellipsis text-primary">
						{customTitle
							? customTitle
							: shouldCapitalize
								? capitalize(addWhitespaceBeforeCapitals(key))
								: key}
					</span>
					{#if loading}
						<Loader2 size={14} class="animate-spin ml-2" />
					{/if}
					{#if tooltip}
						<Tooltip small {documentationLink}>
							{@html tooltip}
						</Tooltip>
					{/if}
				</div>
				{#if displayType}
					<div class="text-xs text-primary mr-1">
						{fieldType === 'array' && subFieldType
							? `${fieldTypeToTsType(subFieldType)}[]`
							: fieldTypeToTsType(fieldType)}
					</div>
				{/if}
			</div>

			<div class={classNames('flex gap-x-2 gap-y-1 justify-end items-center')}>
				{#if componentInput?.type && allowTypeChange !== false}
					<ToggleButtonGroup
						selected={componentInput.type}
						onSelected={(newType) => {
							// Preserve fieldType and other properties when changing type
							componentInput = {
								...componentInput,
								type: newType,
								fieldType: componentInput.fieldType ?? fieldType
							}
							// Clear ctx when switching away from ctx type
							if (newType !== 'ctx') {
								delete componentInput.ctx
							}
						}}
					>
						{#snippet children({ item })}
							<ToggleButton {item} value="user" icon={User} iconOnly tooltip="User Input" />
							<ToggleButton {item} value="static" icon={Pen} iconOnly tooltip="Static" />
							<ToggleButton {item} value="ctx" icon={Shield} iconOnly tooltip="Context (secure backend value)" />
						{/snippet}
					</ToggleButtonGroup>
				{/if}
			</div>
		</div>

		{#if componentInput?.type === 'static'}
			<div class={'w-full flex flex-row-reverse'}>
				<StaticInputEditor
					{id}
					{fieldType}
					{subFieldType}
					{selectOptions}
					{format}
					{placeholder}
					bind:componentInput
				/>
			</div>
		{:else if componentInput?.type === 'ctx'}
			<div class="w-full">
				<Select
					items={ctxProperties}
					bind:value={componentInput.ctx}
					placeholder="Select context property"
				/>
				{#if componentInput.ctx}
					<span class="text-2xs italic text-tertiary mt-1 flex items-center gap-1">
						<Shield size={12} />
						Securely resolved by the backend (cannot be altered by users)
					</span>
				{/if}
			</div>
		{:else if componentInput?.type === 'user' || componentInput?.type == undefined}
			<span class="text-2xs italic text-primary">Field's value is a frontend input</span>
		{/if}
		{#if componentInput?.type === 'user' && ((fieldType == 'object' && format?.startsWith('resource-') && format !== 'resource-s3_object') || fieldType == 'resource')}
			<div class="flex flex-row items-center">
				<Toggle
					size="xs"
					bind:checked={componentInput.allowUserResources}
					options={{
						left: 'static resource select only',
						right: 'resources from users allowed'
					}}
				/>
				<Tooltip
					>Apps are executed on behalf of publishers. If you want to accept resources from user, you
					need to enable this (potentially dangerous!)</Tooltip
				>
			</div>
		{/if}
	</div>
{/if}
