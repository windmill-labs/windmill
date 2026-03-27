<script lang="ts">
	import { preventDefault } from 'svelte/legacy'

	import {
		type CompletedJob,
		type Job,
		JobService,
		OpenAPI,
		type Preview,
		type WorkflowStatus
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { wsBase } from '$lib/workspaceUrl'
	import { displayDate } from '$lib/utils'
	import Tabs from '../common/tabs/Tabs.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import HighlightCode from '../HighlightCode.svelte'
	import LogViewer from '../LogViewer.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SplitPanesWrapper from '../splitPanes/SplitPanesWrapper.svelte'
	import { CheckCircle2, Loader2, XCircle } from 'lucide-svelte'
	import type Editor from '../Editor.svelte'
	import type DiffEditor from '../DiffEditor.svelte'
	import ScriptFix from '../copilot/ScriptFix.svelte'
	import JobOtelTraces from '../JobOtelTraces.svelte'
	import Cell from '../table/Cell.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import WorkflowTimeline from '../WorkflowTimeline.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { PreviewPanelUi } from '../custom_ui'
	import { getStringError } from '../copilot/chat/utils'

	interface Props {
		lang: Preview['language'] | undefined
		previewIsLoading?: boolean
		previewJob: (Job & { result_stream?: string; result?: any; success?: boolean }) | undefined
		pastPreviews?: CompletedJob[]
		editor?: Editor | undefined
		diffEditor?: DiffEditor | undefined
		args?: Record<string, any> | undefined
		workspace?: string | undefined
		showCaptures?: boolean
		customUi?: PreviewPanelUi | undefined
		children?: import('svelte').Snippet
		capturesTab?: import('svelte').Snippet
		customResultPanel?: import('svelte').Snippet
		showCustomResultPanel?: boolean
	}

	let {
		lang,
		previewIsLoading = false,
		previewJob,
		pastPreviews = [],
		editor = undefined,
		diffEditor = undefined,
		args = undefined,
		workspace = undefined,
		showCaptures = false,
		customUi = undefined,
		children,
		capturesTab,
		customResultPanel,
		showCustomResultPanel = false
	}: Props = $props()

	type DContent = {
		mode: 'json' | Preview['language'] | 'plain'
		title: string
		content: any
	}

	let selectedTab = $state('logs')
	let drawerOpen: boolean = $state(false)
	let drawerContent: DContent | undefined = $state(undefined)

	export function setFocusToLogs() {
		selectedTab = 'logs'
	}

	function openDrawer(newContent: DContent) {
		drawerContent = newContent
		drawerOpen = true
	}

	function closeDrawer() {
		drawerOpen = false
	}

	function asWorkflowStatus(x: any): Record<string, WorkflowStatus> {
		if (!x || typeof x !== 'object') return {}
		const result: Record<string, WorkflowStatus> = {}
		for (const [k, v] of Object.entries(x)) {
			if (!k.startsWith('_') || k.startsWith('_step/')) result[k] = v as WorkflowStatus
		}
		return result
	}

	function getStepResults(x: any): Record<string, any> {
		return x?._checkpoint?.completed_steps ?? {}
	}

	let forceJson = $state(false)
	let isWac = $derived(!!previewJob?.workflow_as_code_status)
	let wacDone = $derived(
		previewJob?.type == 'CompletedJob' ||
			(previewJob != undefined && !previewIsLoading && !!previewJob.workflow_as_code_status)
	)
</script>

<Drawer bind:open={drawerOpen} size="800px">
	<DrawerContent title={drawerContent?.title} on:close={() => closeDrawer()}>
		{#if drawerContent?.mode === 'json'}
			<DisplayResult
				workspaceId={previewJob?.workspace_id}
				jobId={previewJob?.id}
				result={drawerContent.content}
				customUi={customUi?.displayResult}
				language={lang}
				result_stream={previewJob?.result_stream}
			/>
		{:else if drawerContent?.mode === 'plain'}
			<pre
				class="overflow-x-auto break-words relative h-full m-2 text-xs bg-surface shadow-inner p-2"
				>{drawerContent?.content}
			</pre>
		{:else if drawerContent?.mode}
			<HighlightCode language={drawerContent?.mode} code={drawerContent?.content} />
		{/if}
	</DrawerContent>
</Drawer>
<div class="h-full flex flex-col">
	<Tabs bind:selected={selectedTab} class="pt-1" wrapperClass="flex-none">
		<Tab value="logs" label="Logs & Result" />
		{#if customUi?.disableHistory !== true}
			<Tab value="history" label="History" />
		{/if}
		{#if showCaptures && customUi?.disableTriggerCaptures !== true}
			<Tab value="captures" label="Trigger captures" />
		{/if}
		{#if customUi?.disableTracing !== true}
			<Tab value="tracing" label="Tracing" />
		{/if}

		{#snippet content()}
			<div class="grow min-h-0">
				{#if selectedTab === 'logs'}
					{#if isWac}
						<div class="h-full overflow-auto">
							<WorkflowTimeline
								flow_status={asWorkflowStatus(previewJob?.workflow_as_code_status)}
								flowDone={wacDone}
								stepResults={getStepResults(previewJob?.workflow_as_code_status)}
								result={previewJob?.result}
								success={previewJob?.success !== false}
								autoExpandResult
								jobId={previewJob?.id}
							/>
						</div>
					{:else}
						<SplitPanesWrapper>
							<Splitpanes horizontal>
								<Pane class="relative">
									<LogViewer
										jobId={previewJob?.id}
										duration={previewJob?.['duration_ms']}
										mem={previewJob?.['mem_peak']}
										content={previewJob?.logs}
										isLoading={previewJob?.['running'] == false && previewIsLoading}
										tag={previewJob?.tag}
										download={customUi?.disableDownload !== true}
										tagLabel={customUi?.tagLabel}
									/>
								</Pane>
								<Pane>
									{@render children?.()}
									{#if showCustomResultPanel && customResultPanel}
										<div class="h-full">
											{@render customResultPanel()}
										</div>
									{:else if previewJob != undefined && (previewJob.result_stream || previewJob.result)}
										<div class="relative w-full h-full p-2">
											<div class="relative h-full">
												<DisplayResult
													bind:forceJson
													workspaceId={previewJob?.workspace_id}
													jobId={previewJob?.id}
													result={previewJob.result}
													customUi={customUi?.displayResult}
													language={lang}
													result_stream={previewJob?.result_stream}
													fixTableSizingToParent
												>
													{#snippet copilot_fix()}
														{#if lang && editor && diffEditor && args && previewJob && !previewJob.success && getStringError(previewJob.result)}
															<ScriptFix {lang} />
														{/if}
													{/snippet}
												</DisplayResult>
											</div>
										</div>
									{:else}
										<div class="text-sm text-primary p-2 flex justify-between items-center">
											<span>
												{#if previewIsLoading}
													<Loader2 class="animate-spin" />
												{:else}
													Test to see the result here
												{/if}
											</span>
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/rich_display_rendering"
											>
												The result renderer in Windmill supports rich display rendering, allowing
												you to customize the display format of your results.
											</Tooltip>
										</div>
									{/if}
								</Pane>
							</Splitpanes>
						</SplitPanesWrapper>
					{/if}
				{/if}
				{#if selectedTab === 'history'}
					<div>
						<DataTable size="xs" noBorder>
							<Head>
								<tr>
									<Cell first>Id</Cell>
									<Cell>Created at</Cell>
									<Cell>Success</Cell>
									<Cell>Result</Cell>
									<Cell>Code</Cell>
									<Cell last>Logs</Cell>
								</tr>
							</Head>
							<tbody class="divide-y">
								{#each pastPreviews as { id, created_at, success }}
									<tr>
										<Cell first>
											<a
												class="pr-3"
												href="{$wsBase}/run/{id}"
												target="_blank">{id.substring(30)}</a
											>
										</Cell>
										<Cell>{displayDate(created_at)}</Cell>
										<Cell>
											{#if success}
												<CheckCircle2 size={10} class="text-green-600" />
											{:else}
												<XCircle size={10} class="text-red-700" />
											{/if}
										</Cell>
										<Cell>
											<button
												class="text-xs"
												onclick={preventDefault(() => {
													openDrawer({ mode: 'json', content: undefined, title: 'Result' })
													JobService.getCompletedJobResult({
														workspace: workspace ?? $workspaceStore ?? 'NO_W',
														id
													}).then((res) => {
														drawerContent && (drawerContent.content = res)
													})
												})}
											>
												See Result
											</button>
										</Cell>
										<Cell>
											<button
												class="text-xs"
												onclick={preventDefault(async () => {
													const code = (
														await JobService.getCompletedJob({
															workspace: workspace ?? $workspaceStore ?? 'NO_W',
															id
														})
													).raw_code
													openDrawer({
														mode: lang ?? 'plain',
														content: String(code),
														title: `Code ${lang}`
													})
												})}
											>
												View code
											</button>
										</Cell>
										<Cell last>
											<button
												class="text-xs"
												onclick={preventDefault(async () => {
													const logs = await (
														await fetch(
															OpenAPI.BASE +
																`/w/${workspace ?? $workspaceStore}/jobs_u/get_logs/${id}`
														)
													).text()
													console.log(logs)
													openDrawer({
														mode: 'plain',
														content: String(logs),
														title: `Logs for ${id}`
													})
												})}
											>
												View logs
											</button>
										</Cell>
									</tr>
								{/each}
							</tbody>
						</DataTable>
					</div>
				{/if}
				{#if selectedTab === 'captures'}
					{@render capturesTab?.()}
				{/if}
				{#if selectedTab === 'tracing'}
					{#if previewJob?.id}
						<JobOtelTraces jobId={previewJob.id} />
					{:else}
						<div class="p-4 text-secondary"> Run a preview to see HTTP request traces </div>
					{/if}
				{/if}
			</div>
		{/snippet}
	</Tabs>
</div>
