<script lang="ts">
	import type { Schema } from '$lib/common'
	import {
		GitSyncService,
		ResourceService,
		WorkspaceService,
		type Resource,
		type ResourceType
	} from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { createEventDispatcher, untrack } from 'svelte'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { clearJsonSchemaResourceCache } from './schema/jsonSchemaResource.svelte'
	import ResourceForm from './ResourceForm.svelte'
	import ReplaceGitCredential from './git_sync/ReplaceGitCredential.svelte'
	import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'
	import Alert from './common/alert/Alert.svelte'
	import DraftConflictAlert from './DraftConflictAlert.svelte'
	import { resource } from 'runed'
	import { useActingUser } from '$lib/actingUser.svelte'
	import { draftValuesEqual } from '$lib/userDraft.svelte'
	import { onUserInput } from '$lib/userDraftEditGate'
	import {
		newItemPath,
		saveEach,
		useItems,
		type ItemAdapter,
		type ItemHandle
	} from '$lib/itemStore.svelte'

	interface Props {
		canSave?: boolean
		resource_type?: string | undefined
		path?: string
		hidePath?: boolean
		onChange?: (args: { path: string; args: Record<string, any>; description: string }) => void
		defaultValues?: Record<string, any> | undefined
		/** Workspace this editor acts in — every call, permission check and cache key below
		 * derives from it. Optional for the packaged component; the navigation workspace is
		 * substituted once, at `effectiveWorkspace`, and nowhere else. */
		workspace?: string | undefined
		selected?: string | undefined
		/** Show the value as JSON rather than as the resource type's form. Bindable so a caller can
		 *  choose the view a given resource opens on, and the in-form toggle still works. */
		viewJsonSchema?: boolean
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
		viewJsonSchema = $bindable(),
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

	// Sole ambient read in this file: the acting workspace is an input, and only its
	// default comes from the navigation store.
	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)
	// Fallback to `effectiveWorkspace` insulates against reactify-style
	// parents that re-spread props without `selected` — otherwise it
	// transiently resets and the form below remounts on every keystroke.
	let selected = $derived(selectedProp ?? effectiveWorkspace)
	let initialPath = path
	// The item this editor opens: the resource at `initialPath`, or a new one.
	const itemPath = initialPath || newItemPath()
	// Read once, like the path: this editor opens one item for its lifetime.
	const template: ResourceState = untrack(() => ({
		path: '',
		description: '',
		args: (defaultValues && Object.keys(defaultValues).length > 0 ? defaultValues : {}) as any,
		labels: undefined,
		wsSpecific: false
	}))

	// Every workspace the editor has shown this resource in (WsSpecificVersions re-points it).
	let workspaces = $state<string[]>([])
	const acting = useActingUser(() => selected)
	let perWsValid: Record<string, boolean> = $state({})

	/** The schema can only ever write `args`. `path`, `labels`, `description` and
	 * `wsSpecific` are beyond its reach, so a difference in one of those is the
	 * user's — whatever event did or didn't reach the gate. Removing a label runs
	 * a click handler and emits nothing native, and would otherwise be absorbed. */
	function differsOutsideArgs(a: ResourceState, b: ResourceState): boolean {
		return !draftValuesEqual({ ...a, args: null }, { ...b, args: null })
	}

	const resourceAdapter: ItemAdapter<ResourceState> = {
		// The schema form materializes values the stored resource never carried as it renders;
		// until the user's first input those join the deployed side instead of making a draft.
		settles: true,
		absorbs: (next, deployed) => !differsOutsideArgs(next, deployed),
		async load({ workspace, path }) {
			const r = await ResourceService.getResource({ workspace, path, getDraft: true })
			const { draft, draft_saved_at, no_deployed } = r as any
			return {
				// Draft-only paths (`no_deployed`) have no row — saving must CREATE.
				deployed: no_deployed
					? undefined
					: {
							path: r.path,
							description: r.description ?? '',
							args: (r.value ?? {}) as any,
							labels: r.labels ?? undefined,
							wsSpecific: r.ws_specific ?? false
						},
				// `.draft` already holds the editor's `ResourceState` shape.
				draft: draft as ResourceState | undefined,
				draftSavedAt: draft_saved_at,
				meta: r
			}
		},
		async write({ workspace, path, value: s, deployed, meta }) {
			const fetched = meta as Resource | undefined
			if (deployed) {
				await ResourceService.updateResource({
					workspace,
					path,
					requestBody: {
						path: s.path,
						value: s.args,
						description: s.description,
						labels: s.labels,
						ws_specific: s.wsSpecific
					}
				})
				if (fetched?.resource_type === 'json_schema') clearJsonSchemaResourceCache(path, workspace)
			} else {
				await ResourceService.createResource({
					workspace,
					requestBody: {
						path: s.path,
						value: s.args,
						description: s.description,
						resource_type: fetched?.resource_type ?? resource_type!,
						labels: s.labels,
						ws_specific: s.wsSpecific
					}
				})
			}
			// Path now exists server-side — drop the autocomplete cache so
			// it shows up immediately instead of after the 60s TTL.
			invalidateWorkspacePaths(workspace)
		}
	}

	const handles = useItems<ResourceState>(
		'resource',
		() =>
			workspaces.map((ws) => ({
				workspace: ws,
				path: itemPath,
				template,
				valid: () => perWsValid[ws] !== false,
				writable: () => {
					const r = fetchedOf(ws)
					return (
						!r ||
						canWrite(items[ws]?.value?.path ?? initialPath, r.extra_perms ?? {}, acting.in(ws))
					)
				}
			})),
		resourceAdapter
	)
	const items = $derived.by(() => {
		const out: Record<string, ItemHandle<ResourceState>> = {}
		workspaces.forEach((ws, i) => {
			const handle = handles[i]
			if (handle) out[ws] = handle
		})
		return out
	})

	function fetchedOf(ws: string): Resource | undefined {
		return items[ws]?.meta as Resource | undefined
	}

	// Open the selected workspace's version of the resource alongside the others. A new one
	// only exists in the workspace it is being created in.
	$effect(() => {
		const ws = selected
		if (!ws) return
		untrack(() => {
			if (workspaces.includes(ws)) return
			if (initialPath) workspaces.push(ws)
			else workspaces = [ws]
		})
	})

	const selectedItem = $derived(selected ? items[selected] : undefined)

	// Nothing counts until this workspace's form is on screen, and while the
	// schema is still arriving a precursor alone does not: it would open the gate
	// just in time for the schema's materialized values to join the draft. `Path`,
	// the labels and the description render above that skeleton and stay editable
	// throughout, so a real value event still counts and keeps the edit.
	onUserInput((kind) => {
		if (!selectedItem?.loaded) return
		if (kind === 'precursor' && loadingSchema) return
		selectedItem.markEdited()
	})

	let isValid = $state(true)
	let jsonError = $state('')

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
			// A resource type may declare no properties at all — `dbt_profile` is a
			// `profiles.yml` block whose keys are its adapter's, not Windmill's — and
			// the form renders those as one JSON editor. `Object.keys(undefined)`
			// threw here, which left the editor on its loading skeleton forever.
			order: schema.order ?? Object.keys(schema.properties ?? {}).sort()
		}
	})
	let loadingSchema = $derived(resourceTypeResource.loading)

	let current = $derived(selectedItem?.value)
	// The saved URL, not the draft's: a credential is bound to the repository it
	// is issued for, so binding one to an edit that has not landed yet would tie
	// it to something the resource does not point at.
	let deployedUrl = $derived(
		selected ? ((fetchedOf(selected)?.value as any)?.url as string | undefined) : undefined
	)
	// The deployed path, for the same reason as the deployed URL: the server
	// answers about what is stored, and an unsaved rename names nothing yet.
	let deployedPath = $derived(
		selected && initialPath ? (selectedItem?.deployed?.path ?? initialPath) : undefined
	)
	// Asked of the server rather than read off the resource: the resource is
	// client-editable, exported and copied into forks, so nothing written on it
	// stays true. Re-asked when the saved URL moves, since that is a different
	// repository. The answer is for admins, who are the only ones who could act
	// on it, so nobody else asks.
	const credentialOrigin = resource(
		[
			() => selected,
			() => deployedPath,
			() => deployedUrl,
			() => resource_type,
			() => acting.in(selected)?.is_admin
		],
		async ([ws, path, _url, type, admin]) =>
			ws && path && type === 'git_repository' && admin
				? await GitSyncService.getCredentialOrigin({ workspace: ws, path }).catch(() => undefined)
				: undefined
	)
	// Only a credential this workspace holds is its to replace: a fork borrows
	// its ancestor's, and storing a replacement here would split it in two.
	let holdsCredential = $derived(credentialOrigin.current?.origin === 'held')
	// Only an unsaved *URL* blocks replacing the token, not any unsaved change:
	// opening the drawer materialises schema defaults (`folder: ""`), so a whole-
	// resource dirty check would disable it the moment the drawer opens.
	let urlDirty = $derived(!!deployedUrl && current?.args?.url !== deployedUrl)
	let resourceToEdit: Resource | undefined = $derived(selected ? fetchedOf(selected) : undefined)
	// `undefined` until both the resource and the acting user have landed — a pending verdict
	// is neither a grant nor the denial the read-only alert announces, so the two must stay
	// distinguishable.
	let can_write: boolean | undefined = $derived.by(() => {
		// A resource that does not exist yet has nobody's permissions on it.
		if (!initialPath || !selected) return true
		const r = fetchedOf(selected)
		if (!r || !acting.resolved(selected)) return undefined
		return canWrite(current?.path ?? initialPath, r.extra_perms ?? {}, acting.in(selected))
	})

	const dirtyWorkspaces = $derived(Object.keys(items).filter((ws) => items[ws].dirty))
	// Banner is scoped to the selected workspace — the diff/discard only
	// operate on it, so showing it for an unrelated dirty workspace would be
	// misleading. The cross-workspace `otherDirty` alert below still covers
	// that case.
	const selectedDirty = $derived(!!selectedItem?.dirty)
	const otherDirty = $derived(
		dirtyWorkspaces.length == 1
			? dirtyWorkspaces.filter((ws) => ws !== effectiveWorkspace)
			: dirtyWorkspaces
	)

	// Keep resource_type in sync for the base workspace (controls the schema)
	$effect(() => {
		const r = fetchedOf(effectiveWorkspace)
		if (r) untrack(() => (resource_type = r.resource_type))
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
		canSave = dirtyWorkspaces.length > 0 && dirtyWorkspaces.every((ws) => items[ws].canSave)
	})

	// Drive the parent drawer's "unsaved changes" banner. The drawer chrome
	// (header + banner slot) lives in ResourceEditorDrawer, above this
	// lazily-imported content, so the state is lifted up via these accessors.
	$effect(() => {
		onDraftStateChange?.(!!initialPath && selectedDirty)
	})
	$effect(() => {
		onCanWriteChange?.(can_write === true)
	})

	export function localDraftDeployed(): ResourceState | undefined {
		return selectedItem?.deployed
	}
	export function localDraftCurrent(): ResourceState | undefined {
		return current
	}
	/** Resolves `true` when the discard removed the resource: a draft-only one is its draft. */
	export async function discardLocalDraft(): Promise<boolean> {
		const outcome = await selectedItem?.discard()
		return !!outcome?.removed
	}

	// A draft-only resource is gone once its draft is discarded.
	$effect(() => {
		if (selectedItem?.removed) untrack(() => dispatch('refresh', initialPath))
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

	/** Sole writer of `current.path` — an arg still holding `$var:<the path being
	 * replaced>` is the resource's own linked secret, which the backend renames
	 * along with the resource, so the reference moves with it. An arg pointing at
	 * any other variable was set by the user and is left alone. */
	function setPath(npath: string): void {
		if (!current) return
		const prev = current.path
		// `args` is whatever the raw JSON editor parsed — `null` included.
		for (const [k, v] of Object.entries(current.args ?? {})) {
			if (v === `$var:${prev}`) current.args[k] = `$var:${npath}`
		}
		current.path = npath
	}

	/** Whether the write landed. It toasts its own failure, so most callers ignore this;
	 * one that follows the save with bookkeeping of its own has to know not to. */
	export async function save(): Promise<boolean> {
		const targets = dirtyWorkspaces.map((ws) => items[ws])
		const outcomes = await saveEach(targets)
		const failed = outcomes.find((o) => !o.ok && !o.skipped)
		if (failed && !failed.ok) {
			sendUserToast(`Could not save resource: ${failed.error}`, true)
			return false
		}
		const last = outcomes.at(-1)
		sendUserToast(
			targets.length > 1 ? `Saved resource in ${targets.length} workspaces` : `Saved resource`
		)
		dispatch('refresh', last?.ok ? last.path : path)
		return true
	}
</script>

<div>
	<div class="flex flex-col gap-6 pb-2">
		{#if selectedItem?.status === 'conflicted'}
			<DraftConflictAlert
				onReload={() => selectedItem?.resolveConflict('reload')}
				onOverwrite={() => selectedItem?.resolveConflict('overwrite')}
			/>
		{/if}

		{#if otherDirty.length > 0}
			<Alert type="warning" title="Editing multiple workspaces">
				You are going to edit the value in: {otherDirty.join(', ')}
			</Alert>
		{/if}

		{#if holdsCredential && selected}
			<Alert type="info" title="Windmill holds this repository's access token">
				<div class="flex flex-col items-start gap-2">
					<div>
						The URL carries no credential. Windmill stores the token and renews it before it
						expires.
						{#if urlDirty}
							Save your URL change to replace the token.
						{/if}
					</div>
					<ReplaceGitCredential
						workspace={selected}
						repoUrl={deployedUrl ?? ''}
						disabled={urlDirty || !deployedUrl}
					/>
				</div>
			</Alert>
		{/if}

		<!-- Held back until there is a verdict: rendering the form against a pending `can_write`
			would flash read-only controls at someone who can in fact write. -->
		{#if current && can_write !== undefined}
			{#key current}
				<ResourceForm
					bind:path={() => current!.path, setPath}
					bind:labels={current.labels}
					bind:description={current.description}
					bind:args={current.args}
					bind:wsSpecific={current.wsSpecific}
					bind:isValid
					bind:viewJsonSchema={() => viewJsonSchema ?? false, (v) => (viewJsonSchema = v)}
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
					workspace={selected}
					actingUser={acting.in(selected) ?? null}
				/>
			{/key}
		{/if}
	</div>
</div>
