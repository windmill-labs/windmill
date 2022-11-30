<script context="module">
	export function load() {
		return {
			stuff: { title: 'Scripts' }
		}
	}
</script>

<script lang="ts">
	import { faCodeFork, faEdit, faPlay, faPlus } from '@fortawesome/free-solid-svg-icons'
	import { AppService, Policy, type ListableApp } from '$lib/gen'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, sendUserToast } from '$lib/utils'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { Button, Badge, Skeleton, Drawer, DrawerContent } from '$lib/components/common'

	type ListableAppMarked = ListableApp & { canWrite: boolean; marked?: string }

	let apps: ListableAppMarked[] = []
	let preFilteredApps: ListableAppMarked[] = []
	let filteredApps: ListableAppMarked[] = []
	let filter = ''
	let loading = true

	let path: string = ''
	let initialPath = ''
	let pathError = ''

	async function loadApps(): Promise<void> {
		apps = (await AppService.listApps({ workspace: $workspaceStore! })).map((x) => {
			return {
				canWrite:
					x.path && x.extra_perms
						? canWrite(x.path, x.extra_perms, $userStore) && x.workspace_id == $workspaceStore
						: false,
				...x
			}
		})

		loading = false
	}

	import SearchItems from '$lib/components/SearchItems.svelte'
	import type { App } from '$lib/components/apps/types'
	import { goto } from '$app/navigation'
	import Path from '$lib/components/Path.svelte'

	$: owners = Array.from(
		new Set(filteredApps?.map((x) => x.path?.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	$: preFilteredApps =
		ownerFilter != undefined ? apps.filter((x) => x.path?.startsWith(ownerFilter ?? '')) : apps

	let ownerFilter: string | undefined = undefined

	$: {
		if ($workspaceStore && ($userStore || $superadmin)) {
			loadApps()
		}
	}
	async function createApp() {
		const appJson: App = {
			grid: [],
			title: 'New app',
			inlineScripts: {}
		}

		const policy = {
			triggerables: {},
			execution_mode: Policy.execution_mode.PUBLISHER,
			on_behalf_of: `u/${$userStore?.username}`
		}
		try {
			const appId = await AppService.createApp({
				workspace: $workspaceStore!,
				requestBody: {
					value: appJson,
					path,
					summary: 'App summary',
					policy
				}
			})

			goto(`/apps/edit/${appId}`)
		} catch (e) {
			sendUserToast('Error creating app', e)
		}
	}

	let drawerOpen = false

	function closeDrawer() {
		drawerOpen = false
	}
</script>

<Drawer bind:open={drawerOpen} size="800px">
	<DrawerContent title="Add an app" on:close={() => closeDrawer()}>
		<Path bind:error={pathError} bind:path {initialPath} namePlaceholder="my_app" kind="app">
			<div slot="ownerToolkit">
				App permissions depend on their path. Select the group <span class="font-mono">all</span>
				to share it, and <span class="font-mono">user</span> to keep it private.
				<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
			</div>
		</Path>

		<Button on:click={() => createApp()}>Create app</Button>
	</DrawerContent>
</Drawer>

<SearchItems
	{filter}
	items={preFilteredApps}
	bind:filteredItems={filteredApps}
	f={(x) => x.summary + ' (' + x.path + ')'}
/>

<CenteredPage>
	<PageHeader
		title="Apps"
		tooltip="A Script can be used standalone or as part of a Flow. 
		When standalone, it has webhooks and an auto-generated UI from its parameters whom you can access clicking on 'Run'.
		Scripts have owners (users or groups) and can be shared to users and groups."
	>
		<Button size="md" startIcon={{ icon: faPlus }} on:click={() => (drawerOpen = true)}>
			New app
		</Button>
	</PageHeader>

	<div class="mb-1" />

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
			{#each filteredApps as { summary, path, extra_perms, marked, canWrite }, index (`${path}-${index}`)}
				<a
					class="border border-gray-400 p-2 rounded-sm shadow-sm  hover:border-blue-600 text-gray-800"
					href="/apps/get/{path}"
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
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</div>
							<div class="flex flex-col items-end grow pt-4">
								<div class="flex flex-row-reverse place gap-x-2 items-end">
									<div>
										<Button
											color="dark"
											size="xs"
											startIcon={{ icon: faPlay }}
											href="/apps/get/{path}"
										>
											Preview
										</Button>
									</div>
									{#if canWrite}
										<div>
											<Button
												variant="border"
												color="dark"
												size="xs"
												startIcon={{ icon: faEdit }}
												href="/apps/edit/{path}"
											>
												Edit
											</Button>
										</div>
									{/if}
								</div>
							</div>
						</div>
					</div>
				</a>
			{/each}
		{:else}
			{#each Array(10).fill(0) as _}
				<Skeleton layout={[[4]]} />
			{/each}
		{/if}
	</div>
</CenteredPage>
