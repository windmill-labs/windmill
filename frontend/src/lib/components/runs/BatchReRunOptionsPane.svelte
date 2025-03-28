<script lang="ts" module>
	export type BatchReRunOptions = {
		[kind in 'flow' | 'script']: {
			[path: string]: {
				input_transforms?: { [property: string]: InputTransform }
				use_latest_version?: boolean
			}
		}
	}
</script>

<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import { JobService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { Schema } from '$lib/common'
	import InputTransformForm from '../InputTransformForm.svelte'
	import type { FlowPropPickerConfig, PropPickerContext } from '../prop_picker'
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import type { PickableProperties } from '../flows/previousResults'
	import Alert from '../common/alert/Alert.svelte'
	import { buildExtraLibForBatchReruns } from '$lib/components/jobs/batchReruns'
	import { pluralize } from '$lib/utils'
	import { mergeObjectSchemasWithUnion } from '$lib/schema'
	import Toggle from '../Toggle.svelte'

	let {
		selectedIds,
		options = $bindable()
	}: {
		selectedIds: string[]
		options: BatchReRunOptions
	} = $props()

	let selected: JobGroup | undefined = $state()

	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined)
	})

	type JobGroup = {
		script_path: string
		kind: 'script' | 'flow'
		schemas: { schema: Schema; script_hash: string; count: number }[]
		latestSchema: Schema
	}

	async function fetchJobGroups(): Promise<JobGroup[]> {
		if (!$workspaceStore) return []
		// TODO : cache and filter jobs we already have
		const selectedJobsSchemas = await JobService.listSelectedJobsSchema({
			workspace: $workspaceStore,
			requestBody: selectedIds
		})
		const latestSchemas = await JobService.listSelectedJobsLatestSchema({
			workspace: $workspaceStore,
			requestBody: selectedIds
		})
		const jobGroup: JobGroup[] = []
		for (const curr of selectedJobsSchemas) {
			if (!curr.script_path || !curr.script_hash) continue
			const latestSchema: Schema = (latestSchemas.find(
				(ls) => ls.kind === curr.kind && ls.script_path === curr.script_path
			)?.schema ?? {}) as Schema

			const group =
				jobGroup.find((j) => j.kind === curr.kind && j.script_path === curr.script_path) ??
				jobGroup[
					jobGroup.push({
						kind: curr.kind,
						script_path: curr.script_path,
						schemas: [],
						latestSchema
					}) - 1
				]
			group.schemas.push({
				schema: curr.schema as Schema,
				count: curr.count,
				script_hash: curr.script_hash
			})
		}

		selected =
			(selected &&
				jobGroup.find(
					(g) => g.script_path === selected?.script_path && g.kind === selected.kind
				)) ??
			jobGroup[0]
		return jobGroup
	}

	function jobGroupTotalCount(group: JobGroup) {
		return group.schemas.reduce((p, c) => p + c.count, 0)
	}
	function getHashesWithProperty(propertyName: string, group: JobGroup): Set<string> {
		const set = new Set<string>()
		for (const s of group.schemas) {
			if (propertyName in s.schema.properties) {
				set.add(s.script_hash)
			}
		}
		return set
	}

	const selectedUsesLatestSchema = $derived(
		!!selected &&
			(selected?.kind === 'flow' ||
				(options[selected.kind][selected.script_path]?.use_latest_version ?? false))
	)

	const jobGroupsPromise = $derived(selectedIds && fetchJobGroups())
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
						{#await jobGroupsPromise then jobGroup}
							{#each jobGroup as group}
								<button
									class="border rounded-sm w-full text-left font-normal py-1.5 px-2 text-2xs flex justify-between {selected?.kind ===
										group.kind && selected.script_path === group.script_path
										? 'border-blue-500 bg-blue-100 dark:bg-frost-900/50'
										: 'hover:bg-blue-50 dark:hover:bg-frost-900/50'}"
									onclick={() => (selected = group)}
								>
									<span class="truncate"> {group.script_path}</span>
									<span class="text-gray-400">({jobGroupTotalCount(group)})</span>
								</button>
							{/each}
						{/await}
					</div>
				</PanelSection>
			</Pane>
			<Pane size={68} class="relative">
				<PanelSection
					title="Inputs"
					class="overflow-y-scroll absolute inset-0"
					id="batch-rerun-options-args"
				>
					<div class="text-sm w-full pb-2">
						<Alert type="info" title="Available expressions :">
							Use the <code>job</code> object to access data about the original job
						</Alert>
					</div>
					<Toggle
						checked={selectedUsesLatestSchema}
						disabled={selected?.kind === 'flow'}
						on:change={(e) => {
							if (!selected) return
							;(options[selected.kind][selected.script_path] ??= {}).use_latest_version =
								e.detail as boolean
						}}
						options={{
							right: 'Always use latest version'
						}}
					/>
					{#if selected}
						<!-- Even if we use the latest schema, we want the editor -->
						<!-- to only lint the original jobs' values -->
						{@const mergedSchema = mergeObjectSchemasWithUnion(
							selected.schemas.map((s) => s.schema)
						)}
						{@const displayedSchema = selectedUsesLatestSchema
							? selected.latestSchema
							: mergedSchema}
						{@const extraLib = buildExtraLibForBatchReruns({
							schema: mergedSchema,
							script_path: selected.script_path,
							script_hashes: selected.schemas.map((s) => s.script_hash)
						})}
						<div class="w-full h-full">
							{#key [selected, displayedSchema]}
								{#each Object.keys(displayedSchema.properties) as propertyName}
									{@const hashesWithProperty = getHashesWithProperty(propertyName, selected)}
									<InputTransformForm
										class="items-start mb-4"
										arg={options[selected.kind][selected.script_path]?.input_transforms?.[
											propertyName
										] ?? {
											type: 'javascript',
											expr: /^[a-zA-Z_$][0-9a-zA-Z_$]*$/.test(propertyName)
												? `job.input.${propertyName}`
												: `job.input[${JSON.stringify(propertyName)}]`
										}}
										on:change={(e) => {
											if (!selected) return
											const newArg = e.detail.arg as InputTransform
											;((options[selected.kind][selected.script_path] ??= {}).input_transforms ??=
												{})[propertyName] = newArg
										}}
										argName={propertyName}
										schema={displayedSchema}
										{extraLib}
										previousModuleId={undefined}
										pickablepropertyMap={{
											hasResume: false,
											previousId: undefined,
											priorIds: {},
											flow_input: {}
										}}
										hideHelpButton
										{...hashesWithProperty.size !== selected.schemas.length && {
											headerTooltip: `Used in ${pluralize(hashesWithProperty.size, `${selected?.kind} version`)}: ${[...hashesWithProperty.values()].join(', ')}`
										}}
									/>
								{/each}
							{/key}
						</div>
					{/if}
				</PanelSection>
			</Pane>
		</Splitpanes>
	</div>
</div>
