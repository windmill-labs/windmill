<script lang="ts">
	import {
		faArchive,
		faCalendarAlt,
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
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, groupBy, sendUserToast, truncateHash } from '$lib/utils'
	import Badge from '$lib/components/Badge.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import Modal from '$lib/components/Modal.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Tabs from '$lib/components/Tabs.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	type Tab = 'all' | 'personal' | 'groups' | 'shared' | 'community'
	type Section = [string, ScriptW[]]
	type ScriptW = Script & { canWrite: boolean; tab: Tab }
	let scripts: ScriptW[] = []
	let filteredScripts: ScriptW[]
	let scriptFilter = ''
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

	$: filteredScripts =
		scriptFilter.length > 0 ? fuse.search(scriptFilter).map((value) => value.item) : scripts

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
			filteredScripts.filter((x) => x.tab != 'community'),
			(sc: Script) => sc.path.split('/').slice(0, 2).join('/'),
			defaults
		)
		communityScripts = [['community', filteredScripts.filter((x) => x.tab == 'community')]]
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
		const allScripts = (await ScriptService.listScripts({ workspace: $workspaceStore! })).map(
			(x: Script) => {
				let t: Tab = x.workspace_id == $workspaceStore ? tabFromPath(x.path) : 'community'
				return {
					canWrite:
						canWrite(x.path, x.extra_perms, $userStore) && x.workspace_id == $workspaceStore,
					tab: t,
					...x
				}
			}
		)
		scripts = tab == 'all' ? allScripts : allScripts.filter((x) => x.tab == tab)
		fuse.setCollection(scripts)
	}

	async function archiveScript(path: string): Promise<void> {
		try {
			await ScriptService.archiveScriptByPath({ workspace: $workspaceStore!, path })
			loadScripts()
			sendUserToast(`Successfully archived script ${path}`)
		} catch (err) {
			sendUserToast(`Could not archive this script ${err.body}`, true)
		}
	}

	$: {
		if ($workspaceStore && ($userStore || $superadmin)) {
			loadScripts()
		}
	}
</script>

<CenteredPage>
	<PageHeader
		title="Scripts"
		tooltip="Scripts are the building blocks of windmill. A script has an auto-generated UI from its
		parameters whom you can access clicking on 'Run...'. Like everything in windmill, scripts have
		owners (users or groups) and can be shared to other users and other groups. It is enough to have
		read-access on a script to be able to execute it. However, you will also need to have been
		granted visibility on the resources and variables it uses, otherwise it will behave as if those
		items did not exist at runtime of the script."
	>
		<div class="flex flex-row">
			<button
				class="default-button-secondary border-none"
				on:click={() => {
					templateModal.openModal()
				}}
				><Icon class="text-blue-500 mb-1" data={faPlus} scale={0.9} /> New script from template</button
			>
			<a class="default-button" href="/scripts/add"
				><Icon class="text-white mb-1" data={faPlus} scale={0.9} /> &nbsp; New script</a
			>
		</div>
	</PageHeader>

	<Tabs
		tabs={[
			['all', 'all'],
			['personal', `personal space (${$userStore?.username})`],
			['groups', 'groups'],
			['shared', 'shared'],
			['community', 'community']
		]}
		bind:tab
		on:update={loadScripts}
	/>
	<input placeholder="Search scripts" bind:value={scriptFilter} class="search-bar mt-2" />
	<div class="grid grid-cols-1 divide-y">
		{#each tab == 'all' ? ['personal', 'groups', 'shared', 'community'] : [tab] as sectionTab}
			<div class="shadow p-4 my-2">
				{#if sectionTab == 'personal'}
					<h2 class="">
						My personal space ({`u/${$userStore?.username}`})
					</h2>
					<p class="italic text-xs text-gray-600 mb-4">
						All scripts owned by you (and visible only to you if you do not explicitely share them)
						will be displayed below
					</p>
				{:else if sectionTab == 'groups'}
					<h2 class="">Groups that I am member of</h2>
					<p class="italic text-xs text-gray-600">
						All scripts being owned by groups that you are member of will be displayed below
					</p>
				{:else if sectionTab == 'shared'}
					<h2 class="">Shared with me</h2>
					<p class="italic text-xs text-gray-600">
						All scripts visible to you because they have been shared to you will be displayed below
					</p>
				{:else if sectionTab == 'community'}
					<h2 class="">Community templates & examples</h2>
					<p class="italic text-xs text-gray-600 mb-8">
						All scripts by the community that went through a review process and merged to the
						<a href="https://github.com/windmill-labs/windmill">official github repo</a> will be displayed
						below. Contributions welcome as Github PR.
					</p>
				{/if}
				{#each sectionTab == 'community' ? communityScripts : groupedScripts.filter((x) => tabFromPath(x[0]) == sectionTab) as [section, scripts]}
					{#if sectionTab != 'personal' && sectionTab != 'community'}
						<h3 class="mt-2 mb-2">
							owner: {section}
							{#if section == 'g/all'}
								<Tooltip class="mx-1"
									>'g/all' is the namespace for the group all. Every user is a member of all.
									Everything in this namespace is visible by all users. At the opposite, 'u/myuser'
									are private user namespaces.</Tooltip
								>
							{/if}
						</h3>
					{/if}
					{#if scripts.length == 0}
						<p class="text-xs text-gray-600 font-black">
							No scripts for this owner space yet. To create one, click on the top right button.
						</p>
					{:else}
						<div class="grid md:grid-cols-2 gap-4 sm:grid-cols-1 2xl:grid-cols-3">
							{#each scripts as { summary, path, hash, language, extra_perms, canWrite, lock_error_logs }}
								<div
									class="flex flex-col justify-between script max-w-lg overflow-visible shadow-sm shadow-blue-100 border border-gray-200 bg-gray-50 py-2"
								>
									<a href="/scripts/get/{hash}">
										<div class="px-6 overflow-auto ">
											<div class="font-semibold text-gray-700">
												{!summary || summary.length == 0 ? path : summary}
											</div>
											<p class="text-gray-700 text-xs">
												<a class="text-gray-700 text-xs" href="/scripts/get/{hash}"
													>Path: {path}
												</a><span class="commit-hash ml-3">{truncateHash(hash)}</span>
											</p>
										</div>
									</a>
									<div class="flex flex-row pl-6 pr-2 mt-2">
										<div class="mr-3 w-full">
											<SharedBadge {canWrite} extraPerms={extra_perms} />
											<Badge twBgColor="bg-blue-200">{language}</Badge>
											{#if lock_error_logs}<Badge
													twBgColor="bg-red-200"
													tooltip="The script was not deployed due to an error during deployment. See more details about the error on the script page."
													>Deployment error</Badge
												>{/if}
										</div>
										<div class="flex flex-row-reverse w-full place">
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
															icon: faEdit,
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
											<div>
												<a
													class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
													href="/scripts/run/{hash}"
												>
													<div class="inline-flex items-center justify-center">
														<Icon data={faPlay} scale={0.5} />
														<span class="pl-1">Run...</span>
													</div>
												</a>
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
