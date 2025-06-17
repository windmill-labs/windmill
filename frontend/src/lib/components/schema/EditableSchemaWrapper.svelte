<script lang="ts">
	import type { Schema } from '$lib/common'
	import { twMerge } from 'tailwind-merge'
	import EditableSchemaForm from '../EditableSchemaForm.svelte'
	import { createEventDispatcher } from 'svelte'
	import Toggle from '../Toggle.svelte'
	import { emptySchema, validateFileExtension } from '$lib/utils'
	import { Alert } from '../common'
	import AddPropertyV2 from '$lib/components/schema/AddPropertyV2.svelte'
	import { Plus } from 'lucide-svelte'
	import Select from '../select/Select.svelte'

	interface Props {
		schema: Schema | undefined | any
		uiOnly?: boolean
		noPreview?: boolean
		fullHeight?: boolean
		formatExtension?: string | undefined
	}

	let {
		schema = $bindable(),
		uiOnly = false,
		noPreview = false,
		fullHeight = true,
		formatExtension = $bindable(undefined)
	}: Props = $props()

	let resourceIsTextFile: boolean = $state(false)
	let addPropertyComponent: AddPropertyV2 | undefined = $state(undefined)
	let editableSchemaForm: EditableSchemaForm | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	$effect(() => {
		if (!resourceIsTextFile && formatExtension !== undefined) {
			formatExtension = undefined
		}
	})

	let invalidExtension = $derived(
		formatExtension && formatExtension != ''
			? !validateFileExtension(formatExtension ?? 'txt')
			: false
	)

	function switchResourceIsFile() {
		if (!resourceIsTextFile) {
			schema = emptySchema()
			formatExtension = undefined
		} else {
			formatExtension = ''
			schema = emptySchema()
			schema.order = ['content']
			schema.properties = {
				content: {
					type: 'string',
					description: 'Text contents of the file'
				}
			}
		}
		dispatch('change', schema)
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

{#if !resourceIsTextFile}
	<div
		class={twMerge(
			fullHeight ? 'h-full' : 'h-80',
			noPreview ? '' : 'border rounded-md p-2',
			'overflow-y-auto flex flex-col gap-2'
		)}
	>
		{#if noPreview}
			<AddPropertyV2
				bind:schema
				bind:this={addPropertyComponent}
				on:change={() => dispatch('change', schema)}
				on:addNew={(e) => {
					editableSchemaForm?.openField(e.detail)
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
			on:change={() => dispatch('change', schema)}
			isFlowInput
			on:edit={(e) => {
				addPropertyComponent?.openDrawer(e.detail)
			}}
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
						bind:schema
						bind:this={addPropertyComponent}
						on:change={() => dispatch('change', schema)}
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
{#if resourceIsTextFile}
	<label
		for="format-extension"
		class="text-base font-medium whitespace-nowrap flex items-center gap-4"
	>
		File extension :
		<Select
			autofocus
			items={suggestedFileExtensions.map((e) => ({ value: e, label: e }))}
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
			The <span class="font-bold font-mono"> .{formatExtension} </span> extension will be used to infer
			the format when displaying the content and this is also how the resource will appear when pulling
			via the CLI.
		</Alert>
		<div></div>
	{/if}
{/if}
<Toggle
	bind:checked={resourceIsTextFile}
	options={{
		right: 'This resource type represents a plain text file (clears current schema)',
		rightTooltip:
			'A text file such as a config file, template, or any other file format that contains plain text'
	}}
	on:change={() => switchResourceIsFile()}
/>
