<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import { AppService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { deployRawAppValue } from '$lib/rawAppDeploy'
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
		if (app.raw_app) {
			// A raw version restores through the raw endpoint, which rebuilds its
			// bundle. `allowKindChange` so the last version from before a conversion
			// restores the app as raw — the way to undo one. The low-code direction
			// stays refused: it has no bundle to write.
			await deployRawAppValue({
				workspace: $workspaceStore!,
				path: app.path,
				value: app.value,
				summary: app.summary,
				policy: app.policy,
				customPath: app.custom_path,
				allowKindChange: true
			})
		} else {
			await AppService.updateApp({
				workspace: $workspaceStore!,
				path: app.path,
				requestBody: {
					...app
				}
			})
		}

		historyBrowserDrawerOpen = false
	}
</script>

<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
	<DrawerContent title="Deployment History" on:close={() => (historyBrowserDrawerOpen = false)}>
		<DeploymentHistory
			on:restore={async (e) => {
				try {
					await updateApp(e.detail)
					sendUserToast('App restored from previous deployment')
				} catch (err: any) {
					sendUserToast(`Could not restore app: ${err.body ?? err.message}`, true)
				}
			}}
			{appPath}
			on:close={() => {
				historyBrowserDrawerOpen = false
			}}
		/>
	</DrawerContent>
</Drawer>
