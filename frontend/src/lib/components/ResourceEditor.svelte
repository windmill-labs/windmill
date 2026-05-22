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
	import { UserDraft, checkStaleness, type UserDraftHandle } from '$lib/userDraft.svelte'
	import { notifyRestoredFromLocal } from '$lib/userDraftToast'
	import LocalDraftStaleModal from './common/confirmationModal/LocalDraftStaleModal.svelte'

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
		selected: selectedProp = $bindable()
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

	// Local-draft staleness modal: opened when the backend resource moved
	// on (someone else edited it) since the local autosave was written.
	let staleModalOpen = $state(false)
	let pendingStale: { ws: string; backend: ResourceState } | undefined = undefined

	function onStaleLoadLatest(): void {
		if (!pendingStale) {
			staleModalOpen = false
			return
		}
		const { ws, backend } = pendingStale
		// Drop the divergent autosave and reset the handle to the freshly
		// fetched backend state. A later edit re-creates the autosave and
		// the seeding effect records the new rev.
		UserDraft.discard('resource', initialPath ?? '', backend, { workspace: ws })
		initialStates[ws] = $state.snapshot(backend) as ResourceState
		pendingStale = undefined
		staleModalOpen = false
	}

	function onStaleKeepDraft(): void {
		if (pendingStale) {
			const { ws } = pendingStale
			// Ack the new backend rev so the modal doesn't fire again until
			// the backend moves once more. Keeps the local autosave intact.
			UserDraft.saveMeta(
				'resource',
				initialPath ?? '',
				{ remoteRev: fetchedRev[ws] },
				{ workspace: ws }
			)
		}
		pendingStale = undefined
		staleModalOpen = false
	}

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
				args: (defaultValues && Object.keys(defaultValues).length > 0
					? defaultValues
					: {}) as any,
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
				// Reconcile the local autosave with the backend before the
				// handle is registered. If the backend moved on since the
				// autosave was written (recorded rev != current rev) surface
				// the staleness modal; otherwise the form is just showing the
				// user's unsaved work — a toast with a "Reset to deployed"
				// escape is enough.
				const persisted = UserDraft.get<ResourceState>('resource', initialPath ?? '', {
					workspace: ws
				})
				const previousMeta = UserDraft.getMeta('resource', initialPath ?? '', { workspace: ws })
				if (persisted !== undefined && !deepEqual(persisted, s)) {
					const cause = checkStaleness(previousMeta, r.edited_at)
					if (cause) {
						pendingStale = { ws, backend: s }
						staleModalOpen = true
					} else {
						if (previousMeta.remoteRev === undefined && previousMeta.remoteDraftRev === undefined) {
							// Legacy autosave (no rev recorded) — backfill so the
							// next backend change is detectable as drift.
							UserDraft.saveMeta(
								'resource',
								initialPath ?? '',
								{ remoteRev: r.edited_at },
								{ workspace: ws }
							)
						}
						notifyRestoredFromLocal(false, true, {
							onResetToDeployed: () => {
								UserDraft.discard('resource', initialPath ?? '', s, { workspace: ws })
							}
						})
					}
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

<LocalDraftStaleModal
	open={staleModalOpen}
	cause="version"
	onLoadLatest={onStaleLoadLatest}
	onKeepDraft={onStaleKeepDraft}
/>

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
