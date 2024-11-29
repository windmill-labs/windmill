<script lang="ts">
	import { Drawer } from '$lib/components/common'

	import AppReportsDrawerInner from './AppReportsDrawerInner.svelte'
	export let appPath: string
	export let open = false
</script>

<Drawer bind:open size="800px">
	<AppReportsDrawerInner bind:open {appPath} />
</Drawer>
