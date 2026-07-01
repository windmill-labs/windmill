<script lang="ts">
	import WorkspaceDiffDrawer from './WorkspaceDiffDrawer.svelte'
	import { ArrowRight, GitFork, Pencil } from 'lucide-svelte'
	import { userWorkspaces } from '$lib/stores'
	import { useSessionDeployModel } from './sessionDeployModel.svelte'
	import type { DeployItem } from './sessionDeployModel'

	// Session Review & Deploy drawer. Builds the unified deploy model (drafts +
	// fork comparison) and hands it to WorkspaceDiffDrawer, which renders the tree
	// + scroll-through diff column. A non-fork session has no parent, so the model
	// runs in main (Draft → Parent) mode.
	let {
		workspaceId,
		parentWorkspaceId,
		keys,
		chatId
	}: {
		workspaceId: string
		parentWorkspaceId?: string
		keys?: Set<string>
		chatId?: string
	} = $props()

	const isFork = $derived(!!parentWorkspaceId)
	const ws = $derived($userWorkspaces.find((w) => w.id === workspaceId))
	const parentWs = $derived(
		parentWorkspaceId ? $userWorkspaces.find((w) => w.id === parentWorkspaceId) : undefined
	)

	let inner: WorkspaceDiffDrawer | undefined = $state(undefined)

	const model = useSessionDeployModel(() => ({
		workspaceId,
		parentWorkspaceId,
		parentName: parentWs?.name ?? parentWorkspaceId,
		isFork,
		mask: keys
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

	const title = $derived(isFork ? '' : 'Drafts')
	const reviewBase = $derived(
		`/forks/compare?workspace_id=${encodeURIComponent(workspaceId)}${
			chatId ? `&from_session=${encodeURIComponent(chatId)}` : ''
		}`
	)
	const reviewHref = $derived(isFork ? reviewBase : `${reviewBase}&mode=draft`)

	export function open() {
		inner?.open()
	}
</script>

<WorkspaceDiffDrawer bind:this={inner} {model} {title} {reviewHref} {editUrlFor}>
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
