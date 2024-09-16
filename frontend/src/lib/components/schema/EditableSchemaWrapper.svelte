<script lang="ts">
	import type { Schema } from '$lib/common'
	import { twMerge } from 'tailwind-merge'
	import EditableSchemaForm from '../EditableSchemaForm.svelte'
	import AddProperty from './AddProperty.svelte'
	import { createEventDispatcher } from 'svelte'
	import Toggle from '../Toggle.svelte'
	import { emptySchema } from '$lib/utils'

	export let schema: Schema | undefined | any
	export let offset: number = 0
	export let uiOnly: boolean = false
	export let noPreview: boolean = false
	export let fullHeight: boolean = true
	export let lightweightMode: boolean = false
	export let formatExtension: string | undefined = undefined

	let resourceIsTextFile: boolean = false
	let addProperty: AddProperty | undefined = undefined

	const dispatch = createEventDispatcher()

	$: !resourceIsTextFile && (formatExtension = undefined)


	function switchResourceIsFile() {
		if (!resourceIsTextFile) {
			schema = emptySchema()
			formatExtension = undefined
		} else  {

			schema = emptySchema()
			schema.order = ["content"]
			schema.properties = {
				content: {
					type: "string",
					description: "Text contents of the file"
				}
			}


		}
		dispatch('change', schema)
	}
</script>

<div class="w-3/12">
	<Toggle
		bind:checked={resourceIsTextFile}
		options={{ right: 'This resource type is a text file' }}
		on:change={() => switchResourceIsFile()}
	/>
	{#if resourceIsTextFile}
		<input
			bind:value={formatExtension}
			placeholder="File format (e.g `.json`)"
			on:keydown={(event) => {
				if (event.key === 'Enter') {
				}
			}}
		/>
	{/if}
</div>
{#if !resourceIsTextFile}
	<div class={twMerge(fullHeight ? 'h-full' : 'h-80', 'border overflow-y-auto rounded-md')}>
		<div class="p-4 border-b">
			<AddProperty
				on:change={() => dispatch('change', schema)}
				bind:schema
				bind:this={addProperty}
			/>
		</div>
		<EditableSchemaForm
			bind:schema
			on:change={() => dispatch('change', schema)}
			isFlowInput
			on:edit={(e) => {
				addProperty?.openDrawer(e.detail)
			}}
			on:delete={(e) => {
				addProperty?.handleDeleteArgument([e.detail])
			}}
			{offset}
			{uiOnly}
			{noPreview}
			{lightweightMode}
		/>
	</div>
{/if}
