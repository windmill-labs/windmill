<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import type { FlowCopilotModule } from './flow'
	import { ScriptService, type Script } from '$lib/gen'

	import { Wand2 } from 'lucide-svelte'
	import SearchItems from '../SearchItems.svelte'
	import { emptyString } from '$lib/utils'

	export let funcDesc: string
	export let trigger = false

	// state
	let input: HTMLInputElement | undefined
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

<div class="text-primary transition-all flex-grow">
	<div>
		<div class="flex py-2 relative">
			<input
				type="text"
				bind:this={input}
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
			{#if funcDesc.length === 0}
				<Wand2
					size={14}
					class="absolute right-4 top-1/2 -translate-y-1/2 fill-current opacity-70 text-violet-800 dark:text-violet-400"
				/>
			{/if}
		</div>
	</div>
</div>
