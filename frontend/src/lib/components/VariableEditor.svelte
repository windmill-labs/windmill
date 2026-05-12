<script lang="ts">
	import { VariableService, WorkspaceService } from '$lib/gen'
	import { createEventDispatcher, untrack } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite, readFieldsRecursively } from '$lib/utils'
	import { Save } from 'lucide-svelte'
	import VariableForm from './VariableForm.svelte'
	import WsSpecificVersions from './WsSpecificVersions.svelte'
	import { resource } from 'runed'
	import { deepEqual } from 'fast-equals'
	import { getUserExt } from '$lib/user'
	import type { UserExt } from '$lib/stores'
	import { UserDraft } from '$lib/userDraft.svelte'

	const dispatch = createEventDispatcher()

	type VariableState = {
		path: string
		variable: { value: string; is_secret: boolean; description: string }
		labels: string[] | undefined
		wsSpecific: boolean
	}

	let editPath: string | undefined = $state(undefined)

	let states: Record<string, VariableState> = $state({})
	let initialStates: Record<string, VariableState> = $state({})
	let existedInitially: Record<string, boolean> = $state({})
	let extraPerms: Record<string, Record<string, boolean>> = $state({})
	let perWsUser: Record<string, UserExt | undefined> = $state({})
	let selected: string | undefined = $state(undefined)
	let pathError = $state('')

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
	const current = $derived(selected ? states[selected] : undefined)
	const can_write = $derived.by(() => {
		if (!selected || !edit) return true
		const perms = extraPerms[selected]
		if (!perms) return true
		return canWrite(editPath ?? '', perms, perWsUser[selected] ?? $userStore)
	})
	const dirtyWorkspaces = $derived(
		Object.keys(states).filter((ws) => !deepEqual(states[ws], initialStates[ws]))
	)
	const anyDirty = $derived(dirtyWorkspaces.length > 0)
	const otherDirty = $derived(
		dirtyWorkspaces.length == 1
			? dirtyWorkspaces.filter((ws) => ws !== $workspaceStore)
			: dirtyWorkspaces
	)
	const dirtyValid = $derived(
		dirtyWorkspaces.every((ws) => states[ws].variable.value.length <= MAX_VARIABLE_LENGTH)
	)
	const dirtyCanWrite = $derived(
		dirtyWorkspaces.every((ws) => {
			const perms = extraPerms[ws]
			return !perms || canWrite(editPath ?? '', perms, perWsUser[ws] ?? $userStore)
		})
	)

	// The local autosave is the entire multi-workspace `states` bundle, so
	// cross-workspace edits in the same drawer session are preserved across
	// refresh. UserDraft keys on the user's session workspace regardless of
	// which target workspace each state entry belongs to.
	type VariableDraft = Record<string, VariableState>

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
				const backendState: VariableState = {
					path: v.path,
					variable: {
						value: v.value ?? '',
						is_secret: v.is_secret,
						description: v.description ?? ''
					},
					labels: v.labels ?? undefined,
					wsSpecific: v.ws_specific ?? false
				}
				const localBundle = UserDraft.get<VariableDraft>('variable', p)
				const localState = localBundle?.[ws]
				const useLocal = localState && !deepEqual(localState, backendState)
				states[ws] = useLocal ? structuredClone(localState) : backendState
				initialStates[ws] = structuredClone(backendState)
				existedInitially[ws] = true
				extraPerms[ws] = v.extra_perms ?? {}
				perWsUser[ws] = user
			})
		})
	})

	// Persist the full states bundle on every mutation. Empty path (new
	// variable) stays in-memory only; UserDraft.save no-ops there.
	$effect(() => {
		if (Object.keys(states).length === 0) return
		readFieldsRecursively(states)
		UserDraft.save('variable', editPath ?? '', states)
	})

	function reset() {
		states = {}
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
		// Empty-path drafts live in-memory only; rehydrate any in-memory
		// bundle if one already exists on this page, including states
		// staged for other target workspaces.
		const localBundle = UserDraft.get<VariableDraft>('variable', '')
		const baseState: VariableState = {
			path: '',
			variable: { value: '', is_secret: true, description: '' },
			labels: undefined,
			wsSpecific: false
		}
		const localFor = (w: string): VariableState | undefined => localBundle?.[w]

		const localOwn = localFor(ws)
		states[ws] = localOwn ? structuredClone(localOwn) : baseState
		initialStates[ws] = structuredClone(baseState)
		existedInitially[ws] = false
		if (localBundle) {
			for (const w of Object.keys(localBundle)) {
				if (w === ws) continue
				states[w] = structuredClone(localBundle[w])
				initialStates[w] = structuredClone(baseState)
				existedInitially[w] = false
			}
		}
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
		const s = states[selected]
		const ini = initialStates[selected]
		if (s) s.variable.value = getV.value ?? ''
		if (ini) ini.variable.value = getV.value ?? ''
		form?.setCode(getV.value ?? '')
	}

	async function save(): Promise<void> {
		const dirty = dirtyWorkspaces
		try {
			for (const ws of dirty) {
				const s = states[ws]
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
			}
			sendUserToast(edit ? `Updated variable in ${dirty.length} workspace(s)` : `Created variable`)
			UserDraft.remove('variable', editPath ?? '')
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
