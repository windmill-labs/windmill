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
	import { getUserExt } from '$lib/user'
	import type { UserExt } from '$lib/stores'
	import { UserDraft, draftValuesEqual, type UserDraftHandle } from '$lib/userDraft.svelte'
	import LocalDraftBanner from './LocalDraftBanner.svelte'
	import { isEncryptedDraftValue } from '$lib/encryptedDraft'
	import { setLocalDraftHint } from '$lib/localDraftHints.svelte'

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

	const handlesArray = UserDraft.useMany<VariableState>(() =>
		workspaceSpecs.map((s) => ({
			itemKind: 'variable' as const,
			path: editPath ?? '',
			workspace: s.ws,
			defaultValue: s.defaultValue,
			// Autosaves landing back on the deployed value become deletes (same
			// comparison as the banner's `dirtyWorkspaces`, so they can't disagree).
			// Guarded by `existedInitially` so draft-only items aren't destroyed.
			discardIf: (val) => !!existedInitially[s.ws] && draftValuesEqual(val, initialStates[s.ws])
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
		Object.keys(states).filter((ws) => !draftValuesEqual(states[ws].draft, initialStates[ws]))
	)

	// The list-page `*` hint is owned by UserDraftDbSyncer (set on save, cleared
	// on delete). The editor only CLEARS it — a workspace at the deployed
	// baseline has no draft, so drop any stale hint (this is how a draft
	// discarded in another tab vanishes on reopen). Never SET here.
	$effect(() => {
		const p = editPath
		const loadedWs = Object.keys(states)
		const dirty = dirtyWorkspaces
		untrack(() => {
			if (!p) return
			for (const ws of loadedWs) {
				if (!dirty.includes(ws)) setLocalDraftHint(ws, 'variable', p, false)
			}
		})
	})
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
	const dirtyValid = $derived(
		dirtyWorkspaces.every((ws) => {
			const v = states[ws].draft
			// `$encrypted:` markers are ciphertext; the backend re-derives the
			// real value on save, so the length cap doesn't apply.
			return (
				!!v &&
				(isEncryptedDraftValue(v.variable.value) || v.variable.value.length <= MAX_VARIABLE_LENGTH)
			)
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
				VariableService.getVariable({
					workspace: ws,
					path: p,
					decryptSecret: false,
					getDraft: true
				}),
				getUserExt(ws)
			]).then(([v, user]) => {
				// `.draft` already holds the editor's `VariableState` shape.
				const savedDraftState = (v as any).draft as VariableState | undefined
				// Deployed baseline as the dirty-check reference, so the banner
				// compares draft-vs-deployed and fires immediately when a draft exists.
				const deployedState: VariableState = {
					path: v.path,
					variable: {
						value: v.value ?? '',
						is_secret: v.is_secret,
						description: v.description ?? ''
					},
					labels: v.labels ?? undefined,
					wsSpecific: v.ws_specific ?? false
				}
				// Open with the saved draft if present, else the deployed.
				const s: VariableState = savedDraftState ?? deployedState
				ensureHandle(ws, s)
				initialStates[ws] = structuredClone(deployedState)
				// Draft-only paths (`no_deployed`) have no row — saving must
				// CREATE, not update (update 404s).
				existedInitially[ws] = !(v as any).no_deployed
				extraPerms[ws] = v.extra_perms ?? {}
				perWsUser[ws] = user
			})
		})
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
				// The just-saved state is the new deployed baseline; reset the
				// handle to it via `discard` (not `remove` — blanking the cell to
				// `undefined` reads as dirty). The `value: null` POST also deletes
				// the server draft row so `is_draft` clears on refetch.
				initialStates[ws] = $state.snapshot(s) as VariableState
				existedInitially[ws] = true
				UserDraft.discard('variable', editPath ?? '', s, { workspace: ws })
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

<Drawer bind:this={drawer} size="50rem">
	<DrawerContent
		title={edit ? `Update variable at ${initialPath}` : 'Add a variable'}
		bannerReserved={edit}
		on:close={drawer?.closeDrawer}
	>
		{#snippet banner()}
			<LocalDraftBanner
				show={edit && selectedDirty}
				reserveSpace={edit}
				getDeployed={() => (selected ? initialStates[selected] : undefined)}
				getCurrent={() => current}
				onDiscard={() => {
					if (!selected) return
					UserDraft.discard('variable', editPath ?? '', initialStates[selected], {
						workspace: selected
					})
				}}
				disabled={!can_write}
			/>
		{/snippet}
		<div class="flex flex-col gap-8 pb-2">
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
