<script lang="ts">
	import {
		type CompletedJob,
		type Job,
		JobService,
		OpenAPI,
		type Preview,
		type WorkflowStatus
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
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
	import Cell from '../table/Cell.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import WorkflowTimeline from '../WorkflowTimeline.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { PreviewPanelUi } from '../custom_ui'

	export let lang: Preview['language'] | undefined
	export let previewIsLoading = false
	export let previewJob: Job | undefined
	export let pastPreviews: CompletedJob[] = []
	export let editor: Editor | undefined = undefined
	export let diffEditor: DiffEditor | undefined = undefined
	export let args: Record<string, any> | undefined = undefined
	export let workspace: string | undefined = undefined
	export let showCaptures: boolean = false
	export let customUi: PreviewPanelUi | undefined = undefined
	export let fixChatMode: boolean = false

	type DrawerContent = {
		mode: 'json' | Preview['language'] | 'plain'
		title: string
		content: any
	}

	let selectedTab = 'logs'
	let drawerOpen: boolean = false
	let drawerContent: DrawerContent | undefined = undefined

	export function setFocusToLogs() {
		selectedTab = 'logs'
	}

	function openDrawer(newContent: DrawerContent) {
		drawerContent = newContent
		drawerOpen = true
	}

	function closeDrawer() {
		drawerOpen = false
	}

	function asWorkflowStatus(x: any): Record<string, WorkflowStatus> {
		return x as Record<string, WorkflowStatus>
	}

	let forceJson = false
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
		<Tab value="logs" size="xs">Logs & Result</Tab>
		{#if customUi?.disableHistory !== true}
			<Tab value="history" size="xs">History</Tab>
		{/if}
		{#if showCaptures && customUi?.disableTriggerCaptures !== true}
			<Tab value="captures" size="xs">Trigger captures</Tab>
		{/if}

		<svelte:fragment slot="content">
			<div class="grow min-h-0">
				{#if selectedTab === 'logs'}
					<SplitPanesWrapper>
						<Splitpanes horizontal>
							{#if previewJob?.is_flow_step == false && previewJob?.flow_status && !(typeof previewJob.flow_status == 'object' && '_metadata' in previewJob.flow_status)}
								<Pane class="relative">
									<WorkflowTimeline
										flow_status={asWorkflowStatus(previewJob.flow_status)}
										flowDone={previewJob.type == 'CompletedJob'}
									/>
								</Pane>
							{/if}
							<Pane class="relative">
								<LogViewer
									jobId={previewJob?.id}
									duration={previewJob?.['duration_ms']}
									mem={previewJob?.['mem_peak']}
									content={previewJob?.logs}
									isLoading={previewJob?.['running'] == false && previewIsLoading}
									tag={previewJob?.tag}
									download={customUi?.disableDownload !== true}
								/>
							</Pane>
							<Pane>
								<slot />
								{#if previewJob != undefined && 'result' in previewJob}
									<div class="relative w-full h-full p-2">
										<div class="relative">
											<DisplayResult
												bind:forceJson
												workspaceId={previewJob?.workspace_id}
												jobId={previewJob?.id}
												result={previewJob.result}
												customUi={customUi?.displayResult}
												language={lang}
											>
												<svelte:fragment slot="copilot-fix">
													{#if lang && editor && diffEditor && args && previewJob?.result && typeof previewJob?.result == 'object' && `error` in previewJob?.result && previewJob?.result.error}
														<ScriptFix
															on:fix
															chatMode={fixChatMode}
															error={JSON.stringify(previewJob.result.error)}
															{lang}
															{editor}
															{diffEditor}
															{args}
														/>
													{/if}
												</svelte:fragment>
											</DisplayResult>
										</div>
									</div>
								{:else}
									<div class="text-sm text-tertiary p-2 flex justify-between items-center">
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
											The result renderer in Windmill supports rich display rendering, allowing you
											to customize the display format of your results.
										</Tooltip>
									</div>
								{/if}
							</Pane>
						</Splitpanes>
					</SplitPanesWrapper>
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
												href="{base}/run/{id}?workspace={workspace ?? $workspaceStore}"
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
												on:click|preventDefault={() => {
													openDrawer({ mode: 'json', content: undefined, title: 'Result' })
													JobService.getCompletedJobResult({
														workspace: workspace ?? $workspaceStore ?? 'NO_W',
														id
													}).then((res) => {
														drawerContent && (drawerContent.content = res)
													})
												}}
											>
												See Result
											</button>
										</Cell>
										<Cell>
											<button
												class="text-xs"
												on:click|preventDefault={async () => {
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
												}}
											>
												View code
											</button>
										</Cell>
										<Cell last>
											<button
												class="text-xs"
												on:click|preventDefault={async () => {
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
												}}
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
					<slot name="capturesTab" />
				{/if}
			</div>
		</svelte:fragment>
	</Tabs>
</div>
