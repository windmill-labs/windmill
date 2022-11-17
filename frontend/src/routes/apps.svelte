<script lang="ts">
	import { goto } from '$app/navigation'
	import type { App } from '$lib/components/apps/types'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { DrawerContent, Skeleton } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Path from '$lib/components/Path.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { AppService, ListableApp, Policy } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { faEdit, faPlus } from '@fortawesome/free-solid-svg-icons'

	let path: string = ''
	let initialPath = ''
	let pathError = ''

	let apps: ListableApp[] | undefined = undefined

	async function createApp() {
		const appJson: App = {
			grid: [],
			title: 'New app'
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

	async function loadApps(): Promise<void> {
		apps = await AppService.listApps({ workspace: $workspaceStore! })
	}

	let drawerOpen = false

	function closeDrawer() {
		drawerOpen = false
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadApps()
		}
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

<CenteredPage>
	<PageHeader title="Apps">
		<Button size="sm" startIcon={{ icon: faPlus }} on:click={() => (drawerOpen = true)}>
			New app
		</Button>
	</PageHeader>

	<div class="p-4 border ">
		{#if !apps}
			<div class="grid gap-4 sm:grid-cols-1 md:grid-cols-2 xl:grid-cols-3 mt-2">
				{#each new Array(3) as _}
					<Skeleton layout={[[8.5]]} />
				{/each}
			</div>
		{:else if apps?.length == 0}
			<p class="text-xs text-gray-600 italic mt-2">No scripts yet</p>
		{:else if apps}
			<div class="grid md:grid-cols-2 gap-4 sm:grid-cols-1 xl:grid-cols-3 mt-2">
				{#each apps as { summary, path, extra_perms }}
					<a
						class="border p-4 rounded-sm shadow-sm space-y-2 hover:border-blue-600 text-gray-800 flex flex-col justify-between"
						href="/apps/get/{path}"
					>
						<div class="flex flex-col gap-1">
							<a href="/apps/get/{path}" class="px-6">
								<div class="font-semibold text-gray-700">
									{!summary || summary.length == 0 ? path : summary}
								</div>
								<p class="text-gray-700 text-xs">
									{path}
								</p>
							</a>
							<div class="flex flex-wrap items-center gap-2 mt-1 px-6">
								<SharedBadge canWrite={true} extraPerms={extra_perms} />
							</div>
						</div>
						<div class="flex flex-row-reverse items-end w-full gap-2 pr-2 mt-2">
							<div>
								<Button
									variant="border"
									size="xs"
									startIcon={{ icon: faEdit }}
									href="/apps/edit/{path}"
								>
									Edit
								</Button>
							</div>
						</div>
					</a>
				{/each}
			</div>
		{/if}
	</div>
</CenteredPage>
