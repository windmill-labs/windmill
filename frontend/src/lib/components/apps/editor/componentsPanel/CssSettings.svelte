<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { AlertTriangle, Info } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import type { AppViewerContext } from '../../types'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import CssHelperPanel from './CssHelperPanel.svelte'
	import { premiumStore } from '$lib/stores'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Drawer, DrawerContent, Tab, TabContent, Tabs } from '$lib/components/common'
	import ThemeList from './ThemeList.svelte'

	const { app, cssEditorOpen } = getContext<AppViewerContext>('AppViewerContext')

	let cssEditor: SimpleEditor | undefined = undefined
	let alertHeight: number | undefined = undefined
	let themeViewer: any = undefined

	let selectedTab: 'css' | 'theme' = 'css'
	onMount(() => {
		$cssEditorOpen = true
	})
</script>

<Drawer bind:this={themeViewer} size="800px">
	<DrawerContent title="View themes" on:close={themeViewer.closeDrawer}>sa</DrawerContent>
</Drawer>

<Splitpanes horizontal>
	<Pane size={60}>
		<Tabs bind:selected={selectedTab}>
			<Tab size="xs" value="css">Code</Tab>
			<Tab size="xs" value="theme">Theme</Tab>
			<svelte:fragment slot="content">
				<TabContent value="css" class="h-full">
					{#if !$premiumStore.premium}
						<div bind:clientHeight={alertHeight} class="p-2 flex flex-row gap-2">
							<div class="flex flex-row gap-2 items-center text-yellow-500 text-xs">
								<AlertTriangle size={16} />
								EE only
								<Tooltip light>
									App CSS editor is an exclusive feature of the Enterprise Edition. You can
									experiment with this feature in the editor, but please note that the changes will
									not be visible in the preview.
								</Tooltip>
							</div>
							<div class="flex flex-row gap-2 items-center text-blue-500 text-xs">
								<Info size={16} />
								Component styling is still available in the Community Edition
								<Tooltip light>
									You can still style components individually in the Community Edition.
								</Tooltip>
							</div>
						</div>
					{/if}
					<div style="height: calc(100% - {alertHeight || 0}px);">
						<SimpleEditor
							class="h-full"
							lang="css"
							bind:code={$app.cssString}
							fixedOverflowWidgets={false}
							small
							automaticLayout
							bind:this={cssEditor}
							deno={false}
						/>
					</div>
				</TabContent>
				<TabContent value="theme">
					<ThemeList />
				</TabContent>
			</svelte:fragment>
		</Tabs>
	</Pane>
	<Pane size={40}>
		<CssHelperPanel
			on:insertSelector={(e) => {
				const code = cssEditor?.getCode()
				cssEditor?.setCode(code + '\n' + e.detail)
				$app = $app
			}}
		/>
	</Pane>
</Splitpanes>
