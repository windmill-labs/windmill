<script lang="ts">
	import type { Schema } from '$lib/common'
	import { ResourceService, WorkspaceService, type Resource, type ResourceType } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { createEventDispatcher, untrack } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { clearJsonSchemaResourceCache } from './schema/jsonSchemaResource.svelte'
	import ResourceForm from './ResourceForm.svelte'

	interface Props {
		canSave?: boolean
		resource_type?: string | undefined
		path?: string
		hidePath?: boolean
		onChange?: (args: { path: string; args: Record<string, any>; description: string }) => void
		defaultValues?: Record<string, any> | undefined
		workspace?: string | undefined
	}

	let {
		canSave = $bindable(true),
		resource_type = $bindable(undefined),
		path = $bindable(''),
		hidePath = false,
		onChange,
		defaultValues = undefined,
		workspace = undefined
	}: Props = $props()

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)

	let isValid = $state(true)
	let jsonError = $state('')
	let viewJsonSchema = $state(false)
	let can_write = $state(true)

	let initialPath = path

	let resourceToEdit: Resource | undefined = $state(undefined)

	let description: string = $state('')
	let labels: string[] | undefined = $state(undefined)
	let resourceSchema: Schema | undefined = $state(undefined)
	let args: Record<string, any> = $state({})
	let loadingSchema = $state(true)
	let linkedVars: string[] = $state([])
	let resourceTypeInfo: ResourceType | undefined = $state(undefined)
	let newResource = $derived(!path)
	let wsSpecific = $state(false)
	let deployTo: string | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	$effect(() => {
		if (!effectiveWorkspace) {
			deployTo = undefined
			return
		}

		WorkspaceService.getDeployTo({ workspace: effectiveWorkspace }).then((x) => {
			deployTo = x.deploy_to
		})
	})

	async function initEdit() {
		resourceToEdit = await ResourceService.getResource({ workspace: effectiveWorkspace, path })
		description = resourceToEdit!.description ?? ''
		labels = resourceToEdit!.labels ?? undefined
		resource_type = resourceToEdit!.resource_type
		args = resourceToEdit?.value ?? ({} as any)
		wsSpecific = resourceToEdit?.ws_specific ?? false
		loadResourceType()
		can_write =
			resourceToEdit.workspace_id == effectiveWorkspace &&
			canWrite(path, resourceToEdit.extra_perms ?? {}, $userStore)
		linkedVars = Object.entries(args)
			.filter(([_, v]) => typeof v == 'string' && v == `$var:${initialPath}`)
			.map(([k, _]) => k)
	}

	if (!untrack(() => newResource)) {
		initEdit()
	} else if (resource_type) {
		loadResourceType()
	} else {
		sendUserToast('Resource type cannot be undefined for new resource creation', true)
	}

	export async function editResource(): Promise<void> {
		if (resourceToEdit) {
			await ResourceService.updateResource({
				workspace: effectiveWorkspace,
				path: resourceToEdit.path,
				requestBody: { path, value: args, description, labels, ws_specific: wsSpecific }
			})
			if (resourceToEdit.resource_type === 'json_schema') {
				clearJsonSchemaResourceCache(resourceToEdit.path, effectiveWorkspace)
			}
			sendUserToast(`Updated resource at ${path}`)
			dispatch('refresh', path)
		} else {
			throw Error('Cannot edit undefined resource')
		}
	}

	export async function createResource(): Promise<void> {
		await ResourceService.createResource({
			workspace: effectiveWorkspace,
			requestBody: {
				path,
				value: args,
				description,
				resource_type: resource_type!,
				labels,
				ws_specific: wsSpecific
			}
		})
		sendUserToast(`Updated resource at ${path}`)
		dispatch('refresh', path)
	}

	async function loadResourceType(): Promise<void> {
		if (resource_type) {
			try {
				const resourceType = await ResourceService.getResourceType({
					workspace: effectiveWorkspace,
					path: resource_type
				})

				resourceTypeInfo = resourceType
				if (resourceType.schema) {
					resourceSchema = resourceType.schema as Schema
					resourceSchema.order =
						resourceSchema.order ?? Object.keys(resourceSchema.properties).sort()
				}
			} catch (err) {
				resourceSchema = undefined
				loadingSchema = false
			}
		} else {
			sendUserToast(`ResourceType cannot be undefined.`, true)
		}
		loadingSchema = false
	}

	$effect(() => {
		if (defaultValues && Object.keys(defaultValues).length > 0) {
			args = defaultValues
		}
	})

	$effect(() => {
		canSave = (can_write && isValid && jsonError == '') || (viewJsonSchema && jsonError == '')
	})

	$effect(() => {
		onChange && onChange({ path, args, description })
	})

	$effect(() => {
		if (linkedVars.length > 0 && path) {
			untrack(() => {
				linkedVars.forEach((k) => {
					args[k] = `$var:${path}`
				})
			})
		}
	})
</script>

<div>
	<div class="flex flex-col gap-6 py-2">
		<ResourceForm
			bind:path
			bind:labels
			bind:description
			bind:args
			bind:wsSpecific
			bind:isValid
			bind:viewJsonSchema
			bind:jsonError
			{initialPath}
			{hidePath}
			{deployTo}
			{can_write}
			{resource_type}
			{resourceTypeInfo}
			{resourceSchema}
			{loadingSchema}
			{resourceToEdit}
			onLoadResourceType={loadResourceType}
		/>
	</div>
</div>
