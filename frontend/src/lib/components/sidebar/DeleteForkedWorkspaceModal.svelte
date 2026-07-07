<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { clearStores, switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$lib/navigation'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { findWorkspaceDescendants, workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import {
		deleteSessionsForWorkspace,
		reconcileAfterWorkspaceChange
	} from '$lib/components/sessions/sessionState.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	type ForkedDatatable = {
		name: string
		resourceType: string
		resourcePath: string
		dropOnDelete: boolean
	}

	let open = $state(false)
	let forkedDatatables: ForkedDatatable[] = $state([])
	let deleteForkedChildren = $state(false)

	const forkedDescendants = $derived(
		$workspaceStore ? findWorkspaceDescendants($workspaceStore, $userWorkspaces ?? []) : []
	)
	// Fork/dev workspaces are detected by their parent link, not the `wm-fork-` id prefix.
	const currentWsIsFork = $derived(workspaceIsFork($workspaceStore, $userWorkspaces ?? []))

	async function loadForkedDatatables() {
		if (!$workspaceStore) return
		try {
			const settings = await WorkspaceService.getPublicSettings({ workspace: $workspaceStore })
			const datatables = settings.datatable?.datatables ?? {}
			forkedDatatables = Object.entries(datatables)
				.filter(([_, dt]) => dt.forked_from != null)
				.map(([name, dt]) => ({
					name,
					resourceType: dt.database.resource_type ?? 'instance',
					resourcePath: dt.database.resource_path ?? '',
					dropOnDelete: true
				}))
		} catch {
			forkedDatatables = []
		}
	}

	// Load the fork's datatables, then open the confirm modal. The caller only
	// surfaces the affordance for forked workspaces; the modal also self-guards
	// its markup on `currentWsIsFork` below.
	export async function openModal() {
		await loadForkedDatatables()
		deleteForkedChildren = false
		open = true
	}

	async function deleteFork() {
		const workspace = $workspaceStore ?? ''
		// Capture the parent before delete so we can land the user there
		// instead of dropping them back on the workspace-picker menu.
		// Only valid if the parent is still in the user's workspace list.
		const parentId = $userWorkspaces.find((w) => w.id === workspace)?.parent_workspace_id
		const parentStillAccessible = !!(parentId && $userWorkspaces.find((w) => w.id === parentId))
		const dbsToDrop = forkedDatatables.filter((dt) => dt.dropOnDelete).map((dt) => dt.name)

		if (dbsToDrop.length > 0) {
			const errors = await WorkspaceService.dropForkedDatatableDatabases({
				workspace,
				requestBody: { datatable_names: dbsToDrop }
			})
			for (const err of errors) {
				sendUserToast(err, true)
			}
		}

		// Fork-scoped ducklake namespaces (metadata schemas + data files) — driven by the
		// backend registry, so no per-lake selection is needed. Best-effort: a failure is
		// surfaced but doesn't block the workspace delete (the registry rows survive a
		// failed cleanup and re-creating the same fork id retries it — the toast tells the
		// user something was left behind).
		try {
			const dlErrors = await WorkspaceService.dropForkedDucklakeNamespaces({ workspace })
			for (const err of dlErrors) {
				sendUserToast(err, true)
			}
		} catch (err) {
			sendUserToast(`Failed to drop fork ducklake namespaces: ${err}`, true)
		}

		if (deleteForkedChildren && forkedDescendants.length > 0) {
			for (const child of forkedDescendants) {
				try {
					await WorkspaceService.dropForkedDucklakeNamespaces({ workspace: child.id }).then(
						(errs) => errs.forEach((e) => sendUserToast(e, true)),
						(err) =>
							sendUserToast(`Failed to drop fork ducklake namespaces of ${child.id}: ${err}`, true)
					)
					await WorkspaceService.deleteWorkspace({ workspace: child.id })
				} catch (err) {
					sendUserToast(`Failed to delete forked child ${child.id}: ${err}`, true)
					return
				}
				// Backend delete is authoritative; session cleanup is best-effort so a
				// local failure can't abort the remaining (parent) deletes.
				await deleteSessionsForWorkspace(child.id).catch((e) =>
					console.error(`Session cleanup for ${child.id} failed`, e)
				)
			}
		}

		await WorkspaceService.deleteWorkspace({ workspace })
		await deleteSessionsForWorkspace(workspace).catch((e) =>
			console.error('Session cleanup after workspace delete failed', e)
		)
		sendUserToast('You deleted the workspace')
		if (parentStillAccessible && parentId) {
			// Refresh the workspace list AND reconcile session lifecycle before
			// landing on the parent. The refresh keeps the sidebar's
			// `visibleSessions` filter from rendering every committed session as
			// "Fork — no longer available" (it reads `$userWorkspaces`, which
			// `clearStores()` would null). The reconcile re-roots surviving child
			// forks: deleting this fork without "delete children" re-parents them
			// via the backend's ON DELETE SET NULL, so their sessions' stored
			// `workspace_root_id` must be recomputed off the now-deleted ancestor.
			// A reconcile failure must NOT strand the user on the just-deleted
			// workspace — always fall through to the parent switch + navigation.
			try {
				await reconcileAfterWorkspaceChange()
			} catch (e) {
				console.error('Failed to reconcile sessions after workspace delete', e)
			}
			switchWorkspace(parentId)
			await goto('/')
		} else {
			clearStores()
			await goto('/user/workspaces')
		}
	}
</script>

{#if currentWsIsFork}
	<ConfirmationModal
		{open}
		title="Delete forked workspace"
		confirmationText="Remove"
		on:canceled={() => {
			open = false
		}}
		on:confirmed={() => {
			open = false
			deleteFork()
		}}
	>
		<div class="flex flex-col w-full space-y-4">
			<span>Are you sure you want to delete this workspace fork? (deleting {$workspaceStore})</span>
			{#if forkedDescendants.length > 0}
				<div class="border rounded-md divide-y">
					<div class="px-4 py-2 flex items-center justify-between gap-2">
						<div class="flex flex-col min-w-0">
							<span class="text-xs font-semibold text-secondary">Forked children</span>
							<span class="text-3xs text-hint">
								This fork has {forkedDescendants.length} forked
								{forkedDescendants.length === 1 ? 'child' : 'children'} (transitively).
							</span>
						</div>
						<Toggle
							class="shrink-0"
							size="xs"
							bind:checked={deleteForkedChildren}
							options={{ right: 'Also delete children' }}
						/>
					</div>
					<ul class="px-4 py-2 text-3xs text-hint max-h-32 overflow-y-auto">
						{#each forkedDescendants as child}
							<li class="font-mono truncate" title={child.id}>{child.id}</li>
						{/each}
					</ul>
				</div>
			{/if}
			{#if forkedDatatables.length > 0}
				<div class="border rounded-md divide-y">
					<div class="px-4 py-2 text-xs font-semibold text-secondary"> Forked databases </div>
					{#each forkedDatatables as dt}
						<div class="flex items-center justify-between px-4 py-2">
							<div class="flex flex-col">
								<span class="text-xs font-medium text-secondary">{dt.name}</span>
								<span class="text-3xs text-hint">
									{dt.resourceType === 'instance'
										? dt.resourcePath
										: `${$workspaceStore?.replace(/-/g, '_')}__${dt.name}`}
								</span>
							</div>
							<Toggle
								class="shrink-0"
								size="xs"
								bind:checked={dt.dropOnDelete}
								options={{ right: 'Drop database' }}
							/>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</ConfirmationModal>
{/if}
