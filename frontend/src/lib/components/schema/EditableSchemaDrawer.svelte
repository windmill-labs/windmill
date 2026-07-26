<script lang="ts">
	import EditableSchemaDrawer from './EditableSchemaDrawer.svelte'
	import type { Schema } from '$lib/common'
	import { GripVertical, Pen, Trash, Plus } from 'lucide-svelte'
	import EditableSchemaForm from '../EditableSchemaForm.svelte'
	import { Drawer, DrawerContent } from '../common'
	import AddProperty from './AddProperty.svelte'
	import { dragHandle, dragHandleZone } from '@windmill-labs/svelte-dnd-action'
	import { flip } from 'svelte/animate'
	import { emptyString, generateRandomString } from '$lib/utils'
	import Button from '$lib/components/common/button/Button.svelte'
	import { createEventDispatcher, tick } from 'svelte'
	import Label from '../Label.svelte'
	import Toggle from '../Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import AddPropertyV2 from '$lib/components/schema/AddPropertyV2.svelte'

	const flipDurationMs = 200

	const dispatch = createEventDispatcher()

	let addPropertyComponent: AddProperty | undefined = $state(undefined)
	let schemaFormDrawer: Drawer | undefined = $state(undefined)
	let editableSchemaForm: EditableSchemaForm | undefined = $state(undefined)

	function handleConsider(e) {
		const { items: newItems } = e.detail

		items = newItems
	}

	function handleFinalize(e) {
		const { items: newItems } = e.detail

		items = newItems

		const keys = items.map((item) => item.value)

		schema = {
			...schema,
			properties: keys.reduce((acc, key) => {
				acc[key] = schema.properties[key]
				return acc
			}, {}),
			order: keys
		}

		tick().then(() => dispatch('change', schema))
	}

	interface Props {
		schema: Schema | undefined | any
		jsonView?: boolean
		hiddenArgs?: string[]
		onClose?: () => void
		isFlowInput?: boolean
		isAppInput?: boolean
	}

	let {
		schema = $bindable(),
		jsonView = $bindable(false),
		hiddenArgs = undefined,
		isFlowInput = false,
		isAppInput = false
	}: Props = $props()

	// let schema = $state(structuredClone($state.snapshot(schema)))

	let schemaString: string = $state(JSON.stringify(schema, null, '\t'))
	let editor: SimpleEditor | undefined = $state(undefined)
	let error: string | undefined = $state(undefined)
	let items = $derived([
		...new Set(
			(schema?.order ?? Object.keys(schema?.properties ?? {}))
				?.map((item, index) => {
					return { value: item, id: item }
				})
				.filter((item) => !hiddenArgs?.includes(item.value)) ?? []
		)
	]) as Array<{
		value: string
		id: string
	}>

	const rnd = generateRandomString()
</script>

<div class="flex flex-wrap justify-between mb-2 w-full items-center gap-y-2">
	<AddProperty
		on:change={() => {
			if (jsonView) {
				schemaString = JSON.stringify(schema, null, '\t')
				editor?.setCode(schemaString)
			}
		}}
		bind:schema
		bind:this={addPropertyComponent}
	/>

	<Toggle
		bind:checked={jsonView}
		size="xs"
		options={{
			right: 'JSON editor',
			rightTooltip:
				'Arguments can be edited either using the wizard, or by editing their JSON schema.'
		}}
		lightMode
		on:change={() => {
			schemaString = JSON.stringify(schema, null, '\t')
			editor?.setCode(schemaString)
		}}
	/>
</div>

{#if !jsonView}
	{#key rnd}
		<div
			use:dragHandleZone={{
				items,
				flipDurationMs,
				dropTargetStyle: {},
				type: rnd
			}}
			onconsider={handleConsider}
			onfinalize={handleFinalize}
			class="gap-1 flex flex-col mt-2"
		>
			{#if items?.length > 0}
				{#each items as item (item.id)}
					<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
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
								<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div class="flex flex-row gap-1 item-center h-full justify-center">
									<Button
										iconOnly
										size="xs2"
										color="light"
										startIcon={{ icon: Trash }}
										on:click={() => {
											addPropertyComponent?.handleDeleteArgument([item.value])
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

									<div class="flex items-center handle" use:dragHandle>
										<GripVertical size={16} />
									</div>
								</div>
							</div>

							{#if schema.properties[item.value]?.type === 'object' && !(schema.properties[item.value].oneOf && schema.properties[item.value].oneOf.length >= 2)}
								<div class="flex flex-col w-full mt-2">
									<Label label="Nested properties">
										<EditableSchemaDrawer
											bind:schema={schema.properties[item.value]}
											{isFlowInput}
											{isAppInput}
										/>
									</Label>
								</div>
							{/if}
						{:else}
							<div class="text-primary"> Value is undefined </div>
						{/if}
					</div>
				{/each}
			{/if}
		</div>
	{/key}
	<Drawer bind:this={schemaFormDrawer} size="1200px">
		{#snippet children()}
			<DrawerContent title="UI Customisation" on:close={() => schemaFormDrawer?.closeDrawer()}>
				<EditableSchemaForm
					schemaFormClassName="min-h-full"
					bind:this={editableSchemaForm}
					bind:schema
					{isFlowInput}
					{isAppInput}
					on:edit={(e) => {
						addPropertyComponent?.openDrawer(e.detail)
					}}
					on:delete={(e) => {
						addPropertyComponent?.handleDeleteArgument([e.detail])
					}}
					{hiddenArgs}
					editTab="inputEditor"
				>
					{#snippet addProperty()}
						<AddPropertyV2
							bind:schema
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
					{/snippet}
				</EditableSchemaForm>
			</DrawerContent>
		{/snippet}
	</Drawer>
{:else}
	<div class="mt-2 bg-surface-tertiary rounded-md border py-2.5">
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
	</div>
	{#if !emptyString(error)}
		<div class="text-red-400 text-xs">{error}</div>
	{:else}
		<div><br /> </div>
	{/if}
{/if}
