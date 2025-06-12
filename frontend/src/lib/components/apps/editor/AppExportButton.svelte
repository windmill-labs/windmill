<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'

	import { copyToClipboard } from '$lib/utils'

	import { Highlight } from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { Button } from '../../common'
	import { Clipboard } from 'lucide-svelte'
	import { yaml } from 'svelte-highlight/languages'
	import YAML from 'yaml'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'

	let jsonViewerDrawer: Drawer

	let app: any | undefined = undefined

	let rawType: 'json' | 'yaml' = 'yaml'

	export function open(app_l: any) {
		app = app_l
		jsonViewerDrawer?.toggleDrawer()
	}
</script>

<HighlightTheme />

<Drawer bind:this={jsonViewerDrawer} size="800px">
	<DrawerContent title="App Export" on:close={() => jsonViewerDrawer.toggleDrawer()}>
		<div>
			<Tabs bind:selected={rawType}>
				<Tab value="yaml">YAML</Tab>
				<Tab value="json">JSON</Tab>
				{#snippet content()}
					<div class="relative pt-2">
						<Button
							on:click={() =>
								copyToClipboard(
									rawType === 'yaml'
										? YAML.stringify(app ?? {})
										: JSON.stringify(app ?? {}, null, 4)
								)}
							color="dark"
							variant="border"
							size="sm"
							startIcon={{ icon: Clipboard }}
							btnClasses="absolute top-2 right-2 w-min z-20"
							iconOnly
						/>
						{#key rawType}
							<Highlight
								class="overflow-auto px-1 flex-1"
								language={rawType === 'yaml' ? yaml : json}
								code={rawType === 'yaml'
									? YAML.stringify(app ?? {})
									: JSON.stringify(app ?? {}, null, 4)}
							/>
						{/key}
					</div>
				{/snippet}
			</Tabs>
		</div></DrawerContent
	>
</Drawer>
