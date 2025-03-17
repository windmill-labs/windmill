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

	export let schema: Schema | undefined | any
	export let uiOnly: boolean = false
	export let noPreview: boolean = false
	export let fullHeight: boolean = true
	export let formatExtension: string | undefined = undefined

	let resourceIsTextFile: boolean = false
	let addProperty: AddPropertyV2 | undefined = undefined
	let editableSchemaForm: EditableSchemaForm | undefined = undefined

	const dispatch = createEventDispatcher()

	$: !resourceIsTextFile && (formatExtension = undefined)

	$: invalidExtension =
		formatExtension && formatExtension != ''
			? !validateFileExtension(formatExtension ?? 'txt')
			: false

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
	let autocompleteExtension = true
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
				bind:this={addProperty}
				on:change={() => dispatch('change', schema)}
				on:addNew={(e) => {
					editableSchemaForm?.openField(e.detail)
				}}
			>
				<svelte:fragment slot="trigger">
					<div
						class="w-full py-2 flex justify-center items-center border border-dashed rounded-md hover:bg-surface-hover"
					>
						<Plus size={14} />
					</div>
				</svelte:fragment>
			</AddPropertyV2>
		{/if}
		<EditableSchemaForm
			onlyMaskPassword
			bind:this={editableSchemaForm}
			bind:schema
			on:change={() => dispatch('change', schema)}
			isFlowInput
			on:edit={(e) => {
				addProperty?.openDrawer(e.detail)
			}}
			on:delete={(e) => {
				addProperty?.handleDeleteArgument([e.detail])
			}}
			{uiOnly}
			{noPreview}
			editTab="inputEditor"
		>
			<svelte:fragment slot="addProperty">
				{#if !noPreview}
					<AddPropertyV2
						bind:schema
						bind:this={addProperty}
						on:change={() => dispatch('change', schema)}
					>
						<svelte:fragment slot="trigger">
							<div
								class="w-full py-2 flex justify-center items-center border border-dashed rounded-md hover:bg-surface-hover"
							>
								<Plus size={14} />
							</div>
						</svelte:fragment>
					</AddPropertyV2>
				{/if}
			</svelte:fragment>
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
			<!-- svelte-ignore a11y-autofocus -->
			<input
				autofocus={true}
				bind:value={formatExtension}
				class="!h-[32px] py-1 !text-xs !w-64"
				placeholder="Enter your extension"
				on:keydown={(event) => {
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
