<script lang="ts">
	import { page } from '$app/state'
	import {
		userWorkspaces,
		workspaceStore,
		globalForkModal,
		userStore,
		superadmin,
		type UserWorkspace
	} from '$lib/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { workspaceAIClients } from '$lib/components/copilot/lib'
	import WorkspaceFamilyPicker from '$lib/components/sessions/WorkspaceFamilyPicker.svelte'
	import { Button } from '$lib/components/common'
	import { ChevronDown, GitFork } from 'lucide-svelte'

	let { isCollapsed = false }: { isCollapsed?: boolean } = $props()

	function findRoot(id: string | undefined, all: UserWorkspace[]): UserWorkspace | undefined {
		if (!id) return undefined
		let current = all.find((w) => w.id === id)
		while (current?.parent_workspace_id) {
			const parent = all.find((w) => w.id === current!.parent_workspace_id)
			if (!parent) break
			current = parent
		}
		return current
	}

	const effectiveId = $derived($workspaceStore ?? undefined)
	const root = $derived(findRoot(effectiveId, $userWorkspaces))
	const currentWs = $derived(
		effectiveId ? $userWorkspaces.find((w) => w.id === effectiveId) : undefined
	)
	const isFork = $derived(!!currentWs && !!root && currentWs.id !== root.id)
	const forkName = $derived(currentWs?.name ?? effectiveId)

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
		// A session is scoped to its (forked) workspace, so switching forks while
		// viewing one must leave it too.
		const editPages = [
			'/scripts/edit/',
			'/flows/edit/',
			'/apps/edit/',
			'/scripts/get/',
			'/flows/get/',
			'/apps/get/'
		]
		const isOnEditPage = editPages.some((p) => page.route.id?.includes(p) ?? false)
		const isOnSessionPage = page.route.id?.includes('/sessions') ?? false
		if (isOnEditPage || isOnSessionPage) {
			void goto('/')
		} else if (page.url.searchParams.get('workspace')) {
			page.url.searchParams.set('workspace', id)
		}
	}

	function openForkModal() {
		globalForkModal.val = { opened: true }
	}
</script>

<div
	class="flex items-center min-w-0 {isCollapsed ? 'justify-center px-0' : 'px-2'} py-1 rounded-md"
>
	<WorkspaceFamilyPicker
		selectedId={effectiveId}
		onPick={switchWorkspaceDirect}
		onRequestCreateFork={openForkModal}
		{settingsHref}
		{settingsLabel}
		class="min-w-0 w-full"
	>
		{#snippet trigger()}
			<Button
				variant="default"
				unifiedSize="sm"
				title={isFork ? `Fork: ${forkName}` : 'Workspace: root'}
				startIcon={isFork || isCollapsed ? { icon: GitFork } : undefined}
				endIcon={!isCollapsed ? { icon: ChevronDown } : undefined}
				wrapperClasses="w-full"
				btnClasses="min-w-0 w-full {isCollapsed ? 'justify-center' : 'justify-start'} {isFork
					? '!text-accent !border-accent/30 font-semibold'
					: ''}"
			>
				{#if !isCollapsed}
					<span class="truncate min-w-0 flex-1 text-left">
						<span class="{isFork ? 'text-accent/80' : 'text-tertiary'} font-normal"
							>{isFork ? 'Fork:' : 'Workspace:'}</span
						>
						{isFork ? forkName : 'root'}
					</span>
				{/if}
			</Button>
		{/snippet}
	</WorkspaceFamilyPicker>
</div>
