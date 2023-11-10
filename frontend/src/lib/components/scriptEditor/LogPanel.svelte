<script lang="ts">
	import { CompletedJob, Job, JobService, Preview } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import Tabs from '../common/tabs/Tabs.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import TabContent from '../common/tabs/TabContent.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import TableCustom from '../TableCustom.svelte'
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

	export let lang: Preview.language | undefined
	export let previewIsLoading = false
	export let previewJob: Job | undefined
	export let pastPreviews: CompletedJob[] = []
	export let editor: Editor | undefined = undefined
	export let diffEditor: DiffEditor | undefined = undefined
	export let args: Record<string, any> | undefined = undefined

	type DrawerContent = {
		mode: 'json' | Preview.language | 'plain'
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
</script>

<Drawer bind:open={drawerOpen} size="800px">
	<DrawerContent title={drawerContent?.title} on:close={() => closeDrawer()}>
		{#if drawerContent?.content == undefined}
			<div class="p-2"> <Loader2 class="animate-spin" /> </div>
		{:else if drawerContent?.mode === 'json'}
			<DisplayResult
				workspaceId={previewJob?.workspace_id}
				jobId={previewJob?.id}
				result={drawerContent.content}
			/>
		{:else if drawerContent?.mode === 'plain'}
			<pre
				class="overflow-x-auto break-words relative h-full m-2 text-xs bg-surface shadow-inner p-2"
				>{drawerContent?.content}
			</pre>
		{:else if drawerContent?.mode === 'deno' || drawerContent?.mode === 'python3' || drawerContent?.mode === 'go' || drawerContent?.mode === 'bash' || drawerContent?.mode === 'nativets'}
			<HighlightCode language={drawerContent?.mode} code={drawerContent?.content} />
		{/if}
	</DrawerContent>
</Drawer>

<Tabs bind:selected={selectedTab} class="mt-1">
	<Tab value="logs" size="xs">Logs & Result</Tab>
	<Tab value="history" size="xs">History</Tab>

	<svelte:fragment slot="content">
		<!--
			TabContent would wrap it in an extra div which is undesirable in this situation
			because SplitPanesWrapper uses the parent element as a reference point.
		-->
		{#if selectedTab === 'logs'}
			<SplitPanesWrapper>
				<Splitpanes horizontal>
					<Pane class="relative">
						<LogViewer
							jobId={previewJob?.id}
							duration={previewJob?.['duration_ms']}
							mem={previewJob?.['mem_peak']}
							content={previewJob?.logs}
							isLoading={previewIsLoading}
							tag={previewJob?.tag}
						/>
					</Pane>
					<Pane>
						{#if previewJob != undefined && 'result' in previewJob}
							<div class="relative w-full h-full p-2">
								<DisplayResult
									workspaceId={previewJob?.workspace_id}
									jobId={previewJob?.id}
									result={previewJob.result}
								>
									<svelte:fragment slot="copilot-fix">
										{#if lang && editor && diffEditor && args && previewJob?.result?.error}
											<ScriptFix
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
						{:else}
							<div class="text-sm text-tertiary p-2">
								{#if previewIsLoading}
									<Loader2 class="animate-spin" />
								{:else}
									Test to see the result here
								{/if}
							</div>
						{/if}
					</Pane>
				</Splitpanes>
			</SplitPanesWrapper>
		{/if}
		<TabContent value="history" class="p-2">
			<TableCustom>
				<tr slot="header-row">
					<th class="text-xs">Id</th>
					<th class="text-xs">Created at</th>
					<th class="text-xs">Success</th>
					<th class="text-xs">Result</th>
					<th class="text-xs">Code</th>
					<th class="text-xs">Logs</th>
				</tr>
				<tbody slot="body">
					{#each pastPreviews as { id, created_at, success }}
						<tr class="">
							<td class="text-xs">
								<a class="pr-3" href="/run/{id}?workspace={$workspaceStore}" target="_blank"
									>{id.substring(30)}</a
								>
							</td>
							<td class="text-xs">{displayDate(created_at)}</td>
							<td class="text-xs">
								{#if success}
									<CheckCircle2 size={10} class="text-green-600" />
								{:else}
									<XCircle size={10} class="text-red-700" />
								{/if}
							</td>
							<td class="text-xs">
								<button
									class="text-xs"
									on:click|preventDefault={() => {
										openDrawer({ mode: 'json', content: undefined, title: 'Result' })
										JobService.getCompletedJobResult({
											workspace: $workspaceStore ?? 'NO_W',
											id
										}).then((res) => {
											drawerContent && (drawerContent.content = res)
										})
									}}
								>
									See Result
								</button>
							</td>
							<td class="text-xs">
								<button
									class="text-xs"
									on:click|preventDefault={async () => {
										const code = (
											await JobService.getCompletedJob({
												workspace: $workspaceStore ?? 'NO_W',
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
							</td>
							<td>
								<button
									class="text-xs"
									on:click|preventDefault={async () => {
										const logs = (
											await JobService.getCompletedJob({
												workspace: $workspaceStore ?? 'NO_W',
												id
											})
										).logs
										openDrawer({ mode: 'plain', content: String(logs), title: `Logs for ${id}` })
									}}
								>
									View logs
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</TableCustom>
		</TabContent>
	</svelte:fragment>
</Tabs>
