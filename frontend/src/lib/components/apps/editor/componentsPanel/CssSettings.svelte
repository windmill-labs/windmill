<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { AlertTriangle, GitBranch, Info } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import type { AppViewerContext } from '../../types'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import CssHelperPanel from './CssHelperPanel.svelte'
	import { premiumStore, workspaceStore } from '$lib/stores'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import ThemeList from './ThemeList.svelte'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { resolveTheme } from './themeUtils'

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

<Tabs bind:selected={selectedTab}>
	<Tab size="xs" value="css">Code</Tab>
	<Tab size="xs" value="theme">Theme</Tab>
	<svelte:fragment slot="content">
		{#if selectedTab === 'css'}
			<SplitPanesWrapper>
				<Splitpanes horizontal>
					<Pane size={60}>
						{#if !$premiumStore.premium}
							<div bind:clientHeight={alertHeight} class="p-2 flex flex-row gap-2">
								<div class="flex flex-row gap-2 items-center text-yellow-500 text-xs">
									<AlertTriangle size={16} />
									EE only
									<Tooltip light>
										App CSS editor is an exclusive feature of the Enterprise Edition. You can
										experiment with this feature in the editor, but please note that the changes
										will not be visible in the preview.
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
							{#if $app.theme?.type === 'inlined'}
								<SimpleEditor
									class="h-full"
									lang="css"
									bind:code={$app.theme.css}
									fixedOverflowWidgets={false}
									small
									automaticLayout
									bind:this={cssEditor}
									deno={false}
								/>
							{/if}

							{#if $app.theme?.type === 'path'}
								<Button
									variant="border"
									color="light"
									on:click={async () => {
										const theme = await resolveTheme($app.theme, $workspaceStore)
										$app.theme = {
											type: 'inlined',
											css: theme
										}
									}}
								>
									<div class="flex flex-row gap-2 items-center">
										<GitBranch size={16} />
										Fork theme
									</div>
								</Button>
							{/if}
						</div>
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
			</SplitPanesWrapper>
		{/if}
		{#if selectedTab === 'theme'}
			<ThemeList cssString={$app?.theme?.type === 'inlined' ? $app.theme.css : undefined} />
		{/if}
	</svelte:fragment>
</Tabs>
