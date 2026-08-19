<script lang="ts">
	import {
		userWorkspaces,
		workspaceStore,
		globalForkModal,
		userStore,
		superadmin
	} from '$lib/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { fixupUrlAfterWorkspaceSwitch } from './workspaceSwitchUrl'
	import { base } from '$lib/base'
	import { workspaceAIClients } from '$lib/components/copilot/lib'
	import WorkspaceFamilyPicker from '$lib/components/sessions/WorkspaceFamilyPicker.svelte'
	import WorkspaceScopeTrigger from '$lib/components/WorkspaceScopeTrigger.svelte'
	import {
		findWorkspaceRoot,
		findWorkspaceDescendants,
		isForkOwner
	} from '$lib/utils/workspaceHierarchy'
	import { useForkableWorkspaces } from '$lib/utils/useForkableWorkspaces.svelte'

	let { isCollapsed = false }: { isCollapsed?: boolean } = $props()

	const effectiveId = $derived($workspaceStore ?? undefined)
	// Resolve once here and pass to the picker + trigger below, avoiding a duplicate lookup (see
	// useForkableWorkspaces).
	const forkable = useForkableWorkspaces({
		workspaces: () => $userWorkspaces,
		currentWorkspaceId: () => effectiveId,
		isSuperadmin: () => !!$superadmin
	})
	const forkableWorkspaces = $derived(forkable.current)
	const currentWs = $derived(
		effectiveId ? forkableWorkspaces.find((w) => w.id === effectiveId) : undefined
	)

	// Fork count surfaces the family's size right on the trigger, hinting that
	// the muted "root" chip is also the entry point to its forks.
	const forkCount = $derived.by(() => {
		const root = findWorkspaceRoot(effectiveId, forkableWorkspaces)
		return root ? findWorkspaceDescendants(root.id, forkableWorkspaces).length : 0
	})
	const rootLabel = $derived(`${forkCount} fork${forkCount === 1 ? '' : 's'}`)

	// Settings link at the bottom of the picker — admins, superadmins, and fork
	// creators, scoped to the active workspace (fork or root).
	const canManageWorkspace = $derived(
		$userStore?.is_admin || $superadmin || isForkOwner(currentWs, $userStore?.email)
	)
	const settingsHref = $derived(canManageWorkspace ? `${base}/workspace_settings` : undefined)
	const settingsLabel = $derived(`${currentWs?.name ?? effectiveId ?? 'Workspace'} settings`)

	function switchWorkspaceDirect(id: string) {
		if ($workspaceStore === id) return
		workspaceAIClients.init(id)
		switchWorkspace(id)
		void fixupUrlAfterWorkspaceSwitch(id)
	}

	function openForkModal() {
		globalForkModal.val = { opened: true }
	}
</script>

<div class="flex items-center min-w-0 px-2 {isCollapsed ? 'justify-center' : ''} py-0.5 rounded-md">
	<WorkspaceFamilyPicker
		selectedId={effectiveId}
		{forkableWorkspaces}
		onPick={switchWorkspaceDirect}
		onRequestCreateFork={openForkModal}
		{settingsHref}
		{settingsLabel}
		class="min-w-0 w-full"
	>
		{#snippet trigger()}
			<WorkspaceScopeTrigger
				workspaceId={effectiveId}
				{forkableWorkspaces}
				{isCollapsed}
				{rootLabel}
				wrap
				class="w-full"
			/>
		{/snippet}
	</WorkspaceFamilyPicker>
</div>
