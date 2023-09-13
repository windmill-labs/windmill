<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { AlertTriangle, GitBranch, Info } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import type { AppViewerContext } from '../../types'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import CssHelperPanel from './CssHelperPanel.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import ThemeList from './ThemeList.svelte'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { resolveTheme } from './themeUtils'
	import ThemeCodePreview from './ThemeCodePreview.svelte'
	import { sendUserToast } from '$lib/toast'

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
						{#if $enterpriseLicense === undefined}
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
							{:else}
								<ThemeCodePreview theme={$app.theme} />
							{/if}

							<div class="p-2">
								{#if $app.theme?.type === 'path'}
									<Button
										size="xs"
										color="dark"
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
											Fork theme to edit
										</div>
									</Button>
								{/if}
							</div>
						</div>
					</Pane>
					<Pane size={40}>
						<CssHelperPanel
							on:insertSelector={(e) => {
								if ($app?.theme?.type === 'path') {
									sendUserToast(
										'You cannot edit the theme because it is a path theme. Fork the theme to edit it.',
										true
									)
									return
								}

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
			<ThemeList
				on:setCodeTab={() => {
					selectedTab = 'css'
				}}
			/>
		{/if}
	</svelte:fragment>
</Tabs>
