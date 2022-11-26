<script lang="ts">
	import { type Resource, ResourceService, type ResourceType, VariableService } from '$lib/gen'
	import { allTrue, emptySchema, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import type { Schema } from '$lib/common'
	import Path from './Path.svelte'
	import ArgInput from './ArgInput.svelte'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import Required from './Required.svelte'
	import { Button, Drawer } from './common'

	import { workspaceStore } from '$lib/stores'
	import ResourceTypePicker from './ResourceTypePicker.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import autosize from 'svelte-autosize'
	import SimpleEditor from './SimpleEditor.svelte'
	import AppConnect from './AppConnect.svelte'

	let path = ''
	let initialPath = ''
	let pathError = ''

	let step = 1

	let resourceToEdit: Resource | undefined

	let description: string = ''
	let DESCRIPTION_PLACEHOLDER = `You can use markdown to style your description`
	let selectedResourceType: string | undefined
	let resourceType: ResourceType
	let resourceSchema: Schema | undefined
	let args: Record<string, any> = {}

	let error: string | undefined

	let pickForField: string | undefined
	let itemPicker: ItemPicker
	let variableEditor: VariableEditor
	let drawer: Drawer

	let rawCode: string | undefined = undefined

	const dispatch = createEventDispatcher()

	export async function initNew(rt?: string) {
		step = 1
		args = {}
		path = ''
		description = ''
		initialPath = ''
		resourceSchema = emptySchema()
		resourceToEdit = undefined
		selectedResourceType = rt
		drawer.openDrawer()
	}

	export async function initEdit(p: string): Promise<void> {
		initialPath = p
		path = p
		step = 2
		resourceToEdit = await ResourceService.getResource({ workspace: $workspaceStore!, path: p })
		description = resourceToEdit!.description ?? ''
		selectedResourceType = resourceToEdit!.resource_type
		args = resourceToEdit!.value
		await loadResourceType()
		drawer.openDrawer()
	}

	async function createResource(): Promise<void> {
		await ResourceService.createResource({
			workspace: $workspaceStore!,
			requestBody: { path, value: args, description, resource_type: resourceType.name }
		})
		sendUserToast(`Successfully created resource at ${path}`)

		dispatch('refresh')
		drawer.closeDrawer()
	}

	async function editResource(): Promise<void> {
		try {
			if (resourceToEdit) {
				if (rawCode != undefined) {
					try {
						args = JSON.parse(rawCode)
					} catch (e) {
						sendUserToast("Couldn't parse the content as JSON", true)
						return
					}
				}
				await ResourceService.updateResource({
					workspace: $workspaceStore!,
					path: resourceToEdit.path,
					requestBody: { path, value: args, description }
				})
				sendUserToast(`Successfully updated resource at ${path}`)
				dispatch('refresh')
				drawer.closeDrawer()
			} else {
				throw Error('Cannot edit undefined resourceToEdit')
			}
		} catch (err) {
			sendUserToast(`${err}`, true)
		}
	}

	async function loadResourceType(): Promise<void> {
		if (selectedResourceType) {
			try {
				resourceType = await ResourceService.getResourceType({
					workspace: $workspaceStore!,
					path: selectedResourceType
				})

				if (resourceType.schema) {
					resourceSchema = resourceType.schema as Schema
				}
			} catch (err) {
				resourceSchema = undefined
				rawCode = JSON.stringify(args, null, 2)
			}
		} else {
			sendUserToast(`ResourceType cannot be undefined.`, true)
		}
	}

	function resourceAction() {
		return resourceToEdit ? editResource() : createResource()
	}

	let inputCheck: { [id: string]: boolean } = {}

	$: isValid = allTrue(inputCheck) ?? false
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent
		title={resourceToEdit ? 'Edit ' + resourceToEdit.path : 'Add a resource'}
		on:close={drawer.closeDrawer}
	>
		<div>
			<!-- content -->
			{#if step === 1}
				<div class="flex flex-col gap-3 py-3  text-gray-700">
					<div>
						<span class="text-red-600 text-2xs grow">{error ?? ''}</span>
						<span class="mb-1 font-semibold text-gray-700">Path</span>
						<Path
							bind:error={pathError}
							bind:path
							{initialPath}
							namePlaceholder="my_resource"
							kind="resource"
						/>
					</div>
					<span class=" mt-3 font-semibold text-gray-700 "
						>Description <Required required={false} />
					</span>
					<textarea
						type="text"
						use:autosize
						bind:value={description}
						placeholder={DESCRIPTION_PLACEHOLDER}
					/>
					<div>
						<div class="mb-2 font-semibold text-gray-700">
							Resource type<Required required={true} />
						</div>
						<ResourceTypePicker
							bind:value={selectedResourceType}
							notPickable={resourceToEdit != undefined}
							on:click={() => {
								args = {}
							}}
						/>
					</div>
				</div>
			{:else}
				<div class="text-sm">
					{#if resourceSchema && resourceSchema?.properties}
						{#each Object.keys(resourceSchema.properties) as fieldName}
							<div class="flex flex-row w-full items-end justify-between">
								<ArgInput
									label={fieldName}
									description={resourceSchema.properties[fieldName]?.description}
									bind:value={args[fieldName]}
									type={resourceSchema.properties[fieldName]?.type}
									required={resourceSchema.required.includes(fieldName)}
									pattern={resourceSchema.properties[fieldName]?.pattern}
									bind:valid={inputCheck[fieldName]}
									defaultValue={resourceSchema.properties[fieldName]?.default}
									enum_={resourceSchema.properties[fieldName]?.enum}
									contentEncoding={resourceSchema.properties[fieldName]?.contentEncoding}
									itemsType={resourceSchema.properties[fieldName]?.items}
									properties={resourceSchema.properties[fieldName]?.properties}
									format={resourceSchema.properties[fieldName]?.format}
								/>
								<div class="pb-6 ml-2 relative">
									<Button
										variant="border"
										color="blue"
										size="sm"
										btnClasses="min-w-min items-center leading-4 py-0"
										on:click={() => {
											pickForField = fieldName
											itemPicker.openDrawer()
										}}>Insert variable</Button
									>
								</div>
							</div>
						{/each}
					{:else}
						<div class="h-full w-full">
							<SimpleEditor class="editor" lang="json" bind:code={rawCode} />
						</div>
					{/if}
				</div>
			{/if}
		</div>
		<span slot="submission" class="flex gap-4 mr-2">
			{#if step === 1}
				<Button
					target="_blank"
					color="blue"
					variant="border"
					size="sm"
					href="/resources?connect_app=undefined"
				>
					Connect an API
				</Button>
				<Button
					on:click={async () => {
						await loadResourceType()
						step = 2
					}}
					disabled={selectedResourceType == undefined || pathError != ''}
				>
					Next
				</Button>
			{:else}
				<Button variant="border" on:click={() => (step = 1)}>Back</Button>
				<Button on:click={resourceAction} disabled={!isValid}>Save</Button>
			{/if}
		</span>
	</DrawerContent>
</Drawer>

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, _) => {
		if (pickForField) {
			args[pickForField] = '$var:' + path
		}
	}}
	itemName="Variable"
	extraField="name"
	loadItems={async () =>
		(await VariableService.listVariable({ workspace: $workspaceStore ?? '' })).map((x) => ({
			name: x.path,
			...x
		}))}
>
	<div
		slot="submission"
		class="flex flex-row-reverse w-full p-5 bg-white border-t border-gray-200 rounded-bl-lg rounded-br-lg"
	>
		<Button
			variant="border"
			color="blue"
			size="sm"
			on:click={() => {
				variableEditor.initNew()
			}}
		>
			Create a new variable
		</Button>
	</div>
</ItemPicker>

<VariableEditor bind:this={variableEditor} on:create={itemPicker.openDrawer} />
