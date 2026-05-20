<script lang="ts">
	import { VariableService, WorkspaceService } from '$lib/gen'
	import { createEventDispatcher, untrack } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import { Save } from 'lucide-svelte'
	import VariableForm from './VariableForm.svelte'
	import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'
	import WsSpecificVersions from './WsSpecificVersions.svelte'
	import { resource } from 'runed'
	import { deepEqual } from 'fast-equals'
	import { getUserExt } from '$lib/user'
	import type { UserExt } from '$lib/stores'
	import { UserDraft, checkStaleness, type UserDraftHandle } from '$lib/userDraft.svelte'
	import { notifyRestoredFromLocal } from '$lib/userDraftToast'
	import LocalDraftStaleModal from './common/confirmationModal/LocalDraftStaleModal.svelte'

	const dispatch = createEventDispatcher()

	type VariableState = {
		path: string
		variable: { value: string; is_secret: boolean; description: string }
		labels: string[] | undefined
		wsSpecific: boolean
	}

	let editPath: string | undefined = $state(undefined)

	// Per-workspace handles are driven by `useMany`. We track the workspace
	// IDs (and their seeded defaults) in a parallel `$state` array; on every
	// mutation `useMany` reconciles, acquiring entries for new workspaces and
	// releasing them on component teardown. `states` indexes the resulting
	// handles by workspace ID for ergonomic lookup downstream.
	let workspaceSpecs = $state<Array<{ ws: string; defaultValue: VariableState }>>([])
	let initialStates: Record<string, VariableState> = $state({})
	let existedInitially: Record<string, boolean> = $state({})
	let extraPerms: Record<string, Record<string, boolean>> = $state({})
	let perWsUser: Record<string, UserExt | undefined> = $state({})
	let selected: string | undefined = $state(undefined)
	let pathError = $state('')
	// Backend `edited_at` per workspace — the rev the staleness check
	// compares the local autosave's recorded rev against. Variables have
	// no DB-draft concept, so only `remoteRev` is ever populated.
	let fetchedRev: Record<string, string | undefined> = $state({})

	// Local-draft staleness modal: opened when the backend variable moved
	// on (someone else edited it) since the local autosave was written.
	let staleModalOpen = $state(false)
	let pendingStale: { ws: string; backend: VariableState } | undefined = undefined

	function onStaleLoadLatest(): void {
		if (!pendingStale) {
			staleModalOpen = false
			return
		}
		const { ws, backend } = pendingStale
		UserDraft.discard('variable', editPath ?? '', backend, { workspace: ws })
		initialStates[ws] = $state.snapshot(backend) as VariableState
		pendingStale = undefined
		staleModalOpen = false
	}

	function onStaleKeepDraft(): void {
		if (pendingStale) {
			const { ws } = pendingStale
			UserDraft.saveMeta(
				'variable',
				editPath ?? '',
				{ remoteRev: fetchedRev[ws] },
				{ workspace: ws }
			)
		}
		pendingStale = undefined
		staleModalOpen = false
	}

	const handlesArray = UserDraft.useMany<VariableState>(() =>
		workspaceSpecs.map((s) => ({
			itemKind: 'variable' as const,
			path: editPath ?? '',
			workspace: s.ws,
			defaultValue: s.defaultValue
		}))
	)
	const states = $derived.by(() => {
		const out: Record<string, UserDraftHandle<VariableState>> = {}
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
	function ensureHandle(ws: string, defaultValue: VariableState): void {
		if (workspaceSpecs.some((s) => s.ws === ws)) return
		workspaceSpecs.push({ ws, defaultValue })
	}

	let drawer: Drawer | undefined = $state()
	let form: VariableForm | undefined = $state()

	const deployTo = resource(
		() => selected,
		async (ws) =>
			ws ? (await WorkspaceService.getDeployTo({ workspace: ws })).deploy_to : undefined
	)

	const MAX_VARIABLE_LENGTH = 10000
	const edit = $derived(editPath !== undefined)
	const initialPath = $derived(editPath ?? '')
	const current = $derived(selected ? states[selected]?.draft : undefined)
	const can_write = $derived.by(() => {
		if (!selected || !edit) return true
		const perms = extraPerms[selected]
		if (!perms) return true
		return canWrite(editPath ?? '', perms, perWsUser[selected] ?? $userStore)
	})
	const dirtyWorkspaces = $derived(
		Object.keys(states).filter((ws) => !deepEqual(states[ws].draft, initialStates[ws]))
	)
	const anyDirty = $derived(dirtyWorkspaces.length > 0)
	const otherDirty = $derived(
		dirtyWorkspaces.length == 1
			? dirtyWorkspaces.filter((ws) => ws !== $workspaceStore)
			: dirtyWorkspaces
	)
	const dirtyValid = $derived(
		dirtyWorkspaces.every((ws) => {
			const v = states[ws].draft
			return !!v && v.variable.value.length <= MAX_VARIABLE_LENGTH
		})
	)
	const dirtyCanWrite = $derived(
		dirtyWorkspaces.every((ws) => {
			const perms = extraPerms[ws]
			return !perms || canWrite(editPath ?? '', perms, perWsUser[ws] ?? $userStore)
		})
	)

	// Lazy-fetch the variable for the selected workspace when not already cached
	$effect(() => {
		const ws = selected
		const p = editPath
		if (!ws || !p) return
		if (ws in states) return
		untrack(() => {
			Promise.all([
				VariableService.getVariable({ workspace: ws, path: p, decryptSecret: false }),
				getUserExt(ws)
			]).then(([v, user]) => {
				fetchedRev[ws] = v.edited_at
				const s: VariableState = {
					path: v.path,
					variable: {
						value: v.value ?? '',
						is_secret: v.is_secret,
						description: v.description ?? ''
					},
					labels: v.labels ?? undefined,
					wsSpecific: v.ws_specific ?? false
				}
				// See ResourceEditor for the same pattern: a backend that
				// moved on since the autosave was written → staleness modal;
				// otherwise just a "showing your local autosave" toast with
				// a "Reset to deployed" escape.
				const persisted = UserDraft.get<VariableState>('variable', p, { workspace: ws })
				const previousMeta = UserDraft.getMeta('variable', p, { workspace: ws })
				if (persisted !== undefined && !deepEqual(persisted, s)) {
					const cause = checkStaleness(previousMeta, v.edited_at)
					if (cause) {
						pendingStale = { ws, backend: s }
						staleModalOpen = true
					} else {
						if (previousMeta.remoteRev === undefined && previousMeta.remoteDraftRev === undefined) {
							UserDraft.saveMeta('variable', p, { remoteRev: v.edited_at }, { workspace: ws })
						}
						notifyRestoredFromLocal(false, true, {
							onResetToDeployed: () => {
								UserDraft.discard('variable', p, s, { workspace: ws })
							}
						})
					}
				}
				ensureHandle(ws, s)
				initialStates[ws] = structuredClone(s)
				existedInitially[ws] = true
				extraPerms[ws] = v.extra_perms ?? {}
				perWsUser[ws] = user
			})
		})
	})

	// Seed the staleness rev once a real autosave appears (see
	// ResourceEditor for the rationale). Self-limiting via the
	// meta-already-set guard.
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

	function reset() {
		// Clearing workspaceSpecs triggers useMany's reconcile to release
		// every acquired entry. The $derived `states` then collapses to {}.
		workspaceSpecs = []
		initialStates = {}
		existedInitially = {}
		extraPerms = {}
		perWsUser = {}
		pathError = ''
	}

	export function initNew(): void {
		reset()
		editPath = undefined
		const ws = $workspaceStore!
		const s: VariableState = {
			path: '',
			variable: { value: '', is_secret: true, description: '' },
			labels: undefined,
			wsSpecific: false
		}
		ensureHandle(ws, s)
		initialStates[ws] = structuredClone(s)
		existedInitially[ws] = false
		selected = ws
		drawer?.openDrawer()
	}

	export function editVariable(edit_path: string): void {
		reset()
		editPath = edit_path
		selected = $workspaceStore!
		drawer?.openDrawer()
	}

	async function loadSecret(): Promise<void> {
		if (!editPath || !selected) return
		const getV = await VariableService.getVariable({
			workspace: selected,
			path: editPath,
			decryptSecret: true
		})
		const s = states[selected]?.draft
		const ini = initialStates[selected]
		if (s) s.variable.value = getV.value ?? ''
		if (ini) ini.variable.value = getV.value ?? ''
		form?.setCode(getV.value ?? '')
	}

	async function save(): Promise<void> {
		const dirty = dirtyWorkspaces
		try {
			for (const ws of dirty) {
				const s = states[ws].draft!
				const ini = initialStates[ws]
				if (existedInitially[ws]) {
					await VariableService.updateVariable({
						workspace: ws,
						path: ini.path,
						requestBody: {
							path: ini.path != s.path ? s.path : undefined,
							value: s.variable.value == '' ? undefined : s.variable.value,
							is_secret:
								ini.variable.is_secret != s.variable.is_secret ? s.variable.is_secret : undefined,
							description:
								ini.variable.description != s.variable.description
									? s.variable.description
									: undefined,
							labels: s.labels,
							ws_specific: s.wsSpecific
						}
					})
				} else {
					await VariableService.createVariable({
						workspace: ws,
						requestBody: {
							path: s.path,
							value: s.variable.value,
							is_secret: s.variable.is_secret,
							description: s.variable.description,
							labels: s.labels,
							ws_specific: s.wsSpecific
						}
					})
				}
				// Saved on the backend — drop the local autosave for this workspace.
				UserDraft.remove('variable', editPath ?? '', { workspace: ws })
				// Path now exists server-side — drop the autocomplete cache so
				// it shows up immediately instead of after the 60s TTL.
				invalidateWorkspacePaths(ws)
			}
			sendUserToast(edit ? `Updated variable in ${dirty.length} workspace(s)` : `Created variable`)
			dispatch('create')
			drawer?.closeDrawer()
		} catch (err) {
			sendUserToast(`Could not save variable: ${err.body}`, true)
		}
	}
</script>

<LocalDraftStaleModal
	open={staleModalOpen}
	cause="version"
	onLoadLatest={onStaleLoadLatest}
	onKeepDraft={onStaleKeepDraft}
/>

<Drawer bind:this={drawer} size="50rem">
	<DrawerContent
		title={edit ? `Update variable at ${initialPath}` : 'Add a variable'}
		on:close={drawer?.closeDrawer}
	>
		<div class="flex flex-col gap-8">
			{#if !can_write}
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			{/if}

			{#if otherDirty.length > 0}
				<Alert type="warning" title="Editing multiple workspaces">
					You are going to edit the value in: {otherDirty.join(', ')}
				</Alert>
			{/if}

			{#if current}
				{#key current}
					<VariableForm
						bind:this={form}
						bind:path={current.path}
						bind:pathError
						bind:variable={current.variable}
						bind:labels={current.labels}
						bind:wsSpecific={current.wsSpecific}
						{initialPath}
						deployTo={deployTo.current}
						{can_write}
						{edit}
						onLoadSecret={loadSecret}
					/>
				{/key}
			{/if}
		</div>
		{#snippet actions()}
			{#if edit && $workspaceStore}
				<WsSpecificVersions
					kind="variable"
					workspaceId={$workspaceStore}
					{initialPath}
					bind:selected
				/>
			{/if}
			<Button
				on:click={save}
				disabled={!anyDirty || !dirtyValid || !dirtyCanWrite || pathError != ''}
				startIcon={{ icon: Save }}
				variant="accent"
				size="sm"
			>
				{edit ? 'Update' : 'Save'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
