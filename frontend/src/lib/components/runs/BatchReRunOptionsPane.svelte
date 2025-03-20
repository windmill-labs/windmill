<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'

	const { selectedJobs }: { selectedJobs: Job[] } = $props()

	const groupedJobs: {
		script_path: string
		kind: 'script' | 'flow'
		script_hashes: Set<string>
	}[] = $derived.by(() => {
		const scriptGroup: Map<string, { script_hashes: Set<string> }> = new Map()
		const flowGroup: Map<string, { script_hashes: Set<string> }> = new Map()

		for (const job of selectedJobs) {
			if (!job.script_path || !job.script_hash) {
				console.error('No script path or hash', job)
				continue
			}
			let group: ReturnType<(typeof scriptGroup)['get']>
			if (job.job_kind == 'script') {
				if (!scriptGroup.has(job.script_path))
					scriptGroup.set(job.script_path, { script_hashes: new Set() })
				group = scriptGroup.get(job.script_path)
			}
			if (job.job_kind == 'flow') {
				if (!flowGroup.has(job.script_path))
					flowGroup.set(job.script_path, { script_hashes: new Set() })
				group = flowGroup.get(job.script_path)
			}
			if (!group) {
				console.error('Job is neither script or flow', job)
				continue
			}

			if (!group.script_hashes.has(job.script_hash)) group.script_hashes.add(job.script_hash)
		}

		const list: typeof groupedJobs = []
		scriptGroup.forEach((v, k) => list.push({ script_path: k, kind: 'script', ...v }))
		flowGroup.forEach((v, k) => list.push({ script_path: k, kind: 'flow', ...v }))
		console.log(list)
		return list
	})

	const eq = (a: typeof selected, b: typeof selected) =>
		a?.kind === b?.kind && a?.script_path === b?.script_path

	let selected: { script_path: string; kind: 'script' | 'flow' } | undefined = $state()
	$effect(() => {
		if (groupedJobs.every((g) => !eq(g, selected))) selected = undefined
		if (selected === undefined && groupedJobs.length) selected = groupedJobs[0]
	})
	// const allSchemas = $derived.by(async () => {
	// 	const schemas = []
	// 	if (!selectedJob || !selectedPath || !$workspaceStore) return undefined
	// 	if (selectedJob.job_kind === 'flow') {
	// 		// TODO : why only getFlowByPath; how to get by version too
	// 		const flow = await FlowService.getFlowByPath({
	// 			path: selectedPath,
	// 			workspace: $workspaceStore
	// 		})
	// 		return { schema: flow.schema as Schema }
	// 	}
	// 	if (selectedJob.job_kind === 'script') {
	// 		const script = selectedJob.script_hash
	// 			? await ScriptService.getScriptByHash({
	// 					hash: selectedJob.script_hash,
	// 					workspace: $workspaceStore
	// 				})
	// 			: await ScriptService.getScriptByPath({
	// 					path: selectedJob.script_path ?? '',
	// 					workspace: $workspaceStore
	// 				})
	// 		return { schema: script.schema as Schema }
	// 	}
	// })
	// let jobData: Awaited<typeof jobDataPromise> | undefined = $state()
	// $effect(() => {
	// 	jobData = undefined
	// 	jobDataPromise.then((d) => (jobData = d))
	// })
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
						{#each groupedJobs as group}
							<button
								class="border rounded-sm w-full text-left font-normal py-1.5 px-2 text-2xs truncate {eq(
									selected,
									group
								)
									? 'border-blue-500 bg-blue-100 dark:bg-frost-900/50'
									: 'hover:bg-blue-50 dark:hover:bg-frost-900/50'}"
								onclick={() => (selected = group)}
							>
								{group.script_path}
							</button>
						{/each}
					</div>
				</PanelSection>
			</Pane>
			<Pane size={68}>
				<PanelSection title="Inputs" class="" id="batch-rerun-options-args">
					<!-- {#each Object.keys(jobData?.schema.properties ?? {}) as varName}
						<p class="text-xs">{varName}</p>
						<div class="w-full"><SimpleEditor autoHeight lang="javascript" /></div>
					{/each} -->
				</PanelSection>
			</Pane>
		</Splitpanes>
	</div>
</div>
