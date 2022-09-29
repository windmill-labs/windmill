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
	import { VSplitPane } from 'svelte-split-pane'
	import LogViewer from '../LogViewer.svelte'

	export let path: string | undefined
	export let lang: Preview.language
	export let previewIsLoading = false
	export let previewJob: Job | undefined
	export let pastPreviews: CompletedJob[] = []
	export let lastSave: string | null

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
			<pre class="overflow-x-auto break-all relative h-full m-2 text-xs bg-white shadow-inner p-2">
				{drawerContent?.content}
			</pre>
		{:else if drawerContent?.mode === 'deno' || drawerContent?.mode === 'python3' || drawerContent?.mode === 'go'}
			<HighlightCode language={drawerContent?.mode} code={drawerContent?.content} />
		{/if}
	</DrawerContent>
</Drawer>

<Tabs bind:selected={selectedTab}>
	<Tab value="logs"><span class="text-xs">Logs/Result</span></Tab>
	<Tab value="history"><span class="text-xs">History</span></Tab>
	<Tab value="last_save"><span class="text-xs">Last save</span></Tab>

	<svelte:fragment slot="content">
		<TabContent value="logs" class="h-full w-full relative">
			<VSplitPane topPanelSize="50%" downPanelSize="50%">
				<top slot="top">
					<LogViewer content={previewJob?.logs} isLoading={previewIsLoading} />
				</top>
				<down slot="down">
					<pre
						class="overflow-x-auto break-all relative h-full p-2 text-sm">{#if previewJob && 'result' in previewJob && previewJob.result}<DisplayResult
								result={previewJob.result}
							/>
						{:else if previewIsLoading}Waiting for Result...
						{:else}Test to see the result here
						{/if}
        </pre>
				</down>
			</VSplitPane>
		</TabContent>
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

										openDrawer({ mode: lang, content: String(code), title: `Code ${lang}` })
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
		<TabContent value="last_save" class="p-2">
			{#if lastSave}
				<h2>last local save for path {path}</h2>
				<HighlightCode language={lang} code={lastSave} />
			{:else}
				No local save
			{/if}
		</TabContent>
	</svelte:fragment>
</Tabs>
