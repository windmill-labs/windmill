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

	// Picking a version is an explicit choice of what the app should become, so a
	// version from before the app changed kind is allowed to convert it back —
	// that is the only supported way to undo an accidental conversion.
	async function updateApp(app: any) {
		if (app.raw_app) {
			// Restoring a raw app version means re-bundling its sources: `updateApp`
			// would write a low-code version and leave the bundle behind.
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
					...app,
					allow_kind_change: true
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
