<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import type { Job } from '$lib/gen'

	const { selectedJobs }: { selectedJobs: Job[] } = $props()

	const scriptPaths = $derived(new Set(selectedJobs.flatMap((j) => j.script_path ?? [])))
	let selectedPath: string | undefined = $state(undefined)
	$effect(() => {
		if (selectedPath && !scriptPaths.has(selectedPath)) selectedPath = undefined
		const firstScript = scriptPaths.values().next().value
		if (!selectedPath && firstScript) selectedPath = firstScript
	})
</script>

<div class="flex-1 flex flex-col">
	<p class="ml-4 mt-4 text-xs font-semibold truncate">Batch re-run options</p>
	<div class="border m-4 flex-1">
		<Splitpanes>
			<Pane size={30}>
				<PanelSection
					title="Runnables"
					id="batch-rerun-options-runnable-list"
					class="bg-surface-secondary"
				>
					{#each scriptPaths as scriptPath}
						<button
							class="border rounded-sm w-full text-left font-normal py-1 px-2 text-2xs truncate {selectedPath ===
							scriptPath
								? 'border-blue-500 bg-blue-100 dark:bg-frost-900/50'
								: 'hover:bg-blue-50 dark:hover:bg-frost-900/50'}"
							onclick={() => (selectedPath = scriptPath)}
						>
							{scriptPath}
						</button>
					{/each}
				</PanelSection>
			</Pane>
			<Pane size={70}></Pane>
		</Splitpanes>
	</div>
</div>
