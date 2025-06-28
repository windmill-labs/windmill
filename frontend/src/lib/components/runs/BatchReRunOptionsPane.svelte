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
	import { JobService, type InputTransform, type ListSelectedJobGroupsResponse } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { Schema } from '$lib/common'
	import InputTransformForm from '../InputTransformForm.svelte'
	import type { FlowPropPickerConfig, PropPickerContext } from '../prop_picker'
	import { setContext, untrack } from 'svelte'
	import { writable } from 'svelte/store'
	import type { PickableProperties } from '../flows/previousResults'
	import Alert from '../common/alert/Alert.svelte'
	import {
		batchReRunDefaultPropertyExpr,
		buildExtraLibForBatchReruns,
		mergeSchemasForBatchReruns
	} from '$lib/components/jobs/batchReruns'
	import Toggle from '../Toggle.svelte'
	import { TriangleAlert } from 'lucide-svelte'

	let {
		selectedIds,
		options = $bindable()
	}: {
		selectedIds: string[]
		options: BatchReRunOptions
	} = $props()

	let selected: JobGroup | undefined = $state()
	$effect(() => {
		jobGroupsPromise.then((jobGroups) => {
			selected = selected
				? jobGroups.find((g) => g.script_path === selected?.script_path && g.kind === selected.kind)
				: jobGroups[0]
		})
	})

	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined)
	})

	type JobGroup = ListSelectedJobGroupsResponse[number]

	const listSelectedJobsSchemaCache = new Map<string, JobGroup>()
	async function fetchJobGroups(): Promise<JobGroup[]> {
		if (!$workspaceStore) return []

		const cachedSelectedIds = selectedIds.filter((id) => listSelectedJobsSchemaCache.has(id))
		const nonCachedSelectedIds = selectedIds.filter((id) => !listSelectedJobsSchemaCache.has(id))

		console.log(
			`Fetching job groups for ${nonCachedSelectedIds.length} jobs, ${cachedSelectedIds.length} cached`
		)
		const newJobGroups = nonCachedSelectedIds.length
			? await JobService.listSelectedJobGroups({
					workspace: $workspaceStore,
					requestBody: nonCachedSelectedIds
				})
			: []

		// Update cache
		newJobGroups.forEach((group) => {
			group.schemas.forEach((s) => {
				s.job_ids.forEach((job_id) => {
					listSelectedJobsSchemaCache.set(job_id, group)
				})
			})
		})

		const jobGroups: JobGroup[] = newJobGroups

		// Handle cached
		for (const jobId of cachedSelectedIds) {
			const cachedGroup = listSelectedJobsSchemaCache.get(jobId)
			const jobSchema = cachedGroup?.schemas.find((s) => s.job_ids.includes(jobId))
			if (!cachedGroup || !jobSchema) {
				continue
			}
			const group =
				jobGroups.find(
					(j) => j.kind === cachedGroup.kind && j.script_path === cachedGroup.script_path
				) ??
				jobGroups[
					jobGroups.push({
						kind: cachedGroup.kind,
						script_path: cachedGroup.script_path,
						schemas: [],
						latest_schema: cachedGroup.latest_schema
					}) - 1
				]

			const schemaItem =
				group.schemas.find((s) => s.script_hash === jobSchema.script_hash) ??
				group.schemas[
					group.schemas.push({
						schema: jobSchema.schema as Schema,
						job_ids: [],
						script_hash: jobSchema.script_hash
					}) - 1
				]

			schemaItem.job_ids.push(jobId)
		}

		jobGroups.sort((a, b) => {
			if (a.script_path < b.script_path) return -1
			if (a.script_path > b.script_path) return 1
			return 0
		})
		return jobGroups
	}

	function jobGroupTotalCount(group: JobGroup) {
		return group.schemas.reduce((p, c) => p + c.job_ids.length, 0)
	}
	function propertyAlwaysExists(propertyName: string, group: JobGroup): boolean {
		for (const s of group.schemas) {
			if (!(propertyName in (s.schema as Schema).properties)) return false
		}
		return true
	}

	function propertyAlwaysHasSameType(propertyName: string, group: JobGroup): boolean {
		let prevType = 'INIT'
		for (const s of group.schemas) {
			const currType = (s.schema as Schema).properties[propertyName]?.type
			if (currType === undefined) continue
			if (prevType !== 'INIT' && currType !== prevType) return false
			prevType = currType
		}
		return true
	}

	const selectedUsesLatestSchema = $derived(
		!!selected &&
			(selected?.kind === 'flow' ||
				(options[selected.kind][selected.script_path]?.use_latest_version ?? false))
	)

	const jobGroupsPromise = $derived(selectedIds && untrack(() => fetchJobGroups()))
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
					{#if selected}
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
							size="sm"
							options={{
								right: 'Always use latest version',
								rightTooltip:
									selected.kind === 'flow'
										? 'Flow jobs will always run on the latest version of the flow'
										: 'Run all jobs with the latest version of the script even if they originally ran an older version'
							}}
						/>

						<!-- Even if we use the latest schema, we want the editor -->
						<!-- to only lint the original jobs' values -->
						{@const displayedSchema = selectedUsesLatestSchema
							? (selected.latest_schema as Schema)
							: mergeSchemasForBatchReruns(selected.schemas.map((s) => s.schema as Schema))}
						{@const extraLib = buildExtraLibForBatchReruns({
							schemas: selected.schemas,
							script_path: selected.script_path
						})}
						<div class="w-full h-full">
							{#key [selected, displayedSchema]}
								{#each Object.keys(displayedSchema.properties) as propertyName}
									<InputTransformForm
										class="items-start mb-4"
										arg={options[selected.kind][selected.script_path]?.input_transforms?.[
											propertyName
										] ?? {
											type: 'javascript',
											expr: batchReRunDefaultPropertyExpr(propertyName, selected.schemas)
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
										pickableProperties={{
											hasResume: false,
											previousId: undefined,
											priorIds: {},
											flow_input: {}
										}}
										hideHelpButton
										{...propertyAlwaysExists(propertyName, selected)
											? {}
											: {
													headerTooltip:
														'This property does not exist on all versions of the script. You can handle different cases in the code below',
													HeaderTooltipIcon: TriangleAlert,
													headerTooltipIconClass: 'text-orange-500'
												}}
										{...propertyAlwaysHasSameType(propertyName, selected)
											? {}
											: {
													headerTooltip:
														'This property does not always have the same type depending on the version of the script. You can handle different cases in the code below',
													HeaderTooltipIcon: TriangleAlert,
													headerTooltipIconClass: 'text-orange-500'
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
