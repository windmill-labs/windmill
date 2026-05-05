<script lang="ts">
	import type { Schema } from '$lib/common'
	import { ResourceService, WorkspaceService, type Resource, type ResourceType } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { createEventDispatcher, untrack } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { clearJsonSchemaResourceCache } from './schema/jsonSchemaResource.svelte'
	import ResourceForm from './ResourceForm.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { resource } from 'runed'
	import { deepEqual } from 'fast-equals'
	import { getUserExt } from '$lib/user'
	import type { UserExt } from '$lib/stores'

	interface Props {
		canSave?: boolean
		resource_type?: string | undefined
		path?: string
		hidePath?: boolean
		onChange?: (args: { path: string; args: Record<string, any>; description: string }) => void
		defaultValues?: Record<string, any> | undefined
		workspace?: string | undefined
		selected?: string | undefined
	}

	let {
		canSave = $bindable(true),
		resource_type = $bindable(undefined),
		path = $bindable(''),
		hidePath = false,
		onChange,
		defaultValues = undefined,
		workspace = undefined,
		selected = $bindable()
	}: Props = $props()

	type ResourceState = {
		path: string
		description: string
		args: Record<string, any>
		labels: string[] | undefined
		wsSpecific: boolean
	}

	const dispatch = createEventDispatcher()

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)
	let initialPath = path

	let states: Record<string, ResourceState> = $state({})
	let initialStates: Record<string, ResourceState> = $state({})
	let existedInitially: Record<string, boolean> = $state({})
	let fetchedResources: Record<string, Resource> = $state({})
	let perWsUser: Record<string, UserExt | undefined> = $state({})

	let isValid = $state(true)
	let jsonError = $state('')
	let viewJsonSchema = $state(false)
	let perWsValid: Record<string, boolean> = $state({})

	const deployToResource = resource(
		() => selected,
		async (ws) =>
			ws ? (await WorkspaceService.getDeployTo({ workspace: ws })).deploy_to : undefined
	)
	const resourceTypeResource = resource(
		[() => resource_type, () => effectiveWorkspace],
		async ([rt, ws]) =>
			rt && ws ? ResourceService.getResourceType({ path: rt, workspace: ws }) : null
	)

	let deployTo = $derived(deployToResource.current)
	let resourceTypeInfo: ResourceType | undefined = $derived(
		resourceTypeResource.current ?? undefined
	)
	let resourceSchema: Schema | undefined = $derived.by(() => {
		const rt = resourceTypeResource.current
		if (!rt?.schema) return undefined
		const schema = rt.schema as Schema
		return {
			...schema,
			order: schema.order ?? Object.keys(schema.properties).sort()
		}
	})
	let loadingSchema = $derived(resourceTypeResource.loading)

	let current = $derived(selected ? states[selected] : undefined)
	let resourceToEdit: Resource | undefined = $derived(
		selected ? fetchedResources[selected] : undefined
	)
	let can_write = $derived.by(() => {
		if (!selected) return true
		const r = fetchedResources[selected]
		if (!r) return true
		return canWrite(
			current?.path ?? initialPath,
			r.extra_perms ?? {},
			perWsUser[selected] ?? $userStore
		)
	})

	let linkedVars = $derived(
		Object.entries(current?.args ?? {})
			.filter(([_, v]) => typeof v == 'string' && v == `$var:${initialPath}`)
			.map(([k, _]) => k)
	)

	const dirtyWorkspaces = $derived(
		Object.keys(states).filter((ws) => !deepEqual(states[ws], initialStates[ws]))
	)
	const anyDirty = $derived(dirtyWorkspaces.length > 0)
	const otherDirty = $derived(
		dirtyWorkspaces.length == 1
			? dirtyWorkspaces.filter((ws) => ws !== $workspaceStore)
			: dirtyWorkspaces
	)
	const dirtyValid = $derived(dirtyWorkspaces.every((ws) => perWsValid[ws] !== false))
	const dirtyCanWrite = $derived(
		dirtyWorkspaces.every((ws) => {
			const r = fetchedResources[ws]
			return (
				!r ||
				canWrite(states[ws]?.path ?? initialPath, r.extra_perms ?? {}, perWsUser[ws] ?? $userStore)
			)
		})
	)

	// Bootstrap: ensure selected is set on mount (edit or new)
	$effect(() => {
		if (selected !== undefined) return
		if (!effectiveWorkspace) return
		untrack(() => {
			selected = effectiveWorkspace
			if (!initialPath) {
				// New resource
				const s: ResourceState = {
					path: '',
					description: '',
					args: (defaultValues && Object.keys(defaultValues).length > 0
						? defaultValues
						: {}) as any,
					labels: undefined,
					wsSpecific: false
				}
				states[effectiveWorkspace] = s
				initialStates[effectiveWorkspace] = structuredClone(s)
				existedInitially[effectiveWorkspace] = false
			}
		})
	})

	// Lazy-fetch the resource for the selected workspace when not already cached
	$effect(() => {
		const ws = selected
		if (!ws || !initialPath) return
		if (ws in states) return
		untrack(() => {
			Promise.all([
				ResourceService.getResource({ workspace: ws, path: initialPath }),
				getUserExt(ws)
			]).then(([r, user]) => {
				fetchedResources[ws] = r
				const s: ResourceState = {
					path: r.path,
					description: r.description ?? '',
					args: (r.value ?? {}) as any,
					labels: r.labels ?? undefined,
					wsSpecific: r.ws_specific ?? false
				}
				states[ws] = s
				initialStates[ws] = structuredClone(s)
				existedInitially[ws] = true
				perWsUser[ws] = user
				// Keep resource_type in sync for the base workspace (controls the schema)
				if (ws === effectiveWorkspace) {
					resource_type = r.resource_type
				}
			})
		})
	})

	// Keep current.path bound to the outer `path` prop for consumers
	$effect(() => {
		if (current) path = current.path
	})

	$effect(() => {
		if (selected !== undefined) {
			perWsValid[selected] = isValid && jsonError == ''
		}
	})

	$effect(() => {
		canSave = anyDirty && dirtyValid && dirtyCanWrite
	})

	$effect(() => {
		if (current)
			onChange?.({ path: current.path, args: current.args, description: current.description })
	})

	$effect(() => {
		if (!current) return
		if (linkedVars.length > 0 && current.path) {
			untrack(() => {
				linkedVars.forEach((k) => {
					current!.args[k] = `$var:${current!.path}`
				})
			})
		}
	})

	export async function save(): Promise<void> {
		const dirty = dirtyWorkspaces
		try {
			for (const ws of dirty) {
				const s = states[ws]
				const ini = initialStates[ws]
				if (existedInitially[ws]) {
					await ResourceService.updateResource({
						workspace: ws,
						path: ini.path,
						requestBody: {
							path: s.path,
							value: s.args,
							description: s.description,
							labels: s.labels,
							ws_specific: s.wsSpecific
						}
					})
					const fetched = fetchedResources[ws]
					if (fetched?.resource_type === 'json_schema') {
						clearJsonSchemaResourceCache(ini.path, ws)
					}
				} else {
					await ResourceService.createResource({
						workspace: ws,
						requestBody: {
							path: s.path,
							value: s.args,
							description: s.description,
							resource_type: resource_type!,
							labels: s.labels,
							ws_specific: s.wsSpecific
						}
					})
				}
			}
			sendUserToast(
				dirty.length > 1 ? `Saved resource in ${dirty.length} workspaces` : `Saved resource`
			)
			dispatch('refresh', current?.path ?? path)
		} catch (err) {
			sendUserToast(`Could not save resource: ${err.body ?? err.message}`, true)
		}
	}
</script>

<div>
	<div class="flex flex-col gap-6 py-2">
		{#if otherDirty.length > 0}
			<Alert type="warning" title="Editing multiple workspaces">
				You are going to edit the value in: {otherDirty.join(', ')}
			</Alert>
		{/if}

		{#if current}
			{#key current}
				<ResourceForm
					bind:path={current.path}
					bind:labels={current.labels}
					bind:description={current.description}
					bind:args={current.args}
					bind:wsSpecific={current.wsSpecific}
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
					onLoadResourceType={() => resourceTypeResource.refetch()}
				/>
			{/key}
		{/if}
	</div>
</div>
