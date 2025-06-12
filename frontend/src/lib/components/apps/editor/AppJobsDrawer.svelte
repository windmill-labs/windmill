<script lang="ts">
	import { Badge, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'

	import { workspaceStore } from '$lib/stores'
	import { BellOff, Loader2, RefreshCw } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { classNames, truncateRev, isFlowPreview } from '../../../utils'

	import PanelSection from './settingsPanel/common/PanelSection.svelte'

	import AppTimeline from './AppTimeline.svelte'

	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import type { Job } from '$lib/gen'
	import type { JobById } from '../types'
	import { createEventDispatcher } from 'svelte'

	export let open = false
	export let jobs: string[]
	export let jobsById: Record<string, JobById>

	export let hasErrors: boolean = false
	export let selectedJobId: string | undefined = undefined
	export let refreshComponents: (() => void) | undefined = undefined
	export let errorByComponent: Record<string, { id?: string; error: string }> = {}

	const dispatch = createEventDispatcher()

	let testJobLoader: TestJobLoader
	let job: Job | undefined = undefined
	let testIsLoading = false

	let rightColumnSelect: 'timeline' | 'detail' = 'timeline'

	$: selectedJobId && !selectedJobId?.includes('Frontend') && testJobLoader?.watchJob(selectedJobId)

	$: if (selectedJobId?.includes('Frontend') && selectedJobId) {
		job = undefined
	}
</script>

<TestJobLoader bind:this={testJobLoader} bind:isLoading={testIsLoading} bind:job />

<Drawer bind:open size="900px">
	<DrawerContent
		noPadding
		title="Debug Runs"
		on:close={() => {
			open = false
		}}
		tooltip="Look at latests runs to spot potential bugs."
		documentationLink="https://www.windmill.dev/docs/apps/app_debugging"
	>
		<Splitpanes class="!overflow-visible">
			<Pane size={25}>
				<PanelSection title="Past Runs">
					<div class="flex flex-col gap-2 w-full">
						{#if jobs.length > 0}
							<div class="flex gap-2 flex-col-reverse">
								{#each jobs ?? [] as id}
									{@const selectedJob = jobsById[id]}
									{#if selectedJob}
										<!-- svelte-ignore a11y-click-events-have-key-events -->
										<!-- svelte-ignore a11y-no-static-element-interactions -->
										<div
											class={classNames(
												'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-surface-secondary hover:text-blue-400',
												selectedJob.error ? 'border border-red-500 text-primary' : '',
												selectedJob.error && errorByComponent[selectedJob.component]?.id == id
													? selectedJobId == id
														? 'bg-red-600 !border-blue-600'
														: 'bg-red-400'
													: selectedJobId == id
														? 'text-blue-600'
														: ''
											)}
											on:click={() => {
												selectedJobId = id
												rightColumnSelect = 'detail'
											}}
										>
											<span class="text-xs truncate">{truncateRev(selectedJob.job, 20)}</span>
											<Badge color="indigo">{selectedJob.component}</Badge>
										</div>
									{/if}
								{/each}
							</div>
						{:else}
							<div class="text-sm text-tertiary">No items</div>
						{/if}
					</div>
				</PanelSection>
			</Pane>
			<Pane size={75}>
				<div class="w-full h-full flex flex-col">
					<div>
						<Tabs bind:selected={rightColumnSelect}>
							<Tab value="timeline"><span class="font-semibold text-md">Timeline</span></Tab>
							<Tab value="detail"><span class="font-semibold">Details</span></Tab>
						</Tabs>
					</div>
					{#if rightColumnSelect == 'timeline'}
						<div class="p-2 grow overflow-auto">
							<AppTimeline {jobs} {jobsById} />
						</div>
					{:else if rightColumnSelect == 'detail'}
						<div class="grow flex flex-col w-full overflow-auto">
							{#if selectedJobId}
								{#if selectedJobId?.includes('Frontend')}
									{@const jobResult = jobsById[selectedJobId]}
									{#if jobResult?.error !== undefined}
										<Splitpanes horizontal class="grow border w-full">
											<Pane size={10} minSize={10}>
												<LogViewer
													content={`Logs are avaiable in the browser console directly`}
													isLoading={false}
													tag={undefined}
												/>
											</Pane>
											<Pane size={90} minSize={10} class="text-sm text-secondary">
												<div class="relative h-full px-2">
													<DisplayResult
														result={{
															error: { name: 'Frontend execution error', message: jobResult.error }
														}}
													/>
												</div>
											</Pane>
										</Splitpanes>
									{:else if jobResult !== undefined}
										<Splitpanes horizontal class="grow border w-full">
											<Pane size={10} minSize={10}>
												<LogViewer
													content={`Logs are avaiable in the browser console directly`}
													isLoading={false}
													tag={undefined}
												/>
											</Pane>
											<Pane size={90} minSize={10} class="text-sm text-secondary">
												<div class="relative h-full px-2">
													<DisplayResult
														workspaceId={$workspaceStore}
														jobId={selectedJobId}
														result={jobResult.result}
													/>
												</div>
											</Pane>
										</Splitpanes>
									{:else}
										<Loader2 class="animate-spin" />
									{/if}
								{:else}
									<div class="flex flex-col h-full w-full mb-4">
										{#if job?.['running']}
											<div class="flex flex-row-reverse w-full">
												<Button
													color="red"
													variant="border"
													on:click={() => testJobLoader?.cancelJob()}
												>
													<Loader2 size={14} class="animate-spin mr-2" />

													Cancel
												</Button>
											</div>
										{/if}
										{#if job?.args}
											<div class="p-2">
												<JobArgs
													id={job.id}
													workspace={job.workspace_id ?? $workspaceStore ?? 'no_w'}
													args={job?.args}
												/>
											</div>
										{/if}
										{#if job?.raw_code}
											<div class="pb-2 pl-2 pr-2 w-full overflow-auto h-full max-h-[80px]">
												<HighlightCode language={job?.language} code={job?.raw_code} />
											</div>
										{/if}

										{#if job?.job_kind !== 'flow' && !isFlowPreview(job?.job_kind)}
											{@const jobResult = jobsById[selectedJobId]}
											<Splitpanes horizontal class="grow border w-full">
												<Pane size={50} minSize={10}>
													<LogViewer
														duration={job?.['duration_ms']}
														jobId={job?.id}
														content={job?.logs}
														isLoading={testIsLoading && job?.['running'] == false}
														tag={job?.tag}
													/>
												</Pane>
												<Pane size={50} minSize={10} class="text-sm text-secondary">
													{#if job != undefined && 'result' in job && job.result != undefined}<div
															class="relative h-full px-2"
															><DisplayResult
																workspaceId={$workspaceStore}
																jobId={selectedJobId}
																result={job.result}
															/></div
														>
													{:else if testIsLoading}
														<div class="p-2"><Loader2 class="animate-spin" /> </div>
													{:else if job != undefined && 'result' in job && job?.['result'] == undefined}
														<div class="p-2 text-tertiary">Result is undefined</div>
													{:else}
														<div class="p-2 text-tertiary">
															<Loader2 size={14} class="animate-spin mr-2" />
														</div>
													{/if}
												</Pane>
												{#if jobResult?.transformer}
													<Pane size={50} minSize={10} class="text-sm text-secondary p-2">
														<div class="font-bold">Transformer results</div>
														{#if job != undefined && 'result' in job && job.result != undefined}
															<div class="relative h-full px-2">
																<DisplayResult
																	workspaceId={$workspaceStore}
																	jobId={selectedJobId}
																	result={jobResult?.transformer}
																/>
															</div>
														{:else if testIsLoading}
															<div class="p-2"><Loader2 class="animate-spin" /> </div>
														{:else if job != undefined && 'result' in job && job?.['result'] == undefined}
															<div class="p-2 text-tertiary">Result is undefined</div>
														{:else}
															<div class="p-2 text-tertiary">
																<Loader2 size={14} class="animate-spin mr-2" />
															</div>
														{/if}
													</Pane>
												{/if}
											</Splitpanes>
										{:else}
											<div class="mt-10"></div>
											<FlowProgressBar {job} class="py-4" />
											<div class="w-full mt-10 mb-20">
												{#if job?.id}
													<FlowStatusViewer
														jobId={job?.id}
														on:jobsLoaded={({ detail }) => {
															job = detail
														}}
													/>
												{:else}
													<Loader2 class="animate-spin" />
												{/if}
											</div>
										{/if}
									</div>
								{/if}
							{:else}
								<div class="text-sm p-2 text-tertiary">Select a job to see its details</div>
							{/if}
						</div>
					{/if}
				</div>
			</Pane>
		</Splitpanes>
		{#snippet actions()}
			{#if refreshComponents}
				<Button
					size="md"
					color="light"
					variant="border"
					on:click={() => {
						refreshComponents?.()
					}}
					title="Refresh App"
				>
					Refresh app&nbsp;<RefreshCw size={16} />
				</Button>
			{/if}
			<Button
				size="md"
				color="light"
				variant="border"
				on:click={() => {
					dispatch('clear')
				}}
				>Clear jobs
			</Button>
			{#if hasErrors}
				<Button size="md" color="light" variant="border" on:click={() => dispatch('clearErrors')}>
					Clear Errors &nbsp;<BellOff size={14} />
				</Button>
			{/if}
		{/snippet}
	</DrawerContent>
</Drawer>
