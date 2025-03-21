<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import { ScriptService, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { Schema, SchemaProperty } from '$lib/common'
	import SimpleEditor from '../SimpleEditor.svelte'

	const { selectedJobs }: { selectedJobs: Job[] } = $props()

	type GroupedJob = {
		script_path: string
		kind: 'script' | 'flow'
		script_hashes: Set<string>
	}

	const groupedJobs: GroupedJob[] = $derived.by(() => {
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

		const list: GroupedJob[] = []
		scriptGroup.forEach((v, k) => list.push({ script_path: k, kind: 'script', ...v }))
		flowGroup.forEach((v, k) => list.push({ script_path: k, kind: 'flow', ...v }))
		console.log(list)
		return list
	})

	const eq = (a: GroupedJob | undefined, b: GroupedJob | undefined) =>
		a?.kind === b?.kind && a?.script_path === b?.script_path

	let selected: GroupedJob | undefined = $state()
	$effect(() => {
		if (groupedJobs.every((g) => !eq(g, selected))) selected = undefined
		if (selected === undefined && groupedJobs.length) selected = groupedJobs[0]
	})

	const selectedHashesPromise: Promise<{ schema: Schema; script_hash: string }[]> = $derived.by(
		async () => {
			if (!selected || !$workspaceStore) return []

			// TODO : create routes to avoid many requests
			if (selected.kind === 'script') {
				let scripts = await Promise.all(
					[...selected.script_hashes].map((hash) =>
						ScriptService.getScriptByHash({ hash, workspace: $workspaceStore })
					)
				)
				if (!scripts.length)
					scripts = [
						await ScriptService.getScriptByPath({
							path: selected.script_path,
							workspace: $workspaceStore
						})
					]
				return scripts.map((script) => ({
					schema: (script.schema as Schema) ?? {},
					script_hash: script.hash
				}))
			}

			// TODO : flows and create route for getFlowByVersion (hash to version?)
			if (selected.kind === 'flow') {
				return []
			}

			console.error('selected is neither flow or script')
			return []
		}
	)

	function computePropertyMap(
		selectedHashes: { schema: Schema; script_hash: string }[]
	): Map<string, { property: SchemaProperty; hashes: Set<string> }> {
		const map: ReturnType<typeof computePropertyMap> = new Map()

		for (const { schema, script_hash } of selectedHashes) {
			for (const property in schema.properties) {
				if (!map.has(property))
					map.set(property, {
						property: schema.properties[property],
						hashes: new Set(script_hash)
					})
				else map.get(property)?.hashes.add(script_hash)
			}
		}
		return map
	}
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
					{#await selectedHashesPromise then selectedHashes}
						{@const properties = computePropertyMap(selectedHashes)}

						{#each properties.keys() as propertyName}
							<p class="text-xs">{propertyName}</p>
							<div class="w-full"><SimpleEditor autoHeight lang="javascript" /></div>
						{/each}
					{/await}
				</PanelSection>
			</Pane>
		</Splitpanes>
	</div>
</div>
