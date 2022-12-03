<script lang="ts">
	import { type Resource, ResourceService, type ResourceType, VariableService } from '$lib/gen'
	import { canWrite, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import type { Schema } from '$lib/common'
	import Path from './Path.svelte'
	import Required from './Required.svelte'
	import { Alert, Button, Drawer, Skeleton } from './common'

	import { userStore, workspaceStore } from '$lib/stores'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import autosize from 'svelte-autosize'
	import SimpleEditor from './SimpleEditor.svelte'
	import { faSave } from '@fortawesome/free-solid-svg-icons'
	import SchemaForm from './SchemaForm.svelte'

	let path = ''
	let initialPath = ''
	let pathError = ''

	let resourceToEdit: Resource | undefined

	let description: string = ''
	let DESCRIPTION_PLACEHOLDER = `You can use markdown to style your description`
	let selectedResourceType: string | undefined
	let resourceSchema: Schema | undefined
	let args: Record<string, any> = {}
	let can_write = true
	let loadingSchema = false

	let error: string | undefined

	let drawer: Drawer

	let rawCode: string | undefined = undefined

	const dispatch = createEventDispatcher()

	export async function initEdit(p: string): Promise<void> {
		initialPath = p
		path = p
		resourceToEdit = undefined
		resourceSchema = undefined
		loadingSchema = true
		drawer.openDrawer?.()
		resourceToEdit = await ResourceService.getResource({ workspace: $workspaceStore!, path: p })
		description = resourceToEdit!.description ?? ''
		selectedResourceType = resourceToEdit!.resource_type
		loadResourceType()
		args = resourceToEdit!.value
		can_write =
			resourceToEdit.workspace_id == $workspaceStore &&
			canWrite(p, resourceToEdit.extra_perms ?? {}, $userStore)
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
				const resourceType = await ResourceService.getResourceType({
					workspace: $workspaceStore!,
					path: selectedResourceType
				})

				if (resourceType.schema) {
					resourceSchema = resourceType.schema as Schema
				}
			} catch (err) {
				resourceSchema = undefined
				loadingSchema = false
				rawCode = JSON.stringify(args, null, 2)
			}
		} else {
			sendUserToast(`ResourceType cannot be undefined.`, true)
		}
		loadingSchema = false
	}

	let isValid = true
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
					{#if loadingSchema}
						<Skeleton layout={[[4]]} />
					{:else if resourceSchema && resourceSchema?.properties}
						<SchemaForm
							disabled={!can_write}
							compact
							schema={resourceSchema}
							bind:args
							bind:isValid
						/>
					{:else if !can_write}
						<input type="text" disabled value={rawCode} />
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
