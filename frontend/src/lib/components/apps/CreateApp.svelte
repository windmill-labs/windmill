<script lang="ts">
	import { faDashboard, faPlus } from '@fortawesome/free-solid-svg-icons'
	import { AppService, Policy } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'

	let path: string = ''
	let initialPath = ''
	let pathError = ''

	import type { App } from '$lib/components/apps/types'
	import { goto } from '$app/navigation'
	import Path from '$lib/components/Path.svelte'
	import { LayoutDashboard } from 'lucide-svelte'

	let drawerOpen = false

	function closeDrawer() {
		drawerOpen = false
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

<Button
	size="sm"
	spacingSize="lg"
	startIcon={{ icon: faPlus }}
	on:click={() => (drawerOpen = true)}
>
	New App (alpha) <LayoutDashboard class="ml-1.5" size={18} />
</Button>
