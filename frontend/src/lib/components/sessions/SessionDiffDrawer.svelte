<script lang="ts">
	import WorkspaceDiffDrawer from './WorkspaceDiffDrawer.svelte'
	import { ArrowRight, GitFork, Pencil } from 'lucide-svelte'
	import { userWorkspaces } from '$lib/stores'
	import { useSessionDeployModel } from './sessionDeployModel.svelte'
	import type { DeployItem } from './sessionDeployModel'

	// Session "Edits" drawer. Builds the unified deploy model (drafts + fork
	// comparison) and hands it to WorkspaceDiffDrawer, which renders the tree +
	// scroll-through diff column. A non-fork session has no parent, so the model
	// runs in main (Draft → Parent) mode.
	let {
		workspaceId,
		parentWorkspaceId,
		chatId,
		keys,
		onDataChanged
	}: {
		workspaceId: string
		parentWorkspaceId?: string
		/** CHAT id (Session.chatId — the IndexedDB `chats` key, NOT the session id),
		 *  threaded to the footer's compare-page link as `from_session`: the compare
		 *  page resolves it via readChatModifiedItems to preselect this chat's edits. */
		chatId?: string
		keys?: Set<string>
		/** Notified after a deploy/discard mutated workspace state, so the owner
		 *  (session bar) can refresh its cached comparison/draft sources. */
		onDataChanged?: () => void
	} = $props()

	const isFork = $derived(!!parentWorkspaceId)
	const ws = $derived($userWorkspaces.find((w) => w.id === workspaceId))
	const parentWs = $derived(
		parentWorkspaceId ? $userWorkspaces.find((w) => w.id === parentWorkspaceId) : undefined
	)

	let inner: WorkspaceDiffDrawer | undefined = $state(undefined)

	const model = useSessionDeployModel(() => ({
		workspaceId,
		workspaceName: ws?.name ?? workspaceId,
		parentWorkspaceId,
		// The deploy target's name: the parent workspace for a fork; the session's
		// own workspace in main context (where a draft deploys in place).
		parentName: isFork ? (parentWs?.name ?? parentWorkspaceId) : (ws?.name ?? workspaceId),
		isFork,
		mask: keys,
		onDataChanged
	}))

	// Editor URL for a row: point at the workspace the item actually lives in
	// (parent-only rows link into the parent to avoid a 404 in the fork).
	function editUrlFor(item: DeployItem): string | undefined {
		const ws = encodeURIComponent(
			item.existsInFork ? workspaceId : (parentWorkspaceId ?? workspaceId)
		)
		const path = item.path
		if (item.deployKind === 'flow') return `/flows/edit/${path}?workspace=${ws}`
		if (item.deployKind === 'script') return `/scripts/edit/${path}?workspace=${ws}`
		if (item.deployKind === 'app') return `/apps/edit/${path}?workspace=${ws}`
		if (item.deployKind === 'raw_app') return `/apps_raw/edit/${path}?workspace=${ws}`
		return undefined
	}

	const title = 'Edits this session.'

	// Compare page = the batch/PR surface. The footer hands off THIS session's
	// edits via `from_session`, so the compare page opens preselected.
	const compareSessionHref = $derived(
		`/forks/compare?workspace_id=${encodeURIComponent(workspaceId)}&mode=${isFork ? 'fork' : 'draft'}` +
			(chatId ? `&from_session=${encodeURIComponent(chatId)}` : '')
	)

	export function open() {
		inner?.open()
	}
</script>

<WorkspaceDiffDrawer bind:this={inner} {model} {title} {editUrlFor} {compareSessionHref}>
	{#snippet titleExtra()}
		<div class="flex items-center gap-1.5 text-xs text-secondary min-w-0">
			{#if isFork}
				<GitFork class="w-3.5 h-3.5 shrink-0" />
				<span class="font-medium truncate" title={ws?.name ?? workspaceId}>
					{ws?.name ?? workspaceId}
				</span>
				<ArrowRight class="w-3 h-3 shrink-0 text-tertiary" />
				<span class="font-medium truncate" title={parentWs?.name ?? parentWorkspaceId}>
					{parentWs?.name ?? parentWorkspaceId}
				</span>
			{:else}
				<Pencil class="w-3.5 h-3.5 shrink-0" />
				<span class="font-medium truncate" title={ws?.name ?? workspaceId}>
					{ws?.name ?? workspaceId}
				</span>
			{/if}
		</div>
	{/snippet}
</WorkspaceDiffDrawer>
