<script lang="ts">
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { goto } from '$lib/navigation'
	import { copilotInfo } from '$lib/aiStore'
	import AgentEditorModal from '$lib/components/flows/content/AgentEditorModal.svelte'
	import { openAgentEditor } from '$lib/components/flows/agentEditorStore.svelte'
	import { recentItems } from '$lib/components/home/recentItems.svelte'
	import { workspaceStore } from '$lib/stores'
	import { ResourceService } from '$lib/gen'

	let path = $derived(page.params.path ?? '')
	/** Set by `/agents/add` on the path it minted: a missing agent there is a new one, not an error. */
	let isNew = $derived(page.url.searchParams.get('new_draft') === 'true')

	$effect(() => {
		const target = { path, isNew }
		untrack(() => {
			openAgentEditor(target)
			if (!isNew) recordRecent(target.path)
		})
	})

	// The agent loads inside the editor, so it is read again here. Only a deployed agent is
	// recorded: a draft-only one 404s, and has no page for Home's card to open.
	async function recordRecent(agentPath: string) {
		const workspace = $workspaceStore
		if (!workspace) return
		try {
			const agent = await ResourceService.getResource({ workspace, path: agentPath })
			recentItems.record(workspace, {
				kind: 'agent',
				path: agentPath,
				summary: agent.description
			})
		} catch {}
	}
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
