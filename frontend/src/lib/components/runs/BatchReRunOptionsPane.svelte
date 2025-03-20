<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import { FlowService, ScriptService, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	const { selectedJobs }: { selectedJobs: Job[] } = $props()
	const scriptPaths = $derived(new Set(selectedJobs.flatMap((j) => j.script_path ?? [])))

	let selectedPath: string | undefined = $state(undefined)
	$effect(() => {
		if (selectedPath && !scriptPaths.has(selectedPath)) selectedPath = undefined
		const firstScript = scriptPaths.values().next().value
		if (!selectedPath && firstScript) selectedPath = firstScript
	})
	const selectedJob = $derived(selectedJobs.find((j) => j.script_path === selectedPath))

	const jobDataPromise = $derived.by(async () => {
		if (!selectedJob || !selectedPath || !$workspaceStore) return undefined
		if (selectedJob.job_kind === 'flow') {
			// TODO : why only getFlowByPath; how to get by version too
			const flow = await FlowService.getFlowByPath({
				path: selectedPath,
				workspace: $workspaceStore
			})
			return { schema: flow.schema }
		}
		if (selectedJob.job_kind === 'script') {
			const script = selectedJob.script_hash
				? await ScriptService.getScriptByHash({
						hash: selectedJob.script_hash,
						workspace: $workspaceStore
					})
				: await ScriptService.getScriptByPath({
						path: selectedJob.script_path ?? '',
						workspace: $workspaceStore
					})
			return { schema: script.schema }
		}
	})
</script>

<div class="flex-1 flex flex-col">
	<p class="ml-4 mt-4 text-xs font-semibold truncate">Batch re-run options</p>
	<div class="border m-4 flex-1">
		<Splitpanes>
			<Pane size={32} class="bg-surface-secondary relative">
				<PanelSection
					title="Runnables"
					class="bg-surface-secondary overflow-y-scroll absolute inset-0"
					id="batch-rerun-options-runnable-list"
				>
					<div class="w-full flex flex-col gap-1">
						{#each scriptPaths as scriptPath}
							<button
								class="border rounded-sm w-full text-left font-normal py-1.5 px-2 text-2xs truncate {selectedPath ===
								scriptPath
									? 'border-blue-500 bg-blue-100 dark:bg-frost-900/50'
									: 'hover:bg-blue-50 dark:hover:bg-frost-900/50'}"
								onclick={() => (selectedPath = scriptPath)}
							>
								{scriptPath}
							</button>
						{/each}
					</div>
				</PanelSection>
			</Pane>
			<Pane size={68}>
				<PanelSection title="Inputs" class="" id="batch-rerun-options-args"></PanelSection>
			</Pane>
		</Splitpanes>
	</div>
</div>
