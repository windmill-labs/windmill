<script lang="ts">
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { goto } from '$lib/navigation'
	import { copilotInfo } from '$lib/aiStore'
	import AgentEditorModal from '$lib/components/flows/content/AgentEditorModal.svelte'
	import { openAgentEditor } from '$lib/components/flows/agentEditorStore.svelte'
	import { recentItems } from '$lib/components/home/recentItems.svelte'
	import { workspaceStore } from '$lib/stores'

	let path = $derived(page.params.path ?? '')
	/** Set by `/agents/add` on the path it minted: a missing agent there is a new one, not an error. */
	let isNew = $derived(page.url.searchParams.get('new_draft') === 'true')

	$effect(() => {
		const target = { path, isNew }
		untrack(() => {
			openAgentEditor(target)
			// The agent loads inside the editor, so it is recorded from the route, without a
			// summary: its Home card shows the path. A new one has no page to open yet.
			if (!isNew && $workspaceStore) recentItems.record($workspaceStore, { kind: 'agent', path })
		})
	})
</script>

<div class="h-full w-full">
	<AgentEditorModal
		layout="page"
		enableAi={$copilotInfo.enabled}
		owns={(t) => t.host === undefined}
		startInSettings={page.url.searchParams.get('settings') === 'true'}
		onClose={(deployed) => goto(deployed ? `/agents/get/${path}` : `/?kind=agent`)}
		onDeployed={(saved) =>
			// Replaced, not pushed, when this URL stopped naming what it opens: Back would otherwise
			// reopen `new_draft` on an agent that now exists, or a path a rename just moved away from.
			goto(`/agents/get/${saved}`, { replaceState: isNew || saved !== path })}
	/>
</div>
