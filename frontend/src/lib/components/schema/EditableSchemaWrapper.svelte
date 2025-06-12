<script lang="ts">
	import type { Schema } from '$lib/common'
	import { twMerge } from 'tailwind-merge'
	import EditableSchemaForm from '../EditableSchemaForm.svelte'
	import { createEventDispatcher } from 'svelte'
	import Toggle from '../Toggle.svelte'
	import { emptySchema, validateFileExtension } from '$lib/utils'
	import AutoComplete from 'simple-svelte-autocomplete'
	import { Alert } from '../common'
	import AddPropertyV2 from '$lib/components/schema/AddPropertyV2.svelte'
	import { Plus } from 'lucide-svelte'

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

	function numberOfMatches(listItem: string | undefined, searchWords: string[]): number {
		if (!listItem) {
			return 0
		}

		let matches = 0
		searchWords.forEach((searchWord) => {
			const searchLetters = searchWord.split('')
			if (searchLetters.every((l) => listItem.includes(l))) {
				matches++
			}
		})
		return matches
	}

	let suggestedFileExtensions = [
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
	]
	let autocompleteExtension = $state(true)
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
	<div class="flex items-center space-x-2 w-5/12">
		<label for="format-extension" class="text-base font-medium whitespace-nowrap">
			File extension{autocompleteExtension ? '' : ' (free text)'}:
		</label>
		{#if autocompleteExtension}
			<AutoComplete
				inputId="format-extension"
				autofocus={true}
				items={[...suggestedFileExtensions, 'Choose another extension']}
				onChange={(a) => {
					if (a == 'Choose another extension') {
						formatExtension = ''
						autocompleteExtension = false
					}
				}}
				itemFilterFunction={(listItem, searchWords) => {
					if (searchWords.length == 0 || listItem === 'Choose another extension') {
						return true
					}
					return numberOfMatches(listItem, searchWords) > 0
				}}
				noResultsText="No matches, try the 'Choose another extension' option"
				bind:selectedItem={formatExtension}
				inputClassName="!h-[32px] py-1 !text-xs !w-64"
				hideArrow
				className={'!font-bold'}
				dropdownClassName="!font-normal !w-64 !max-w-64"
				maxItemsToShowInList={8}
				moreItemsText={null}
				lock={true}
			/>
		{:else}
			<!-- svelte-ignore a11y_autofocus -->
			<input
				autofocus={true}
				bind:value={formatExtension}
				class="!h-[32px] py-1 !text-xs !w-64"
				placeholder="Enter your extension"
				onkeydown={(event) => {
					if (event.key === 'Enter') {
						if (formatExtension && !suggestedFileExtensions.includes(formatExtension))
							suggestedFileExtensions.push(formatExtension)

						autocompleteExtension = true
					}
				}}
			/>
		{/if}
	</div>

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
