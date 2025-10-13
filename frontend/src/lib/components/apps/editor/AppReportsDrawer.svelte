<script lang="ts">
	import { Drawer } from '$lib/components/common'

	import AppReportsDrawerInner from './AppReportsDrawerInner.svelte'
	interface Props {
		appPath: string
		open?: boolean
	}

	let { appPath, open = $bindable(false) }: Props = $props()
</script>

<Drawer bind:open size="800px">
	<AppReportsDrawerInner bind:open {appPath} />
</Drawer>
