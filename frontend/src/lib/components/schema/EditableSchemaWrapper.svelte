<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import EditableSchemaForm from '../EditableSchemaForm.svelte'
	import { emptySchema, validateFileExtension } from '$lib/utils'
	import { Alert } from '../common'
	import AddPropertyV2 from '$lib/components/schema/AddPropertyV2.svelte'
	import { Plus } from 'lucide-svelte'
	import Select from '../select/Select.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import type { EditableSchemaWrapperProps } from './editable_schema_wrapper'

	type ResourceMode = 'schema' | 'file' | 'fileset'

	let {
		schema = $bindable(),
		uiOnly = false,
		noPreview = false,
		fullHeight = true,
		formatExtension = $bindable(undefined),
		isFileset = $bindable(undefined),
		customUi
	}: EditableSchemaWrapperProps = $props()

	let resourceMode: ResourceMode = $state('schema')
	let addPropertyComponent: AddPropertyV2 | undefined = $state(undefined)
	let editableSchemaForm: EditableSchemaForm | undefined = $state(undefined)

	$effect(() => {
		if (resourceMode === 'schema') {
			if (formatExtension !== undefined) {
				formatExtension = undefined
			}
			if (isFileset) {
				isFileset = undefined
			}
		}
	})

	let invalidExtension = $derived(
		formatExtension && formatExtension != ''
			? !validateFileExtension(formatExtension ?? 'txt')
			: false
	)

	function switchResourceMode(mode: ResourceMode) {
		resourceMode = mode
		if (mode === 'schema') {
			schema = emptySchema()
			formatExtension = undefined
			isFileset = undefined
		} else if (mode === 'file') {
			formatExtension = ''
			isFileset = undefined
			schema = emptySchema()
			schema.order = ['content']
			schema.properties = {
				content: {
					type: 'string',
					description: 'Text contents of the file'
				}
			}
		} else if (mode === 'fileset') {
			formatExtension = undefined
			isFileset = true
			schema = emptySchema()
		}
	}

	let suggestedFileExtensions = $state([
		'json',
		'yaml',
		'jinja',
		'j2',
		'ini',
		'cfg',
		'toml',
		'html',
		'xml',
		'yml'
	])
</script>

{#if resourceMode === 'schema'}
	<div
		class={twMerge(
			fullHeight ? 'h-full' : 'h-80',
			noPreview ? '' : 'border rounded-md p-2',
			'overflow-y-auto flex flex-col gap-2'
		)}
	>
		{#if noPreview}
			<AddPropertyV2
				noPopover={customUi?.noAddPopover}
				bind:schema
				bind:this={addPropertyComponent}
				onAddNew={(argName) => {
					editableSchemaForm?.openField(argName)
				}}
			>
				{#snippet trigger()}
					<div
						class="w-full py-2 flex justify-center items-center border border-dashed rounded-md hover:bg-surface-hover"
					>
						<Plus size={14} />
					</div>
				{/snippet}
			</AddPropertyV2>
		{/if}
		<EditableSchemaForm
			onlyMaskPassword
			bind:this={editableSchemaForm}
			bind:schema
			isFlowInput
			on:delete={(e) => {
				addPropertyComponent?.handleDeleteArgument([e.detail])
			}}
			{uiOnly}
			{noPreview}
			editTab="inputEditor"
		>
			{#snippet addProperty()}
				{#if !noPreview}
					<AddPropertyV2
						noPopover={customUi?.noAddPopover}
						bind:schema
						bind:this={addPropertyComponent}
					>
						{#snippet trigger()}
							<div
								class="w-full py-2 flex justify-center items-center border border-dashed rounded-md hover:bg-surface-hover"
							>
								<Plus size={14} />
							</div>
						{/snippet}
					</AddPropertyV2>
				{/if}
			{/snippet}
		</EditableSchemaForm>
	</div>
{/if}
{#if resourceMode === 'file'}
	<label
		for="format-extension"
		class="text-xs font-semibold text-emphasis whitespace-nowrap flex items-center gap-4"
	>
		File extension:
		<Select
			autofocus
			items={safeSelectItems(suggestedFileExtensions)}
			bind:value={formatExtension}
			onCreateItem={(ext) => ((formatExtension = ext), suggestedFileExtensions.push(ext))}
		/>
	</label>

	{#if invalidExtension}
		<Alert title="Invalid file extension" type="error">
			The provided extension (<span class="font-bold font-mono">.{formatExtension}</span>) contains
			invalid characters. Note that you shouldn't add the leading dot, (i.e. enter `json` instead of
			`.json`)
		</Alert>
	{:else if formatExtension && formatExtension !== ''}
		<Alert title={`Example: my_file.${formatExtension}`} type="info">
			The <span class="font-bold font-mono"> .{formatExtension} </span> extension will be used to
			infer the format when displaying the content and this is also how the resource will appear
			when pulling via the CLI.
		</Alert>
		<div></div>
	{/if}
{/if}
{#if resourceMode === 'fileset'}
	<Alert title="Fileset resource type" type="info">
		This resource type represents a collection of files. Each file is identified by its relative
		path and contains text content. In the CLI, filesets are stored as directories.
	</Alert>
{/if}
<ToggleButtonGroup
	selected={resourceMode}
	onSelected={(mode) => switchResourceMode(mode)}
>
	{#snippet children({ item })}
		<ToggleButton value="schema" label="JSON" {item} size="sm" />
		<ToggleButton value="file" label="File" {item} size="sm" />
		<ToggleButton value="fileset" label="Fileset" {item} size="sm" />
	{/snippet}
</ToggleButtonGroup>
