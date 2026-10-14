<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import ResourceVersionHistory from '$lib/components/ResourceVersionHistory.svelte'
	import { workspaceStore } from '$lib/stores'
	import { markAgentWritten } from './agentEditorStore.svelte'

	interface Props {
		path: string
		canRestore: boolean
		onRestored?: () => void
	}

	let { path, canRestore, onRestored }: Props = $props()
	let drawer: Drawer | undefined = $state()

	export function open() {
		drawer?.openDrawer()
	}
</script>

<Drawer bind:this={drawer} size="1200px">
	<DrawerContent title="Version history" on:close={() => drawer?.closeDrawer()} noPadding>
		<ResourceVersionHistory
			{path}
			{canRestore}
			onRestore={() => {
				drawer?.closeDrawer()
				// A restore writes the resource as a deploy does, so whatever shows this agent re-reads it.
				markAgentWritten($workspaceStore ?? undefined, path)
				onRestored?.()
			}}
		/>
	</DrawerContent>
</Drawer>
