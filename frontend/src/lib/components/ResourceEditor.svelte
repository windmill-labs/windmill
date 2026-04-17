<script lang="ts">
	import type { Schema } from '$lib/common'
	import { ResourceService, WorkspaceService, type Resource, type ResourceType } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { createEventDispatcher, untrack } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { clearJsonSchemaResourceCache } from './schema/jsonSchemaResource.svelte'
	import ResourceForm from './ResourceForm.svelte'
	import { resource } from 'runed'

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
	let initialPath = path

	let isValid = $state(true)
	let jsonError = $state('')
	let viewJsonSchema = $state(false)

	let description: string = $state('')
	let labels: string[] | undefined = $state(undefined)
	let args: Record<string, any> = $state({})
	let wsSpecific = $state(false)

	const dispatch = createEventDispatcher()

	const deployToResource = resource(
		() => effectiveWorkspace,
		async (ws) =>
			ws ? (await WorkspaceService.getDeployTo({ workspace: ws })).deploy_to : undefined
	)

	const resource_ = resource(
		() => (initialPath ? { path: initialPath, workspace: effectiveWorkspace } : null),
		async (args) => (args ? await ResourceService.getResource(args) : null)
	)
	const resourceType = resource([() => resource_type, () => effectiveWorkspace], async () =>
		resource_type && effectiveWorkspace
			? ResourceService.getResourceType({ path: resource_type, workspace: effectiveWorkspace })
			: null
	)

	let deployTo = $derived(deployToResource.current)
	let resourceToEdit: Resource | undefined = $derived(resource_.current ?? undefined)
	let resourceTypeInfo: ResourceType | undefined = $derived(resourceType.current ?? undefined)
	let resourceSchema: Schema | undefined = $derived.by(() => {
		const rt = resourceType.current
		if (!rt?.schema) return undefined
		const schema = rt.schema as Schema
		schema.order = schema.order ?? Object.keys(schema.properties).sort()
		return schema
	})
	let loadingSchema = $derived(resourceType.loading)
	let can_write = $derived(
		resourceToEdit
			? resourceToEdit.workspace_id == effectiveWorkspace &&
					canWrite(path, resourceToEdit.extra_perms ?? {}, $userStore)
			: true
	)
	let linkedVars = $derived(
		Object.entries(args ?? {})
			.filter(([_, v]) => typeof v == 'string' && v == `$var:${initialPath}`)
			.map(([k, _]) => k)
	)

	$effect(() => {
		const r = resource_.current
		if (r) {
			untrack(() => {
				description = r.description ?? ''
				labels = r.labels ?? undefined
				resource_type = r.resource_type
				args = r.value ?? ({} as any)
				wsSpecific = r.ws_specific ?? false
				path = r.path
			})
		}
	})

	$effect(() => {
		if (defaultValues && Object.keys(defaultValues).length > 0) {
			args = defaultValues
		}
	})

	$effect(() => {
		canSave = (can_write && isValid && jsonError == '') || (viewJsonSchema && jsonError == '')
	})

	$effect(() => {
		onChange?.({ path, args, description })
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

	export async function editResource(): Promise<void> {
		if (!resourceToEdit) {
			throw Error('Cannot edit undefined resource')
		}
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
			onLoadResourceType={() => resourceType.refetch()}
		/>
	</div>
</div>
