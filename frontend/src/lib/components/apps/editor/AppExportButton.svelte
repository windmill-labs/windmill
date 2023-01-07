<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'

	import { copyToClipboard } from '$lib/utils'

	import { faClipboard } from '@fortawesome/free-solid-svg-icons'
	import { Highlight } from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { Button } from '../../common'
	import type { App } from '../types'

	let jsonViewerDrawer: Drawer

	export function open() {
		jsonViewerDrawer?.toggleDrawer()
	}

	export let app: App
</script>

<Drawer bind:this={jsonViewerDrawer} size="800px">
	<DrawerContent title="App JSON" on:close={() => jsonViewerDrawer.toggleDrawer()}>
		<div class="relative">
			<Button
				on:click={() => copyToClipboard(JSON.stringify(app, null, 4))}
				color="dark"
				variant="border"
				size="sm"
				startIcon={{ icon: faClipboard }}
				btnClasses="absolute top-2 right-2"
			>
				Copy content
			</Button>
			<Highlight language={json} code={JSON.stringify(app, null, 4)} />
		</div>
	</DrawerContent>
</Drawer>
