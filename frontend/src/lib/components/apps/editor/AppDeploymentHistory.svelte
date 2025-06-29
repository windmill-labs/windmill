<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import { AppService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import DeploymentHistory from './DeploymentHistory.svelte'

	interface Props {
		appPath?: string | undefined
	}

	let { appPath = undefined }: Props = $props()
	let historyBrowserDrawerOpen = $state(false)

	export function open() {
		historyBrowserDrawerOpen = true
	}

	async function updateApp(app: any) {
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: app.path,
			requestBody: {
				...app
			}
		})

		historyBrowserDrawerOpen = false
	}
</script>

<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
	<DrawerContent title="Deployment History" on:close={() => (historyBrowserDrawerOpen = false)}>
		<DeploymentHistory
			on:restore={(e) => {
				sendUserToast('App restored from previous deployment')
				updateApp(e.detail)
			}}
			{appPath}
			on:close={() => {
				historyBrowserDrawerOpen = false
			}}
		/>
	</DrawerContent>
</Drawer>
