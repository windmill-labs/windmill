<script lang="ts">
	import AppWrapper from '$lib/components/apps/AppWrapper.svelte'
	import type { App } from '$lib/components/apps/types'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Path from '$lib/components/Path.svelte'
	import { ResourceService, type InputTransform, type Resource } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { canWrite, sendUserToast } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'

	type ResourceW = Resource & { canWrite: boolean }

	let path: string = ''
	let initialPath = ''
	let pathError = ''

	let loading = {
		apps: true
	}
	let apps: ResourceW[] | undefined

	async function createApp() {
		const appJson = {}
		await ResourceService.createResource({
			workspace: $workspaceStore!,
			requestBody: {
				resource_type: 'app',
				value: appJson,
				path,
				description: 'App description'
			}
		})
		sendUserToast(`Successfully created app at ${path}`)
	}

	async function loadApps(): Promise<void> {
		apps = (
			await ResourceService.listResource({ workspace: $workspaceStore!, resourceType: 'app' })
		).map((res) => {
			return {
				canWrite:
					canWrite(res.path, res.extra_perms!, $userStore) && $workspaceStore! == res.workspace_id,
				...res
			}
		})
		loading.apps = false
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

	const app: App = {
		components: [
			{
				id: 'a',
				type: 'runformcomponent',
				runType: 'script',
				path: 'u/faton/my_script_3',
				inputs: {
					runInputs: {
						a: {
							type: 'static',
							value: 'Salut'
						}
					}
				}
			},
			{
				id: 'b',
				type: 'displaycomponent',
				inputs: {
					result: {
						type: 'output',
						id: 'a',
						output: 'result'
					}
				}
			}
		],
		title: 'Fake title'
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
	{JSON.stringify(apps)}

	<AppWrapper {app} />
</CenteredPage>
