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
		faList,
		faPlay,
		faPlus,
		faShare
	} from '@fortawesome/free-solid-svg-icons'
	import Fuse from 'fuse.js'
	import Icon from 'svelte-awesome'
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
	import Badge from '$lib/components/Badge.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import Modal from '$lib/components/Modal.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Tabs from '$lib/components/Tabs.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { Highlight } from 'svelte-highlight'
	import { typescript } from 'svelte-highlight/languages/typescript'
	import { Button } from '$lib/components/common'

	type Tab = 'all' | 'personal' | 'groups' | 'shared' | 'examples' | 'hub'
	type Section = [string, ScriptW[]]
	type ScriptW = Script & { canWrite: boolean; tab: Tab }
	let scripts: ScriptW[] = []
	let filteredScripts: ScriptW[]
	let scriptFilter = ''
	let hubFilter = ''
	let groupedScripts: Section[] = []
	let communityScripts: Section[] = []

	let templateModal: Modal
	let templateScripts: Script[] = []
	let templateFilter = ''
	let filteredTemplates: Script[] | undefined
	let tab: Tab = 'all'

	let shareModal: ShareModal

	const fuseOptions = {
		includeScore: false,
		keys: ['description', 'path', 'content', 'hash', 'summary']
	}
	const fuse: Fuse<ScriptW> = new Fuse(scripts, fuseOptions)

	const templateFuse: Fuse<Script> = new Fuse(templateScripts, fuseOptions)

	const hubScriptsFuse: Fuse<any> = new Fuse($hubScripts ?? [], {
		includeScore: false,
		keys: ['app', 'path', 'summary']
	})

	let codeViewer: Modal
	let codeViewerContent: string = ''
	let codeViewerPath: string = ''

	$: filteredScripts =
		scriptFilter.length > 0 ? fuse.search(scriptFilter).map((value) => value.item) : scripts

	$: filteredHub =
		hubFilter.length > 0
			? hubScriptsFuse.search(hubFilter).map((value) => value.item)
			: $hubScripts ?? []

	$: filteredTemplates =
		templateFilter.length > 0
			? templateFuse.search(templateFilter).map((value) => value.item)
			: templateScripts

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

	async function loadTemplateScripts(): Promise<void> {
		templateScripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			isTemplate: true
		})
		templateFuse.setCollection(templateScripts)
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
		fuse.setCollection(scripts)
	}

	async function archiveScript(path: string): Promise<void> {
		await ScriptService.archiveScriptByPath({ workspace: $workspaceStore!, path })
		loadScripts()
		sendUserToast(`Successfully archived script ${path}`)
	}

	async function viewCode(path: string) {
		codeViewerContent = (await getScriptByPath(path)).content
		codeViewerPath = path
		codeViewer.openModal()
	}

	async function loadHubScriptsWFuse(): Promise<void> {
		await loadHubScripts()
		hubScriptsFuse.setCollection($hubScripts ?? [])
	}

	loadHubScriptsWFuse()

	$: {
		if ($workspaceStore && ($userStore || $superadmin)) {
			loadScripts()
		}
	}
</script>

<Modal bind:this={codeViewer}>
	<div slot="title">{codeViewerPath}</div>
	<div slot="content">
		<Highlight language={typescript} code={codeViewerContent} />
	</div></Modal
>

