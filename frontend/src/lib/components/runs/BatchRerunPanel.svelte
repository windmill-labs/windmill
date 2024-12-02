<script lang="ts">
	import { base } from '$lib/base'
	import { FlowService, JobService, ScriptService, type Job, type ScriptArgs } from '../../gen'
	import { Tab, Tabs } from '../common'
	import { workspaceStore } from '$lib/stores'
	import type { Schema } from '$lib/common'
	import SchemaForm from '../SchemaForm.svelte'

	export let selectedIds: string[]
	export let jobs: Job[] | undefined
	export let blankLink = false
	export let workspace: string | undefined

	let viewTab = selectedIds[0] || ''
	$: currentViewJob =
		viewTab === 'common_args' ? undefined : jobs?.find((job) => job.id === viewTab)

	export let args: { [jobId: string]: ScriptArgs }[] = []
	let schemas: { [jobId: string]: Schema }[] = []

	$: {
		if (currentViewJob && !schemas[currentViewJob.id]) {
			loadScript(currentViewJob)
		}
	}

	async function loadScript(job: Job): Promise<void> {
		if (job.job_kind === 'script') {
			if (job.script_hash) {
				schemas[job.id] = (
					await ScriptService.getScriptByHash({
						workspace: $workspaceStore!,
						hash: job.script_hash
					})
				).schema
			} else if (job.script_path) {
				schemas[job.id] = (
					await ScriptService.getScriptByPath({
						workspace: $workspaceStore!,
						path: job.script_path
					})
				).schema
			}
		} else if (job.job_kind === 'flow' && job.script_path) {
			schemas[job.id] = (
				await FlowService.getFlowByPath({
					workspace: $workspaceStore!,
					path: job.script_path
				})
			).schema
		}
		if (schemas[job.id]?.properties && Object.keys(schemas[job.id]).length > 0) {
			args[job.id] = await JobService.getJobArgs({
				workspace: $workspaceStore ?? '',
				id: job.id
			})
		}
	}

	$: if (viewTab !== 'common_args' && !selectedIds.includes(viewTab)) {
		viewTab = selectedIds[0] || '' // when you are viewing a job's tab and you deselect it
	}
</script>

<div class="p-4 flex flex-col gap-2 items-start h-full">
	{#if selectedIds.length > 0}
		<div class=" w-full h-full">
			<Tabs bind:selected={viewTab}>
				<!-- <Tab size="xs" value="common_args">Common Input</Tab> -->
				{#each selectedIds as selectedJobId}
					<Tab size="xs" value={selectedJobId}
						>{jobs?.find((job) => job.id === selectedJobId)?.script_path || 'Job'}</Tab
					>
				{/each}

				<svelte:fragment slot="content">
					<div class="flex flex-col flex-1 h-full">
						{#if viewTab === 'common_args'}
							<p>Not currently implemented</p>
						{:else}
							{@const currentViewJobId = viewTab}
							{@const currentViewSchema = schemas[currentViewJobId]}
							<a
								href="{base}/run/{currentViewJobId}?workspace={workspace ?? $workspaceStore}"
								class="flex flex-row gap-1 items-center"
								target={blankLink ? '_blank' : undefined}
							>
								<span class="font-semibold text-sm leading-6">ID:</span>
								<span class="text-sm">{currentViewJobId ?? ''}</span>
							</a>

							<span class="font-semibold text-xs leading-6">Arguments</span>

							{#if currentViewSchema}
								<div class="my-2" />
								{#if !currentViewSchema.properties || Object.keys(currentViewSchema.properties).length === 0}
									<div class="text-sm py-4 italic">No arguments</div>
								{:else if args[currentViewJobId]}
									<span class=" text-xs leading-6"
										>Jobs will be re-ran using the old arguments, you can override them below</span
									>
									<SchemaForm
										prettifyHeader
										autofocus
										schema={currentViewSchema}
										bind:args={args[currentViewJobId]}
									/>
								{:else}
									<div class="text-xs text-tertiary">Loading...</div>
								{/if}
							{:else}
								<div class="text-xs text-tertiary">Loading...</div>
							{/if}
						{/if}
					</div>
				</svelte:fragment>
			</Tabs>
		</div>
	{/if}
</div>
