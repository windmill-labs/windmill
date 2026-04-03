<script lang="ts">
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { GitForkIcon } from 'lucide-svelte'
	import CreateWorkspaceInner from './CreateWorkspaceInner.svelte'
	import { workspaceStore } from '$lib/stores'
	import { goto } from '$app/navigation'
	import { page } from '$app/state'

	interface Props {
		isFork?: boolean
	}

	let { isFork = false }: Props = $props()
	const rd = page.url.searchParams.get('rd')
</script>

<CenteredModal title="{isFork ? 'Forking' : 'New'} Workspace" centerVertically={false}>
	{#if isFork}
		<div class="flex flex-block gap-2">
			<GitForkIcon size={16} />
			<span class="text-xs text-normal">Forking </span>
			<span class="text-xs text-emphasis font-semibold">
				{$workspaceStore}
			</span>
		</div>
	{/if}
	<CreateWorkspaceInner {isFork} onFinish={() => goto(rd ?? '/')} />
</CenteredModal>
