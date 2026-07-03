<script lang="ts">
	import { page } from '$app/state'
	import {
		userWorkspaces,
		workspaceStore,
		globalForkModal,
		userStore,
		superadmin
	} from '$lib/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { workspaceAIClients } from '$lib/components/copilot/lib'
	import WorkspaceFamilyPicker from '$lib/components/sessions/WorkspaceFamilyPicker.svelte'
	import WorkspaceScopeTrigger from '$lib/components/WorkspaceScopeTrigger.svelte'
	import { findWorkspaceRoot, findWorkspaceDescendants } from '$lib/utils/workspaceHierarchy'

	let { isCollapsed = false }: { isCollapsed?: boolean } = $props()

	const effectiveId = $derived($workspaceStore ?? undefined)
	const currentWs = $derived(
		effectiveId ? $userWorkspaces.find((w) => w.id === effectiveId) : undefined
	)

	// Fork count surfaces the family's size right on the trigger, hinting that
	// the muted "root" chip is also the entry point to its forks.
	const forkCount = $derived.by(() => {
		const root = findWorkspaceRoot(effectiveId, $userWorkspaces)
		return root ? findWorkspaceDescendants(root.id, $userWorkspaces).length : 0
	})
	const rootLabel = $derived(
		forkCount > 0 ? `in root (${forkCount} fork${forkCount === 1 ? '' : 's'})` : 'in root'
	)

	// Settings link at the bottom of the picker — admin/superadmin only, scoped
	// to the active workspace (fork or root).
	const canManageWorkspace = $derived($userStore?.is_admin || $superadmin)
	const settingsHref = $derived(canManageWorkspace ? `${base}/workspace_settings` : undefined)
	const settingsLabel = $derived(`${currentWs?.name ?? effectiveId ?? 'Workspace'} settings`)

	function switchWorkspaceDirect(id: string) {
		if ($workspaceStore === id) return
		workspaceAIClients.init(id)
		switchWorkspace(id)
		// Item-scoped pages would show a wrong-workspace (or missing) item after
		// a workspace change — go home instead, mirroring the workspace picker.
		const editPages = [
			'/scripts/edit/',
			'/flows/edit/',
			'/apps/edit/',
			'/scripts/get/',
			'/flows/get/',
			'/apps/get/'
		]
		const isOnEditPage = editPages.some((p) => page.route.id?.includes(p) ?? false)
		if (isOnEditPage) {
			void goto('/')
		} else if (page.url.searchParams.get('workspace')) {
			page.url.searchParams.set('workspace', id)
		}
	}

	function openForkModal() {
		globalForkModal.val = { opened: true }
	}
</script>

<div class="flex items-center min-w-0 px-2 {isCollapsed ? 'justify-center' : ''} py-0.5 rounded-md">
	<WorkspaceFamilyPicker
		selectedId={effectiveId}
		onPick={switchWorkspaceDirect}
		onRequestCreateFork={openForkModal}
		{settingsHref}
		{settingsLabel}
		class="min-w-0 w-full"
	>
		{#snippet trigger()}
			<WorkspaceScopeTrigger
				workspaceId={effectiveId}
				{isCollapsed}
				{rootLabel}
				wrap
				class="w-full"
			/>
		{/snippet}
	</WorkspaceFamilyPicker>
</div>
