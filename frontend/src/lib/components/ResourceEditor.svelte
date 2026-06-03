<script lang="ts">
	import type { Schema } from '$lib/common'
	import { ResourceService, WorkspaceService, type Resource, type ResourceType } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { createEventDispatcher, untrack } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { clearJsonSchemaResourceCache } from './schema/jsonSchemaResource.svelte'
	import ResourceForm from './ResourceForm.svelte'
	import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { resource } from 'runed'
	import { deepEqual } from 'fast-equals'
	import { getUserExt } from '$lib/user'
	import type { UserExt } from '$lib/stores'
	import { UserDraft, type UserDraftHandle } from '$lib/userDraft.svelte'

	interface Props {
		canSave?: boolean
		resource_type?: string | undefined
		path?: string
		hidePath?: boolean
		onChange?: (args: { path: string; args: Record<string, any>; description: string }) => void
		defaultValues?: Record<string, any> | undefined
		workspace?: string | undefined
		selected?: string | undefined
		/** Notifies the parent drawer whether a local draft for the selected
		 * workspace diverges from the deployed baseline, so it can show the
		 * "unsaved changes" banner below its header. */
		onDraftStateChange?: (hasDraft: boolean) => void
		/** Notifies the parent drawer of write-access for the selected workspace,
		 * so it can hide the banner's Discard button in read-only mode (matches
		 * the trigger editors' `disabled={!can_write}` wiring). */
		onCanWriteChange?: (canWrite: boolean) => void
	}

	let {
		canSave = $bindable(true),
		resource_type = $bindable(undefined),
		path = $bindable(''),
		hidePath = false,
		onChange,
		defaultValues = undefined,
		workspace = undefined,
		selected: selectedProp = $bindable(),
		onDraftStateChange,
		onCanWriteChange
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
	// Fallback to `effectiveWorkspace` insulates against reactify-style
	// parents that re-spread props without `selected` — otherwise it
	// transiently resets and the form below remounts on every keystroke.
	let selected = $derived(selectedProp ?? effectiveWorkspace)
	let initialPath = path

	// Per-workspace handles are driven by `useMany`. We track the workspace
	// IDs (and their seeded defaults) in a parallel `$state` array; on every
	// mutation `useMany` reconciles, acquiring entries for new workspaces and
	// releasing them on component teardown. `states` indexes the resulting
	// handles by workspace ID for ergonomic lookup downstream.
	let workspaceSpecs = $state<Array<{ ws: string; defaultValue: ResourceState }>>([])
	let initialStates: Record<string, ResourceState> = $state({})
	let existedInitially: Record<string, boolean> = $state({})
	let fetchedResources: Record<string, Resource> = $state({})
	let perWsUser: Record<string, UserExt | undefined> = $state({})
	// Backend `edited_at` per workspace — the rev the staleness check
	// compares the local autosave's recorded rev against. Resources have
	// no DB-draft concept, so only `remoteRev` is ever populated.
	let fetchedRev: Record<string, string | undefined> = $state({})

	const handlesArray = UserDraft.useMany<ResourceState>(() =>
		workspaceSpecs.map((s) => ({
			itemKind: 'resource' as const,
			path: initialPath ?? '',
			workspace: s.ws,
			defaultValue: s.defaultValue
		}))
	)
	const states = $derived.by(() => {
		const out: Record<string, UserDraftHandle<ResourceState>> = {}
		for (let i = 0; i < workspaceSpecs.length; i++) {
			const handle = handlesArray[i]
			if (handle) out[workspaceSpecs[i].ws] = handle
		}
		return out
	})

	/** Register a workspace so `useMany` acquires (or reuses) its handle.
	 * `defaultValue` is what the handle reports when no autosave is persisted;
	 * an existing autosave always wins. The default itself never round-trips
	 * to localStorage — only the user's first real edit triggers a write. */
	function ensureHandle(ws: string, defaultValue: ResourceState): void {
		if (workspaceSpecs.some((s) => s.ws === ws)) return
		workspaceSpecs.push({ ws, defaultValue })
	}

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

	let current = $derived(selected ? states[selected]?.draft : undefined)
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
		Object.keys(states).filter((ws) => !deepEqual(states[ws].draft, initialStates[ws]))
	)
	const anyDirty = $derived(dirtyWorkspaces.length > 0)
	// Banner is scoped to the selected workspace — the diff/discard only
	// operate on it, so showing it for an unrelated dirty workspace would be
	// misleading. The cross-workspace `otherDirty` alert below still covers
	// that case.
	const selectedDirty = $derived(!!selected && dirtyWorkspaces.includes(selected))
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
				canWrite(
					states[ws]?.draft?.path ?? initialPath,
					r.extra_perms ?? {},
					perWsUser[ws] ?? $userStore
				)
			)
		})
	)

	// New-resource bootstrap: seed empty state per workspace (edit mode
	// is seeded by the lazy-fetch effect below).
	$effect(() => {
		if (!selected) return
		if (initialPath) return
		if (selected in initialStates) return
		untrack(() => {
			const s: ResourceState = {
				path: '',
				description: '',
				args: (defaultValues && Object.keys(defaultValues).length > 0 ? defaultValues : {}) as any,
				labels: undefined,
				wsSpecific: false
			}
			ensureHandle(selected, s)
			initialStates[selected] = structuredClone(s)
			existedInitially[selected] = false
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
				fetchedRev[ws] = r.edited_at
				const s: ResourceState = {
					path: r.path,
					description: r.description ?? '',
					args: (r.value ?? {}) as any,
					labels: r.labels ?? undefined,
					wsSpecific: r.ws_specific ?? false
				}
				ensureHandle(ws, s)
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

	// Seed the staleness rev the moment a real autosave appears. Until the
	// user's first edit diverges the handle's draft from the backend
	// baseline there's no autosave to attach a rev to; once it does, record
	// the backend rev captured at fetch time so a later external edit is
	// detectable as drift on the next open. Self-limiting: after the write
	// `meta.remoteRev` is set so the guard fails on the re-run.
	$effect(() => {
		for (const ws of Object.keys(states)) {
			const h = states[ws]
			const rev = fetchedRev[ws]
			const baseline = initialStates[ws]
			if (!h || rev === undefined || baseline === undefined) continue
			const draft = h.draft
			if (draft === undefined || deepEqual(draft, baseline)) continue
			if (h.meta.remoteRev !== undefined || h.meta.remoteDraftRev !== undefined) continue
			untrack(() => h.setMeta({ remoteRev: rev }))
		}
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

	// Drive the parent drawer's "unsaved changes" banner. The drawer chrome
	// (header + banner slot) lives in ResourceEditorDrawer, above this
	// lazily-imported content, so the state is lifted up via these accessors.
	$effect(() => {
		onDraftStateChange?.(!!initialPath && selectedDirty)
	})
	$effect(() => {
		onCanWriteChange?.(can_write)
	})

	export function localDraftDeployed(): ResourceState | undefined {
		return selected ? initialStates[selected] : undefined
	}
	export function localDraftCurrent(): ResourceState | undefined {
		return current
	}
	export function discardLocalDraft(): void {
		if (!selected) return
		UserDraft.discard('resource', initialPath ?? '', initialStates[selected], {
			workspace: selected
		})
	}

	$effect(() => {
		if (current)
			// $state.snapshot deep-reads (so the effect re-runs on nested
			// args mutations) and returns a plain object (React consumers
			// can't diff a $state proxy by reference or JSON.stringify).
			onChange?.({
				path: current.path,
				args: $state.snapshot(current.args) as Record<string, any>,
				description: current.description
			})
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
				const s = states[ws].draft!
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
				// Saved on the backend — drop the local autosave for this
				// workspace and refresh the dirty baseline. `s` is the
				// UserDraft handle's draft, a Svelte $state proxy;
				// `structuredClone` can't clone a proxy, so snapshot it to a
				// plain object first.
				initialStates[ws] = $state.snapshot(s) as ResourceState
				UserDraft.remove('resource', initialPath ?? '', { workspace: ws })
				// Path now exists server-side — drop the autocomplete cache so
				// it shows up immediately instead of after the 60s TTL.
				invalidateWorkspacePaths(ws)
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
