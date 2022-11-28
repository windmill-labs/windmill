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
		faCalendarAlt,
		faCodeFork,
		faEdit,
		faEye,
		faGlobe,
		faList,
		faPlay,
		faShare
	} from '@fortawesome/free-solid-svg-icons'
	import Fuse from 'fuse.js'
	import type { Script } from '$lib/gen'
	import { ScriptService } from '$lib/gen'
	import { superadmin, userStore, workspaceStore, hubScripts } from '$lib/stores'
	import {
		canWrite,
		getScriptByPath,
		groupBy,
		loadHubScripts,
		sendUserToast,
		truncateHash
	} from '$lib/utils'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { Button, Tabs, Tab, Badge, Skeleton, DrawerContent } from '$lib/components/common'
	import CreateActions from '$lib/components/scripts/CreateActions.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import type { HubItem } from '$lib/components/flows/pickers/model'

	type Tab = 'all' | 'personal' | 'groups' | 'shared' | 'examples' | 'hub'
	type Section = [string, ScriptW[]]
	type ScriptW = Script & { canWrite: boolean; tab: Tab }
	let scripts: ScriptW[] = []
	let filteredScripts: ScriptW[]
	let scriptFilter = ''
	let hubFilter = ''
	let groupedScripts: Section[] = []
	let communityScripts: Section[] = []
	let loading = true

	let tab: Tab = 'all'

	let shareModal: ShareModal

	const fuseOptions = {
		includeScore: false,
		keys: ['description', 'path', 'content', 'hash', 'summary']
	}
	const fuse: Fuse<ScriptW> = new Fuse(scripts, fuseOptions)

	const hubScriptsFuse: Fuse<any> = new Fuse($hubScripts ?? [], {
		includeScore: false,
		keys: ['app', 'path', 'summary']
	})

	let codeViewer: Drawer
	let codeViewerContent: string = ''
	let codeViewerLanguage: 'deno' | 'python3' | 'go' | 'bash' = 'deno'
	let codeViewerObj: HubItem | undefined = undefined

	$: filteredScripts =
		scriptFilter.length > 0 ? fuse.search(scriptFilter).map((value) => value.item) : scripts

	$: {
		let defaults: string[] = []

		if (tab == 'all' || tab == 'personal') {
			defaults = defaults.concat(`u/${$userStore?.username}`)
		}
		if (tab == 'all' || tab == 'groups') {
			defaults = defaults.concat($userStore?.groups.map((x) => `g/${x}`) ?? [])
		}
		groupedScripts = groupBy(
			filteredScripts.filter((x) => x.tab != 'examples'),
			(sc: Script) => sc.path.split('/').slice(0, 2).join('/'),
			defaults
		)
		communityScripts = [['examples', filteredScripts.filter((x) => x.tab == 'examples')]]
	}

	function tabFromPath(path: string) {
		let t: Tab = 'shared'
		let path_prefix = path.split('/').slice(0, 2)
		if (path_prefix[0] == 'u' && path_prefix[1] == $userStore?.username) {
			t = 'personal'
		} else if (path_prefix[0] == 'g' && $userStore?.groups.includes(path_prefix[1])) {
			t = 'groups'
		}
		return t
	}

	async function loadScripts(): Promise<void> {
		const allScripts = (
			await ScriptService.listScripts({ workspace: $workspaceStore!, perPage: 300 })
		).map((x: Script) => {
			let t: Tab = x.workspace_id == $workspaceStore ? tabFromPath(x.path) : 'examples'
			return {
				canWrite: canWrite(x.path, x.extra_perms, $userStore) && x.workspace_id == $workspaceStore,
				tab: t,
				...x
			}
		})
		scripts = tab == 'all' ? allScripts : allScripts.filter((x) => x.tab == tab)
		loading = false
		fuse.setCollection(scripts)
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

	async function loadHubScriptsWFuse(): Promise<void> {
		await loadHubScripts()
		hubScriptsFuse.setCollection($hubScripts ?? [])
	}

	loadHubScriptsWFuse()

	$: {
		if ($workspaceStore && ($userStore || $superadmin) && tab) {
			loadScripts()
		}
	}
</script>

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
		<Tab value="all">All</Tab>
		<Tab value="hub">Hub</Tab>
		<Tab value="personal">{`Personal space (${$userStore?.username})`}</Tab>
		<Tab value="groups">Groups</Tab>
		<Tab value="shared">Shared</Tab>
		<Tab value="examples">Examples</Tab>
	</Tabs>

	{#if tab != 'hub'}
		<input
			type="text"
			placeholder="Search scripts"
			bind:value={scriptFilter}
			class="search-bar mt-2"
		/>
	{/if}

	<div class="grid grid-cols-1 divide-y">
		{#each tab == 'all' ? ['personal', 'groups', 'shared', 'examples', 'hub'] : [tab] as sectionTab}
			<div class="shadow p-4 my-2">
				{#if sectionTab == 'personal'}
					<h2>
						<span class="text-lg xl:text-xl">Personal</span>
						<span class="text-sm">
							({`u/${$userStore?.username}`}) <Tooltip>
								All scripts owned by you (and visible only to you if you do not explicitly share
								them)
							</Tooltip>
						</span>
					</h2>
				{:else if sectionTab == 'groups'}
					<h2 class="text-lg xl:text-xl">
						Groups <Tooltip>All scripts being owned by groups that you are member of</Tooltip>
					</h2>
				{:else if sectionTab == 'shared'}
					<h2 class="text-lg xl:text-xl">
						Shared <Tooltip>All scripts visible to you because they have been shared to you</Tooltip
						>
					</h2>
				{:else if sectionTab == 'examples'}
					<h2 class="text-lg xl:text-xl">
						Public <Tooltip>
							Template and examples shared across all workspaces of this instance. They are managed
							from a special workspace called 'starter' that only superadmin can change.
						</Tooltip>
					</h2>
				{:else if sectionTab == 'hub'}
					<h2 class="text-lg xl:text-xl mb-4">
						Approved scripts from the WindmillHub <Tooltip>
							All approved Deno scripts from the <a href="https://hub.windmill.dev">WindmillHub</a>.
							Approved scripts have been reviewed by the Windmill team and are safe to use in
							production.
						</Tooltip>
					</h2>

					<div class="flex flex-col max-h-screen">
						{#if $hubScripts != undefined}
							<PickHubScript on:pick={(e) => viewCode(e.detail)} />
						{:else}
							<span class="mt-2 text-sm">
								Hub not reachable. If your environment is air gapped, contact sales@windmill.dev to
								setup a local mirror.
							</span>
						{/if}
					</div>
				{/if}
				{#each sectionTab == 'examples' ? communityScripts : groupedScripts.filter((x) => tabFromPath(x[0]) == sectionTab) as [section, scripts]}
					{#if sectionTab != 'personal' && sectionTab != 'examples'}
						<div class="font-bold text-gray-700 mt-2 mb-2">
							{section}
							{#if section == 'g/all'}
								<Tooltip
									>'g/all' is the namespace for the group all. Every user is a member of all.
									Everything in this namespace is visible by all users. At the opposite, 'u/myuser'
									are private user namespaces.</Tooltip
								>
							{/if}
						</div>
					{/if}
					{#if loading}
						<div class="grid gap-4 sm:grid-cols-1 md:grid-cols-2 xl:grid-cols-3 mt-2">
							{#each new Array(3) as _}
								<Skeleton layout={[[8.5]]} />
							{/each}
						</div>
					{:else if scripts.length == 0 && sectionTab == 'personal'}
						<p class="text-xs text-gray-600 italic mt-2">No scripts yet</p>
					{:else}
						<div class="grid md:grid-cols-2 gap-4 sm:grid-cols-1 xl:grid-cols-3 mt-2">
							{#each scripts as { summary, path, hash, language, extra_perms, canWrite, lock_error_logs, kind }}
								<a
									class="border border-gray-400 p-4 rounded-sm shadow-sm space-y-2 hover:border-blue-600 text-gray-800 flex flex-col justify-between"
									href="/scripts/get/{hash}"
								>
									<div class="flex flex-col gap-1">
										<a href="/scripts/get/{hash}" class="px-6">
											<div class="font-semibold text-gray-700">
												{!summary || summary.length == 0 ? path : summary}
											</div>
											<p class="text-gray-700 text-xs">
												{path}
												<Badge color="gray" baseClass="text-xs">{truncateHash(hash)}</Badge>
											</p>
										</a>
										<div class="flex flex-wrap items-center gap-2 mt-1 px-6">
											<SharedBadge {canWrite} extraPerms={extra_perms} />
											<Badge color="blue" capitalize>{language}</Badge>
											{#if kind != 'script'}
												<Badge color="blue" capitalize>{kind}</Badge>
											{/if}
											{#if lock_error_logs}
												<Badge color="red">Deployment error</Badge>
											{/if}
										</div>
									</div>
									<div class="flex flex-row-reverse items-end w-full gap-2 pr-2 mt-2">
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
										{#if canWrite}
											<div>
												<Button
													variant="border"
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
													variant="border"
													size="xs"
													startIcon={{ icon: faCodeFork }}
													href="/scripts/add?template={path}"
												>
													Fork
												</Button>
											</div>
										{/if}
										<div>
											<Button
												variant="border"
												size="xs"
												startIcon={{ icon: faPlay }}
												href="/scripts/run/{hash}"
											>
												Run
											</Button>
										</div>
									</div>
								</a>
							{/each}
						</div>
					{/if}
				{/each}
			</div>
		{/each}
	</div>
</CenteredPage>

<ShareModal
	bind:this={shareModal}
	kind="script"
	on:change={() => {
		loadScripts()
	}}
/>
