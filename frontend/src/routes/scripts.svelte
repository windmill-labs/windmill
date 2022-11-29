<script context="module">
	export function load() {
		return {
			stuff: { title: 'Scripts' }
		}
	}
</script>

<script lang="ts">
	import {
		faArchive,
		faBuilding,
		faCalendarAlt,
		faCodeFork,
		faEdit,
		faEye,
		faGlobe,
		faList,
		faPlay,
		faShare
	} from '@fortawesome/free-solid-svg-icons'
	import type { Script } from '$lib/gen'
	import { ScriptService } from '$lib/gen'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, getScriptByPath, sendUserToast } from '$lib/utils'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { Button, Tabs, Tab, Badge, Skeleton, DrawerContent } from '$lib/components/common'
	import CreateActions from '$lib/components/scripts/CreateActions.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import type { HubItem } from '$lib/components/flows/pickers/model'
	import Icon from 'svelte-awesome'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import Star from '$lib/components/Star.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	type Tab = 'workspace' | 'hub'
	type ScriptW = Script & { canWrite: boolean; marked?: string }
	let scripts: ScriptW[] = []
	let preFilteredScripts: ScriptW[] = []
	let filteredScripts: ScriptW[] = []
	let filter = ''

	let loading = true

	let tab: Tab = 'workspace'

	let shareModal: ShareModal

	let codeViewer: Drawer
	let codeViewerContent: string = ''
	let codeViewerLanguage: 'deno' | 'python3' | 'go' | 'bash' = 'deno'
	let codeViewerObj: HubItem | undefined = undefined

	async function loadScripts(): Promise<void> {
		scripts = (await ScriptService.listScripts({ workspace: $workspaceStore!, perPage: 300 })).map(
			(x: Script) => {
				return {
					canWrite:
						canWrite(x.path, x.extra_perms, $userStore) && x.workspace_id == $workspaceStore,
					...x
				}
			}
		)
		loading = false
	}

	async function archiveScript(path: string): Promise<void> {
		await ScriptService.archiveScriptByPath({ workspace: $workspaceStore!, path })
		loadScripts()
		sendUserToast(`Successfully archived script ${path}`)
	}

	async function viewCode(obj: HubItem) {
		const { content, language } = await getScriptByPath(obj.path)
		codeViewerContent = content
		codeViewerLanguage = language
		codeViewerObj = obj
		codeViewer.openDrawer()
	}

	$: owners = Array.from(
		new Set(filteredScripts?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	$: preFilteredScripts =
		ownerFilter != undefined ? scripts.filter((x) => x.path.startsWith(ownerFilter ?? '')) : scripts

	let ownerFilter: string | undefined = undefined

	$: {
		if ($workspaceStore && ($userStore || $superadmin)) {
			loadScripts()
		}
	}
</script>

<SearchItems
	{filter}
	items={preFilteredScripts}
	bind:filteredItems={filteredScripts}
	f={(x) => x.summary + ' (' + x.path + ')'}
/>

<Drawer bind:this={codeViewer} size="900px">
	<DrawerContent title={codeViewerObj?.summary ?? ''} on:close={codeViewer.closeDrawer}>
		<div slot="submission" class="flex flex-row gap-2 pr-2"
			><Button
				href="https://hub.windmill.dev/scripts/{codeViewerObj?.app ?? ''}/{codeViewerObj?.ask_id ??
					0}"
				startIcon={{ icon: faGlobe }}
				variant="border">View on the Hub</Button
			><Button
				href="/scripts/add?hub={encodeURIComponent(codeViewerObj?.path ?? '')}"
				startIcon={{ icon: faCodeFork }}>Fork</Button
			></div
		>

		<HighlightCode language={codeViewerLanguage} code={codeViewerContent} />
	</DrawerContent>
</Drawer>

<CenteredPage>
	<PageHeader
		title="Scripts"
		tooltip="A Script can be used standalone or as part of a Flow. 
		When standalone, it has webhooks and an auto-generated UI from its parameters whom you can access clicking on 'Run'.
		Scripts have owners (users or groups) and can be shared to users and groups."
	>
		<CreateActions />
	</PageHeader>

	<Tabs bind:selected={tab}>
		<Tab size="xl" value="workspace"><Icon data={faBuilding} class="mr-1" /> Workspace</Tab>
		<Tab size="xl" value="hub"><Icon data={faGlobe} class="mr-1" /> Hub</Tab>
	</Tabs>

	<div class="mb-1" />

	{#if tab == 'workspace'}
		<input type="text" placeholder="Search Scripts" bind:value={filter} class="text-2xl mt-2" />

		<div class="gap-2 w-full flex flex-wrap pb-1 pt-2">
			{#each owners as owner}
				<Badge
					class="cursor-pointer hover:bg-gray-200"
					on:click={() => {
						ownerFilter = ownerFilter == owner ? undefined : owner
					}}
					color={owner === ownerFilter ? 'blue' : 'gray'}
				>
					{owner}
					{#if owner === ownerFilter}&cross;{/if}
				</Badge>
			{/each}
		</div>
		<div class="grid grid-cols-1 gap-4 lg:grid-cols-2 mt-2 w-full">
			{#if !loading}
				{#each filteredScripts as { summary, path, hash, language, extra_perms, canWrite, lock_error_logs, kind, marked, starred } (path)}
					<a
						class="border border-gray-400 p-2 rounded-sm shadow-sm  hover:border-blue-600 text-gray-800"
						href="/scripts/get/{hash}"
					>
						<div class="flex flex-col gap-1 w-full h-full">
							<div class="font-semibold text-gray-700 truncate">
								{#if marked}
									{@html marked}
								{:else}
									{!summary || summary.length == 0 ? path : summary}
								{/if}
							</div>
							<div class="flex flex-row  justify-between w-full grow gap-2 items-start">
								<div class="text-gray-700 text-xs flex flex-row  flex-wrap  gap-x-1 items-center">
									{path}
									<Star kind="script" {path} {starred} on:starred={loadScripts} />
									<SharedBadge {canWrite} extraPerms={extra_perms} />
									<div><LanguageIcon height={16} lang={language} /></div>
									{#if kind != 'script'}
										<Badge color="blue" capitalize>{kind}</Badge>
									{/if}
									{#if lock_error_logs}
										<Badge color="red">Deployment error</Badge>
									{/if}
								</div>
								<div class="flex flex-col items-end grow pt-4">
									<div class="flex flex-row-reverse place gap-x-2 items-end">
										<div>
											<Dropdown
												dropdownItems={[
													{
														displayName: 'View script',
														icon: faEye,
														href: `/scripts/get/${hash}`
													},
													{
														displayName: 'Edit',
														icon: faEdit,
														href: `/scripts/edit/${hash}`,
														disabled: !canWrite
													},
													{
														displayName: 'Edit code',
														icon: faEdit,
														href: `/scripts/edit/${hash}?step=2`,
														disabled: !canWrite
													},
													{
														displayName: 'Use as template',
														icon: faCodeFork,
														href: `/scripts/add?template=${path}`
													},
													{
														displayName: 'View runs',
														icon: faList,
														href: `/runs/${path}`
													},
													{
														displayName: 'Schedule',
														icon: faCalendarAlt,
														href: `/schedule/add?path=${path}`
													},
													{
														displayName: 'Share',
														icon: faShare,
														action: () => {
															shareModal.openDrawer(path)
														},
														disabled: !canWrite
													},
													{
														displayName: 'Archive',
														icon: faArchive,
														action: () => {
															path ? archiveScript(path) : null
														},
														type: 'delete',
														disabled: !canWrite
													}
												]}
											/>
										</div>
										<div>
											<Button
												color="dark"
												size="xs"
												startIcon={{ icon: faPlay }}
												href="/scripts/run/{hash}"
											>
												Run
											</Button>
										</div>
										{#if canWrite}
											<div>
												<Button
													variant="border"
													color="dark"
													size="xs"
													startIcon={{ icon: faEdit }}
													href="/scripts/edit/{hash}?step=2"
												>
													Edit
												</Button>
											</div>
										{:else}
											<div>
												<Button
													color="dark"
													variant="border"
													size="xs"
													startIcon={{ icon: faCodeFork }}
													href="/scripts/add?template={path}"
												>
													Fork
												</Button>
											</div>
										{/if}
									</div>
								</div></div
							>
						</div></a
					>
				{/each}
			{:else}
				{#each Array(10).fill(0) as sk}
					<Skeleton layout={[[4]]} />
				{/each}
			{/if}
		</div>
	{:else}
		<PickHubScript on:pick={(e) => viewCode(e.detail)} />
	{/if}
</CenteredPage>

<ShareModal
	bind:this={shareModal}
	kind="script"
	on:change={() => {
		loadScripts()
	}}
/>
