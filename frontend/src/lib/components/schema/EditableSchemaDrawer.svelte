<script lang="ts">
	import type { Schema } from '$lib/common'
	import { GripVertical, Pen, Trash, Plus } from 'lucide-svelte'
	import EditableSchemaForm from '../EditableSchemaForm.svelte'
	import { Drawer, DrawerContent } from '../common'
	import AddProperty from './AddProperty.svelte'
	import { dragHandle, dragHandleZone } from '@windmill-labs/svelte-dnd-action'
	import { flip } from 'svelte/animate'
	import { emptyString } from '$lib/utils'
	import Button from '$lib/components/common/button/Button.svelte'
	import { createEventDispatcher, tick } from 'svelte'
	import Label from '../Label.svelte'
	import Toggle from '../Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import AddPropertyV2 from '$lib/components/schema/AddPropertyV2.svelte'

	export let schema: Schema | undefined | any
	export let parentId: string | undefined = undefined

	const flipDurationMs = 200

	const dispatch = createEventDispatcher()

	let addProperty: AddProperty | undefined = undefined
	let schemaFormDrawer: Drawer | undefined = undefined
	let editableSchemaForm: EditableSchemaForm | undefined = undefined

	$: items = ((schema?.order ?? Object.keys(schema?.properties ?? {}))?.map((item, index) => {
		return { value: item, id: item }
	}) ?? []) as Array<{
		value: string
		id: string
	}>

	function handleConsider(e) {
		const { items: newItems } = e.detail

		items = newItems
	}

	function handleFinalize(e) {
		const { items: newItems } = e.detail

		items = newItems

		const keys = items.map((item) => item.value)

		schema.properties = keys.reduce((acc, key) => {
			acc[key] = schema.properties[key]
			return acc
		}, {})

		schema.order = keys

		schema = { ...schema }
		tick().then(() => dispatch('change', schema))
	}

	export let jsonView: boolean = false
	let schemaString: string = JSON.stringify(schema, null, '\t')
	let editor: SimpleEditor | undefined = undefined
	let error: string | undefined = undefined
</script>

<div class="flex flex-col items-end mb-2 w-full">
	<Toggle
		bind:checked={jsonView}
		label="JSON View"
		size="xs"
		options={{
			right: 'JSON editor',
			rightTooltip:
				'Arguments can be edited either using the wizard, or by editing their JSON Schema.'
		}}
		lightMode
		on:change={() => {
			schemaString = JSON.stringify(schema, null, '\t')
			editor?.setCode(schemaString)
		}}
	/>
</div>

<AddProperty
	on:change={() => {
		if (jsonView) {
			schemaString = JSON.stringify(schema, null, '\t')
			editor?.setCode(schemaString)
		}
		// TODO: find out why this work.
		// Solves a bug where adding a new field in a OneOf input in a flow did not do anything
		//
		// It seemed that the schema was correctly changed but then reset higher in the component
		// tree due to the change event, as removing the dispatched removed the bug.
		// I already spent way too long trying to figure out the chain of effects
		setTimeout(() => {
			dispatch('change', schema)
		}, 0.5)
	}}
	bind:schema
	bind:this={addProperty}
/>

{#if !jsonView}
	<div
		use:dragHandleZone={{
			items,
			flipDurationMs,
			dropTargetStyle: {},
			type: parentId ? `app-editor-fields-${parentId}` : 'app-editor-fields'
		}}
		on:consider={handleConsider}
		on:finalize={handleFinalize}
		class="gap-1 flex flex-col mt-2"
	>
		{#if items?.length > 0}
			{#each items as item (item.id)}
				<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
				<div
					animate:flip={{ duration: 200 }}
					class="w-full flex flex-col justify-between border items-center py-1 px-2 rounded-md bg-surface text-sm"
				>
					{#if schema.properties?.[item.value]}
						<div class="flex flex-row justify-between items-center w-full">
							{`${item.value}${
								schema.properties?.[item.value]?.title
									? ` (title: ${schema.properties?.[item.value]?.title})`
									: ''
							} `}
							<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
							<!-- svelte-ignore a11y-no-static-element-interactions -->
							<div class="flex flex-row gap-1 item-center h-full justify-center">
								<Button
									iconOnly
									size="xs2"
									color="light"
									startIcon={{ icon: Trash }}
									on:click={() => {
										addProperty?.handleDeleteArgument([item.value])
									}}
								/>
								<Button
									iconOnly
									size="xs2"
									color="light"
									startIcon={{ icon: Pen }}
									on:click={() => {
										schemaFormDrawer?.openDrawer()

										tick().then(() => {
											editableSchemaForm?.openField(item.value)
										})
									}}
								/>

								<div class="cursor-move flex items-center handle" use:dragHandle>
									<GripVertical size={16} />
								</div>
							</div>
						</div>

						{#if schema.properties[item.value]?.type === 'object' && !(schema.properties[item.value].oneOf && schema.properties[item.value].oneOf.length >= 2)}
							<div class="flex flex-col w-full mt-2">
								<Label label="Nested properties">
									<svelte:self
										on:change={() => {
											schema = schema
											dispatch('change', schema)
										}}
										bind:schema={schema.properties[item.value]}
										parentId={item.value}
									/>
								</Label>
							</div>
						{/if}
					{:else}
						<div class="text-tertiary"> Value is undefined </div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>

	<Drawer bind:this={schemaFormDrawer} size="1200px">
		<DrawerContent title="UI Customisation" on:close={schemaFormDrawer.closeDrawer}>
			<EditableSchemaForm
				on:change
				bind:this={editableSchemaForm}
				bind:schema
				isAppInput
				on:edit={(e) => {
					addProperty?.openDrawer(e.detail)
				}}
				on:delete={(e) => {
					addProperty?.handleDeleteArgument([e.detail])
				}}
				dndType="drawer"
				editTab="inputEditor"
			>
				<svelte:fragment slot="addProperty">
					<AddPropertyV2
						bind:schema
						on:change
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
				</svelte:fragment>
			</EditableSchemaForm>
		</DrawerContent>
	</Drawer>
{:else}
	<div class="mt-2"></div>
	<SimpleEditor
		bind:this={editor}
		small
		fixedOverflowWidgets={false}
		on:change={() => {
			try {
				schema = JSON.parse(schemaString)
				error = ''
			} catch (err) {
				error = err.message
			}
		}}
		bind:code={schemaString}
		lang="json"
		autoHeight
		automaticLayout
	/>
	{#if !emptyString(error)}
		<div class="text-red-400 text-xs">{error}</div>
	{:else}
		<div><br /> </div>
	{/if}
{/if}
