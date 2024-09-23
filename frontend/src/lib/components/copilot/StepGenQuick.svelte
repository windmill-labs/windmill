<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import type { FlowCopilotModule } from './flow'
	import { ScriptService, type Script } from '$lib/gen'

	import { Wand2, Loader2 } from 'lucide-svelte'
	import SearchItems from '../SearchItems.svelte'
	import { emptyString } from '$lib/utils'

	export let funcDesc: string
	export let trigger = false
	export let loading = false

	// state
	export let hubCompletions: FlowCopilotModule['hubCompletions'] = []

	let scripts: Script[] | undefined = undefined
	export let filteredItems: (Script & { marked?: string })[] | (Item & { marked?: string })[] = []
	$: prefilteredItems = scripts ?? []

	async function loadScripts(): Promise<void> {
		const loadedScripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			perPage: 300,
			kinds: trigger ? 'trigger' : 'script'
		})

		scripts = loadedScripts
	}

	$: scripts == undefined && funcDesc?.length > 1 && loadScripts()

	let doneTs = 0
	async function getHubCompletions(text: string) {
		try {
			// make sure we display the results of the last request last
			const ts = Date.now()
			const scripts = (
				await ScriptService.queryHubScripts({
					text: `${text}`,
					limit: 3,
					kind: trigger ? 'trigger' : 'script'
				})
			).map((s) => ({
				...s,
				path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`
			}))
			if (ts < doneTs) return
			doneTs = ts

			hubCompletions = scripts as FlowCopilotModule['hubCompletions']
		} catch (err) {
			if (err.name !== 'CancelError') throw err
		}
	}
</script>

<SearchItems
	filter={funcDesc}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>

<div class="relative text-primary items-center transition-all flex-grow">
	<div class="grow items-cente">
		<input
			type="text"
			bind:value={funcDesc}
			on:input={() => {
				if (funcDesc.length > 2) {
					getHubCompletions(funcDesc)
				} else {
					hubCompletions = []
				}
			}}
			placeholder="Search {trigger ? 'triggers' : 'scripts'} or AI gen"
		/>
	</div>
	<div class="absolute inset-y-0 right-3 flex items-center pointer-events-none">
		{#if loading}
			<Loader2 size={16} class="animate-spin text-gray-400" />
		{/if}
		{#if funcDesc.length === 0 && !loading}
			<Wand2 size={14} class="fill-current opacity-70 text-violet-800 dark:text-violet-400" />
		{/if}
	</div>
</div>
