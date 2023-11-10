<script lang="ts">
	import { getContext } from 'svelte'
	import { AlertTriangle, GitBranch } from 'lucide-svelte'
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

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	let cssEditor: SimpleEditor | undefined = undefined
	let alertHeight: number | undefined = undefined
	let themeViewer: any = undefined
	let selectedTab: 'css' | 'theme' = 'css'

	function insertSelector(selector: string) {
		if ($app?.theme?.type === 'path') {
			sendUserToast(
				'You cannot edit the theme because it is a path theme. Fork the theme to edit it.',
				true
			)
			return
		}

		const code = cssEditor?.getCode()
		cssEditor?.setCode(code + '\n' + selector)
		$app = $app
	}
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
								<div class="flex flex-row items-center text-yellow-500 text-xs">
									<div class="flex items-center whitespace-nowrap">
										<AlertTriangle size={16} />
										EE only
									</div>
									<Tooltip light>
										App CSS editor is an exclusive feature of the Enterprise Edition. You can
										experiment with this feature in the editor, but please note that the changes
										will not be visible once deployed.
									</Tooltip>
								</div>
								<div class="flex flex-row items-center text-blue-500 text-xs">
									Component styling available in CE
									<Tooltip light>
										You can still style components in the Community Edition in the styling section
										of the component's configuration.
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
									fixedOverflowWidgets={true}
									small
									automaticLayout
									bind:this={cssEditor}
									deno={false}
								/>
							{:else}
								<ThemeCodePreview theme={$app.theme}>
									<div class="p-2 w-min">
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
											startIcon={{ icon: GitBranch }}
										>
											Fork theme to edit
										</Button>
									</div>
								</ThemeCodePreview>
							{/if}
						</div>
					</Pane>
					<Pane size={40}>
						<CssHelperPanel on:insertSelector={(e) => insertSelector(e.detail)} />
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
