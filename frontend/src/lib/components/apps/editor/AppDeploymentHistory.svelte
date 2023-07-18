<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import DeploymentHistory from './DeploymentHistory.svelte'

	let versions: number[] = []

	let historyBrowserDrawerOpen = false

	export function open(newVersions: number[]) {
		versions = newVersions
		historyBrowserDrawerOpen = true
	}
</script>

<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
	<DrawerContent title="Deployment History" on:close={() => (historyBrowserDrawerOpen = false)}>
		<DeploymentHistory on:restore {versions} />
	</DrawerContent>
</Drawer>
