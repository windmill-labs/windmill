<script lang="ts">
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { copilotInfo } from '$lib/aiStore'
	import AgentEditorModal from '$lib/components/flows/content/AgentEditorModal.svelte'
	import { openAgentEditor } from '$lib/components/flows/agentEditorStore.svelte'

	let path = $derived(page.params.path ?? '')
	/** Set by `/agents/add` on the path it minted: a missing agent there is a new one, not an error. */
	let isNew = $derived(page.url.searchParams.get('new_draft') === 'true')

	$effect(() => {
		const target = { path, isNew }
		untrack(() => openAgentEditor(target))
	})
</script>

<div class="h-screen w-full">
	<AgentEditorModal
		layout="page"
		enableAi={$copilotInfo.enabled}
		owns={(t) => t.host === undefined}
		onClose={(deployed) => goto(deployed ? `${base}/agents/get/${path}` : `${base}/?kind=agent`)}
		onDeployed={(saved) =>
			// Replaced, not pushed, when this URL stopped naming what it opens: Back would otherwise
			// reopen `new_draft` on an agent that now exists, or a path a rename just moved away from.
			goto(`${base}/agents/get/${saved}`, { replaceState: isNew || saved !== path })}
	/>
</div>