<CenteredPage>
	<PageHeader
		title="Scripts"
		tooltip="A script can either be used standalone or as part of a Flow. 
		When standalone, it has an auto-generated UI from its parameters whom you can access clicking on 'Run'.
		Like everything in windmill, scripts have owners (users or groups) and can be shared to other users and other groups. It is enough to have
		read-access on a script to be able to execute it. However, you will also need to have been
		granted visibility on the resources and variables it uses, otherwise it will behave as if those
		items did not exist at runtime of the script."
	>
		<div class="flex flex-row gap-2">
			<Button
				variant="border"
				size="sm"
				startIcon={{ icon: faPlus }}
				on:click={() => {
					templateModal.openModal()
				}}
			>
				New script from template
			</Button>
			<Button size="sm" startIcon={{ icon: faPlus }} href="/scripts/add">New script</Button>
		</div>
	</PageHeader>

	<Tabs
		tabs={[
			['all', 'all'],
			['hub', 'hub'],
			['personal', `personal space (${$userStore?.username})`],
			['groups', 'groups'],
			['shared', 'shared'],
			['examples', 'examples']
		]}
		bind:tab
		on:update={loadScripts}
	/>
	{#if tab != 'hub'}
		<input placeholder="Search scripts" bind:value={scriptFilter} class="search-bar mt-2" />
	{/if}
	<div class="grid grid-cols-1 divide-y">
		{#each tab == 'all' ? ['personal', 'groups', 'shared', 'examples', 'hub'] : [tab] as sectionTab}
			<div class="shadow p-4 my-2">
				{#if sectionTab == 'personal'}
					<h2 class="mb-2">
						Personal <span class="text-sm"
							>({`u/${$userStore?.username}`}) <Tooltip>
								All scripts owned by you (and visible only to you if you do not explicitly share
								them)
							</Tooltip></span
						>
					</h2>
				{:else if sectionTab == 'groups'}
					<h2 class="">
						Groups <Tooltip>All scripts being owned by groups that you are member of</Tooltip>
					</h2>
				{:else if sectionTab == 'shared'}
					<h2 class="">
						Shared <Tooltip>All scripts visible to you because they have been shared to you</Tooltip
						>
					</h2>
				{:else if sectionTab == 'examples'}
					<h2 class="">
						Public <Tooltip>
							Template and examples shared across all workspaces of this instance. They are managed
							from a special workspace called 'starter' that only superadmin can change.
						</Tooltip>
					</h2>
				{:else if sectionTab == 'hub'}
					<h2 class="">
						Approved scripts from the WindmillHub <Tooltip>
							All approved Deno scripts from the <a href="https://hub.windmill.dev">WindmillHub</a>.
							Approved scripts have been reviewed by the Windmill team and are safe to use in
							production. The hub only offers Deno scripts because Hub scripts are meant to be
							solely used as building blocks of flows and are much more efficient to execute than
							their Python counterparts.
						</Tooltip>
					</h2>
					<input placeholder="Search hub scripts" bind:value={hubFilter} class="search-bar mt-2" />
					<div class="relative">
						{#if $hubScripts != undefined}
							<TableCustom>
								<tr slot="header-row">
									<th>App</th>
									<th>Summary</th>
									<th />
								</tr>
								<tbody slot="body">
									{#each filteredHub ?? [] as { path, summary, app, ask_id }}
										<tr>
											<td class="font-black">{app}</td>
											<td
												><button class="text-left" on:click={() => viewCode(path)}>{summary}</button
												></td
											>
											<td class="whitespace-nowrap"
												><button class="text-blue-500" on:click={() => viewCode(path)}
													>view code</button
												>
												|
												<a
													target="_blank"
													href={`https://hub.windmill.dev/scripts/${app}/${ask_id}`}>hub's page</a
												>
												|
												<a class="font-bold" href={`/scripts/add?hub=${encodeURIComponent(path)}`}
													>fork</a
												>
											</td>
										</tr>
									{/each}
								</tbody>
							</TableCustom>
						{:else}<span class="mt-2 text-sm text-red-400"
								>Hub not reachable. If your environment is air gapped, contact sales@windmill.dev to
								setup a local mirror.</span
							>
						{/if}
					</div>
				{/if}
				{#each sectionTab == 'examples' ? communityScripts : groupedScripts.filter((x) => tabFromPath(x[0]) == sectionTab) as [section, scripts]}
					{#if sectionTab != 'personal' && sectionTab != 'examples'}
						<h3 class="mt-2 mb-2">
							{section}
							{#if section == 'g/all'}
								<Tooltip
									>'g/all' is the namespace for the group all. Every user is a member of all.
									Everything in this namespace is visible by all users. At the opposite, 'u/myuser'
									are private user namespaces.</Tooltip
								>
							{/if}
						</h3>
					{/if}
					{#if scripts.length == 0 && sectionTab == 'personal'}
						<p class="text-xs text-gray-600 italic">No scripts yet</p>
					{:else}
						<div class="grid md:grid-cols-2 gap-4 sm:grid-cols-1 xl:grid-cols-3">
							{#each scripts as { summary, path, hash, language, extra_perms, canWrite, lock_error_logs, kind }}
								<div
									class="flex flex-col justify-between script max-w-lg overflow-visible shadow-sm shadow-blue-100 border border-gray-200 bg-gray-50 py-2"
								>
									<a href="/scripts/get/{hash}">
										<div class="px-6 overflow-auto ">
											<div class="font-semibold text-gray-700">
												{!summary || summary.length == 0 ? path : summary}
											</div>
											<p class="text-gray-700 text-xs">
												<a class="text-gray-700 text-xs" href="/scripts/get/{hash}">{path} </a><span
													class="commit-hash ml-3">{truncateHash(hash)}</span
												>
											</p>
										</div>
									</a>
									<div class="flex flex-row pl-6 pr-2 mt-2">
										<div class="mr-3 w-full">
											<SharedBadge {canWrite} extraPerms={extra_perms} />
											<Badge twBgColor="bg-blue-200">{language}</Badge>
											{#if kind != 'script'}
												<Badge twBgColor="bg-blue-300">{kind}</Badge>
											{/if}
											{#if lock_error_logs}<Badge
													twBgColor="bg-red-200"
													tooltip="The script was not deployed due to an error during deployment. See more details about the error on the script page."
													>Deployment error</Badge
												>{/if}
										</div>
										<div class="flex flex-row-reverse w-full place space-x-1">
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
																shareModal.openModal(path)
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
									</div>
								</div>
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

<Modal
	bind:this={templateModal}
	on:open={() => {
		loadTemplateScripts()
	}}
>
	<div slot="title">Pick a template</div>
	<div slot="content">
		<div class="w-12/12 pb-4">
			<input placeholder="Search templates" bind:value={templateFilter} class="search-bar" />
		</div>
		<div class="flex flex-col mb-2 md:mb-6">
			{#if filteredTemplates && filteredTemplates.length > 0}
				{#each filteredTemplates as { summary, path, hash }}
					<a
						class="p-1 flex flex-row items-baseline gap-2 selected text-gray-700"
						href="/scripts/add?template={path}"
					>
						{#if summary}
							<p class="text-sm font-semibold">{summary}</p>
						{/if}

						<p class="text-sm">{path}</p>
						<p class="text-gray-400 text-xs text-right grow">
							Last version: {hash}
						</p>
					</a>
				{/each}
			{:else}
				<p class="text-sm text-gray-700">No templates</p>
			{/if}
		</div>
	</div>
</Modal>

<style>
	.selected:hover {
		@apply border border-gray-500 rounded-md border-opacity-50;
	}

	.script:hover {
		@apply border border-gray-600 border-opacity-60;
	}
</style>
