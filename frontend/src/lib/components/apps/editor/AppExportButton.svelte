<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'

	import { copyToClipboard } from '$lib/utils'

	import { Highlight } from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { Button } from '../../common'
	import type { App } from '../types'
	import { Clipboard } from 'lucide-svelte'

	let jsonViewerDrawer: Drawer

	let app: App | undefined = undefined

	export function open(app_l: App) {
		app = app_l
		jsonViewerDrawer?.toggleDrawer()
	}
</script>

<Drawer bind:this={jsonViewerDrawer} size="800px">
	<DrawerContent title="App JSON" on:close={() => jsonViewerDrawer.toggleDrawer()}>
		<div class="relative">
			<Button
				on:click={() => copyToClipboard(JSON.stringify(app, null, 4))}
				color="dark"
				variant="border"
				size="sm"
				startIcon={{ icon: Clipboard }}
				btnClasses="absolute top-2 right-2 w-min"
			>
				Copy content
			</Button>
			<Highlight language={json} code={JSON.stringify(app ?? {}, null, 4)} />
		</div>
	</DrawerContent>
</Drawer>
