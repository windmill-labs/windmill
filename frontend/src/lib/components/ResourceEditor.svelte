<script lang="ts">
	import { type Resource, ResourceService, type ResourceType, VariableService } from '$lib/gen'
	import { allTrue, canWrite, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import type { Schema } from '$lib/common'
	import Path from './Path.svelte'
	import ArgInput from './ArgInput.svelte'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'
	import Required from './Required.svelte'
	import { Alert, Button, Drawer } from './common'

	import { userStore, workspaceStore } from '$lib/stores'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import autosize from 'svelte-autosize'
	import SimpleEditor from './SimpleEditor.svelte'
	import { faSave } from '@fortawesome/free-solid-svg-icons'

	let path = ''
	let initialPath = ''
	let pathError = ''

	let resourceToEdit: Resource | undefined

	let description: string = ''
	let DESCRIPTION_PLACEHOLDER = `You can use markdown to style your description`
	let selectedResourceType: string | undefined
	let resourceType: ResourceType
	let resourceSchema: Schema | undefined
	let args: Record<string, any> = {}
	let can_write = true

	let error: string | undefined

	let pickForField: string | undefined
	let itemPicker: ItemPicker
	let variableEditor: VariableEditor
	let drawer: Drawer

	let rawCode: string | undefined = undefined

	const dispatch = createEventDispatcher()

	export async function initEdit(p: string): Promise<void> {
		initialPath = p
		path = p
		resourceToEdit = await ResourceService.getResource({ workspace: $workspaceStore!, path: p })
		description = resourceToEdit!.description ?? ''
		selectedResourceType = resourceToEdit!.resource_type
		args = resourceToEdit!.value
		can_write =
			resourceToEdit.workspace_id == $workspaceStore &&
			canWrite(p, resourceToEdit.extra_perms ?? {}, $userStore)
		await loadResourceType()
		drawer.openDrawer?.()
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
				drawer.closeDrawer?.()
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

	let inputCheck: { [id: string]: boolean } = {}

	$: isValid = allTrue(inputCheck) ?? false
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent
		title={resourceToEdit ? 'Edit ' + resourceToEdit.path : 'Add a resource'}
		on:close={drawer.closeDrawer}
	>
		<div>
			<div class="flex flex-col gap-3 py-3  text-gray-700">
				<div>
					{#if !can_write}
						<div class="m-2">
							<Alert type="warning" title="Only read access"
								>You only have read access to this resource and cannot edit it</Alert
							>
						</div>
					{/if}

					<span class="text-red-600 text-2xs grow">{error ?? ''}</span>
					<Path
						disabled={!can_write}
						bind:error={pathError}
						bind:path
						{initialPath}
						namePlaceholder="my_resource"
						kind="resource"
					/>
				</div>
				<h3>Description <Required required={false} /> </h3>
				<textarea
					type="text"
					disabled={!can_write}
					use:autosize
					bind:value={description}
					placeholder={DESCRIPTION_PLACEHOLDER}
				/>

				<h3 class="mt-4">Value</h3>
				<div class="text-sm">
					{#if resourceSchema && resourceSchema?.properties}
						{#each Object.keys(resourceSchema.properties) as fieldName}
							<div class="flex flex-row w-full items-center justify-between">
								<ArgInput
									compact
									disabled={!can_write}
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
								<div class="ml-2 relative">
									<Button
										variant="border"
										color="blue"
										size="sm"
										btnClasses="min-w-min items-center leading-4 py-0"
										on:click={() => {
											pickForField = fieldName
											itemPicker.openDrawer?.()
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
			</div>
		</div>
		<span slot="submission" class="flex gap-4 mr-2">
			<Button startIcon={{ icon: faSave }} on:click={editResource} disabled={!can_write || !isValid}
				>Save</Button
			>
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
				variableEditor?.initNew?.()
			}}
		>
			Create a new variable
		</Button>
	</div>
</ItemPicker>

<VariableEditor bind:this={variableEditor} on:create={itemPicker.openDrawer} />
