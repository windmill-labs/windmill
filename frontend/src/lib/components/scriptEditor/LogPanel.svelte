<script lang="ts">
	import { CompletedJob, Job, JobService, Preview } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import { faTimes } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'
	import Tabs from '../common/tabs/Tabs.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import TabContent from '../common/tabs/TabContent.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import TableCustom from '../TableCustom.svelte'
	import Drawer from '../common/drawer/Drawer.svelte'
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import HighlightCode from '../HighlightCode.svelte'
	import LogViewer from '../LogViewer.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SplitPanesWrapper from '../splitPanes/SplitPanesWrapper.svelte'
	import { Loader2 } from 'lucide-svelte'
	import type Editor from '../Editor.svelte'
	import type DiffEditor from '../DiffEditor.svelte'

	export let lang: Preview.language | undefined
	export let previewIsLoading = false
	export let previewJob: Job | undefined
	export let pastPreviews: CompletedJob[] = []
	export let editor: Editor | undefined = undefined
	export let diffEditor: DiffEditor | undefined = undefined

	type DrawerContent = {
		mode: 'json' | Preview.language | 'plain'
		title: string
		content: string
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
		{#if drawerContent?.mode === 'json'}
			<Highlight language={json} code={JSON.stringify(drawerContent.content, null, 4)} />
		{:else if drawerContent?.mode === 'plain'}
			<pre
				class="overflow-x-auto break-words relative h-full m-2 text-xs bg-surface shadow-inner p-2">
				{drawerContent?.content}
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
						/>
					</Pane>
					<Pane>
						{#if previewJob != undefined && 'result' in previewJob}
							<div class="relative w-full h-full p-2"
								><DisplayResult
									workspaceId={previewJob?.workspace_id}
									jobId={previewJob?.id}
									result={previewJob.result}
									{editor}
									{diffEditor}
									{lang}
								/>
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
					{#each pastPreviews as { id, created_at, success, result }}
						<tr class="">
							<td class="text-xs">
								<a class="pr-3" href="/run/{id}" target="_blank">{id.substring(30)}</a>
							</td>
							<td class="text-xs">{displayDate(created_at)}</td>
							<td class="text-xs">
								{#if success}
									<Icon class="text-green-600" data={check} scale={0.6} />
								{:else}
									<Icon class="text-red-700" data={faTimes} scale={0.6} />
								{/if}
							</td>
							<td class="text-xs">
								<a
									href="#result"
									class="text-xs"
									on:click={() => {
										openDrawer({ mode: 'json', content: result, title: 'Result' })
									}}
								>
									{JSON.stringify(result).substring(0, 30)}...
								</a>
							</td>
							<td class="text-xs">
								<a
									href="#code"
									class="text-xs"
									on:click={async () => {
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
								</a>
							</td>
							<td>
								<a
									href="#logs"
									class="text-xs"
									on:click={async () => {
										const logs = (
											await JobService.getCompletedJob({
												workspace: $workspaceStore ?? 'NO_W',
												id
											})
										).logs
										openDrawer({ mode: 'plain', content: String(logs), title: `Code ${lang}` })
									}}
								>
									View logs
								</a>
							</td>
						</tr>
					{/each}
				</tbody>
			</TableCustom>
		</TabContent>
	</svelte:fragment>
</Tabs>
