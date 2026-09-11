<script lang="ts">
	import { VariableService, WorkspaceService, type ListableVariable } from '$lib/gen'
	import { createEventDispatcher, untrack } from 'svelte'
	import { workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import OpenInSessionButton from './sessions/OpenInSessionButton.svelte'
	import {
		clearPageDrawerAnchor,
		pageDrawerSessionSource,
		setPageDrawerAnchor
	} from './sessions/pageDrawerSession'
	import { VARIABLES_PATH } from './sessions/previewPaths'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import { Save } from 'lucide-svelte'
	import VariableForm from './VariableForm.svelte'
	import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'
	import WsSpecificVersions from './WsSpecificVersions.svelte'
	import { resource } from 'runed'
	import { useActingUser } from '$lib/actingUser.svelte'
	import LocalDraftBanner from './LocalDraftBanner.svelte'
	import DraftConflictAlert from './DraftConflictAlert.svelte'
	import { isEncryptedDraftValue } from '$lib/encryptedDraft'
	import {
		isTemporaryPath,
		newItemPath,
		saveEach,
		useItems,
		type ItemAdapter,
		type ItemHandle
	} from '$lib/itemStore.svelte'

	const dispatch = createEventDispatcher()

	type VariableState = {
		path: string
		variable: { value: string; is_secret: boolean; description: string }
		labels: string[] | undefined
		wsSpecific: boolean
	}

	// The "current" workspace this editor defaults New/Edit actions to. Session
	// editors pass their acting workspace so secrets are created/updated there
	// rather than in the navigation workspace.
	let { workspace = undefined }: { workspace?: string } = $props()
	// Sole ambient read in this file: the acting workspace is an input, and only its
	// default comes from the navigation store.
	let curWs = $derived(workspace ?? $workspaceStore)

	let editPath: string | undefined = $state(undefined)
	/** The item open in the drawer: `editPath`, or a temporary path while creating one. */
	let itemPath: string | undefined = $state(undefined)
	let template: VariableState | undefined = undefined
	/** Bumped by every opening, so reopening an item reads it afresh. */
	let session = $state(0)
	// Every workspace the drawer has shown this item in (WsSpecificVersions re-points it).
	let workspaces = $state<string[]>([])
	let selected: string | undefined = $state(undefined)
	let pathError = $state('')
	const acting = useActingUser(() => selected)

	const MAX_VARIABLE_LENGTH = 10000

	function isValid(v: VariableState | undefined): boolean {
		// `$encrypted:` markers are ciphertext; the backend re-derives the real value on save,
		// so the length cap doesn't apply.
		return (
			!!v &&
			(isEncryptedDraftValue(v.variable.value) || v.variable.value.length <= MAX_VARIABLE_LENGTH)
		)
	}

	const variableAdapter: ItemAdapter<VariableState> = {
		async load({ workspace, path }) {
			const v = await VariableService.getVariable({
				workspace,
				path,
				decryptSecret: false,
				getDraft: true
			})
			const { draft, draft_saved_at, no_deployed } = v as any
			return {
				deployed: no_deployed
					? undefined
					: {
							path: v.path,
							variable: {
								value: v.value ?? '',
								is_secret: v.is_secret,
								description: v.description ?? ''
							},
							labels: v.labels ?? undefined,
							wsSpecific: v.ws_specific ?? false
						},
				// `.draft` already holds the editor's `VariableState` shape.
				draft: draft as VariableState | undefined,
				draftSavedAt: draft_saved_at,
				meta: v
			}
		},
		async write({ workspace, path, value: s, deployed: ini }) {
			if (ini) {
				await VariableService.updateVariable({
					workspace,
					path,
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
					workspace,
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
			// Path now exists server-side — drop the autocomplete cache so
			// it shows up immediately instead of after the 60s TTL.
			invalidateWorkspacePaths(workspace)
		}
	}

	const handles = useItems<VariableState>(
		'variable',
		() =>
			workspaces.map((ws) => ({
				workspace: ws,
				path: itemPath,
				template,
				session,
				valid: () => isValid(items[ws]?.value),
				writable: () => {
					const perms = extraPermsOf(ws)
					return !perms || canWrite(editPath ?? '', perms, acting.in(ws))
				}
			})),
		variableAdapter
	)
	const items = $derived.by(() => {
		const out: Record<string, ItemHandle<VariableState>> = {}
		workspaces.forEach((ws, i) => {
			const handle = handles[i]
			if (handle) out[ws] = handle
		})
		return out
	})

	function extraPermsOf(ws: string): Record<string, boolean> | undefined {
		const meta = items[ws]?.meta as ListableVariable | undefined
		return meta ? (meta.extra_perms ?? {}) : undefined
	}

	// Open the selected workspace's version of the item alongside the others.
	$effect(() => {
		const ws = selected
		if (!ws || !itemPath) return
		untrack(() => {
			if (!workspaces.includes(ws)) workspaces.push(ws)
		})
	})

	let drawer: Drawer | undefined = $state()

	const deployTo = resource(
		() => selected,
		async (ws) =>
			ws ? (await WorkspaceService.getDeployTo({ workspace: ws })).deploy_to : undefined
	)

	// `selected`, not `curWs`: WsSpecificVersions re-points this drawer at another
	// workspace's version of the variable, and the session must act on the one the
	// user is looking at.
	const sessionSource = $derived(
		pageDrawerSessionSource(VARIABLES_PATH, editPath, selected ?? curWs)
	)
	const selectedItem = $derived(selected ? items[selected] : undefined)
	// A variable this drawer created is edited from then on: the drawer stays open on it when an
	// edit was typed during the create.
	const edit = $derived(editPath !== undefined || selectedItem?.origin === 'deployed')
	const initialPath = $derived(
		editPath ??
			(selectedItem && !isTemporaryPath(selectedItem.key.path) ? selectedItem.key.path : '')
	)
	const current = $derived(selectedItem?.value)
	// `undefined` until the selected workspace's permissions and acting user have both
	// landed — a pending verdict is neither a grant nor the denial the read-only alert
	// announces, so the two must stay distinguishable.
	const can_write: boolean | undefined = $derived.by(() => {
		// Nobody else's permissions apply to a variable this drawer is creating, or just created.
		if (!selected || editPath === undefined) return true
		const perms = extraPermsOf(selected)
		if (!perms || !acting.resolved(selected)) return undefined
		return canWrite(editPath ?? '', perms, acting.in(selected))
	})
	const dirtyWorkspaces = $derived(Object.keys(items).filter((ws) => items[ws].dirty))
	const anyDirty = $derived(dirtyWorkspaces.length > 0)
	const anyBusy = $derived(Object.values(items).some((it) => it.busy))
	// Banner is scoped to the selected workspace — the diff/discard only
	// operate on it, so showing it for an unrelated dirty workspace would be
	// misleading. The cross-workspace `otherDirty` alert below still covers
	// that case.
	const selectedDirty = $derived(!!selectedItem?.dirty)
	const otherDirty = $derived(
		dirtyWorkspaces.length == 1 ? dirtyWorkspaces.filter((ws) => ws !== curWs) : dirtyWorkspaces
	)
	const canSave = $derived(
		anyDirty && dirtyWorkspaces.every((ws) => items[ws].canSave) && pathError == ''
	)

	// Set by Save and cleared by opening anything: closes the drawer once the items on screen
	// have landed with nothing left unsaved, so an edit typed during the write stays in front
	// of you.
	let closeOnSettle = $state(false)
	$effect(() => {
		if (!closeOnSettle || anyBusy) return
		untrack(() => {
			closeOnSettle = false
			const shown = Object.values(items)
			if (shown.every((it) => it.status === 'idle' && !it.dirty)) drawer?.closeDrawer()
		})
	})

	// A draft-only variable is gone once its draft is discarded.
	$effect(() => {
		if (!selectedItem?.removed) return
		untrack(() => {
			dispatch('create')
			drawer?.closeDrawer()
		})
	})

	function open(path: string, ws: string): void {
		closeOnSettle = false
		pathError = ''
		acting.forgetFailures()
		itemPath = path
		workspaces = [ws]
		selected = ws
		session++
		drawer?.openDrawer()
	}

	export function initNew(): void {
		editPath = undefined
		template = {
			path: '',
			variable: { value: '', is_secret: true, description: '' },
			labels: undefined,
			wsSpecific: false
		}
		open(newItemPath(), curWs!)
	}

	export function editVariable(edit_path: string): void {
		editPath = edit_path
		template = undefined
		open(edit_path, curWs!)
		setPageDrawerAnchor(VARIABLES_PATH, edit_path)
	}

	function loadSecret(): void {
		void selectedItem?.learn(async ({ workspace, path }) => {
			const v = await VariableService.getVariable({ workspace, path, decryptSecret: true })
			return (side) => {
				side.variable.value = v.value ?? ''
			}
		})
	}

	async function save(): Promise<void> {
		const targets = dirtyWorkspaces.map((ws) => items[ws])
		const updated = edit
		closeOnSettle = true
		const outcomes = await saveEach(targets)
		const failed = outcomes.find((o) => !o.ok && !o.skipped)
		if (failed && !failed.ok) {
			sendUserToast(`Could not save variable: ${failed.error}`, true)
			return
		}
		sendUserToast(
			updated ? `Updated variable in ${targets.length} workspace(s)` : `Created variable`
		)
		dispatch('create')
	}
</script>

<Drawer bind:this={drawer} size="50rem" on:close={() => clearPageDrawerAnchor(VARIABLES_PATH)}>
	<DrawerContent
		title={edit ? `Update variable at ${initialPath}` : 'Add a variable'}
		bannerReserved={edit}
		on:close={drawer?.closeDrawer}
	>
		{#snippet banner()}
			<LocalDraftBanner
				show={edit && selectedDirty}
				reserveSpace={edit}
				getDeployed={() => selectedItem?.deployed}
				getCurrent={() => current}
				onDiscard={async () => void (await selectedItem?.discard())}
				disabled={!can_write}
			/>
		{/snippet}
		<div class="flex flex-col gap-8 pb-2">
			{#if can_write === false}
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			{/if}

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

			<!-- Held back until there is a verdict: rendering the form against a pending `can_write`
			would flash read-only controls at someone who can in fact write. -->
			{#if current && can_write !== undefined}
				{#key current}
					<VariableForm
						bind:path={current.path}
						bind:pathError
						bind:variable={current.variable}
						bind:labels={current.labels}
						bind:wsSpecific={current.wsSpecific}
						{initialPath}
						deployTo={deployTo.current}
						can_write={can_write === true}
						{edit}
						onLoadSecret={loadSecret}
						workspace={selected}
						actingUser={acting.in(selected) ?? null}
					/>
				{/key}
			{/if}
		</div>
		{#snippet actions()}
			<OpenInSessionButton source={sessionSource} />
			{#if edit && curWs}
				<WsSpecificVersions kind="variable" workspaceId={curWs} {initialPath} bind:selected />
			{/if}
			<Button
				on:click={save}
				disabled={!canSave}
				startIcon={{ icon: Save }}
				variant="accent"
				size="sm"
			>
				{edit ? 'Update' : 'Save'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
