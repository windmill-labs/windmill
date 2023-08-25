<script lang="ts">
	import { getContext } from 'svelte'
	import { LayoutDashboardIcon, MousePointer2, CurlyBraces, Code } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { emptyString } from '$lib/utils'
	import { ClearableInput, Drawer, DrawerContent, Tab, TabContent, Tabs } from '../../../common'
	import type { AppViewerContext } from '../../types'
	import ListItem from './ListItem.svelte'
	import CssProperty from './CssProperty.svelte'
	import { ccomponents, components } from '../component'
	import { slide } from 'svelte/transition'
	import Editor from '$lib/components/Editor.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import CssHelperPanel from './CssHelperPanel.svelte'

	const STATIC_ELEMENTS = ['app'] as const
	const TITLE_PREFIX = 'Css.' as const

	type CustomCSSType = (typeof STATIC_ELEMENTS)[number] | keyof typeof components

	interface CustomCSSEntry {
		type: CustomCSSType
		name: string
		icon: any
		ids: { id: string; forceStyle: boolean; forceClass: boolean }[]
	}

	const { app, cssEditorOpen } = getContext<AppViewerContext>('AppViewerContext')

	let rawCode = ''
	let rawCss = ``

	$: rawCode && parseJson()
	$: rawCss && parseCss()
	let jsonError = ''
	let jsonErrorHeight: number
	let cssError = ''
	let cssErrorHeight: number

	let cssEditor: Editor | undefined = undefined

	function parseJson() {
		try {
			$app.css = JSON.parse(rawCode ?? '')
			jsonError = ''
		} catch (e) {
			jsonError = e.message
		}
	}

	function parseCss() {
		$app.cssString = rawCss
	}

	function switchTab(asJson: boolean) {
		if (asJson) {
			rawCode = JSON.stringify($app.css, null, 2)
			console.log($app.cssString, '#########')

			rawCss = $app.cssString ?? ''
		} else {
			parseJson()
			console.log($app.cssString, '#########')
			rawCss = $app.cssString ?? ''
		}
	}

	const entries: CustomCSSEntry[] = [
		{
			type: 'app',
			name: 'App',
			icon: LayoutDashboardIcon,
			ids: ['viewer', 'grid', 'component'].map((id) => ({ id, forceStyle: true, forceClass: true }))
		},
		...Object.entries(ccomponents).map(([type, { name, icon, customCss }]) => ({
			type: type as keyof typeof components,
			name,
			icon,
			ids: Object.entries(customCss).map(([id, v]) => ({
				id,
				forceStyle: v?.style != undefined,
				forceClass: v?.['class'] != undefined
			}))
		}))
	]
	entries.sort((a, b) => a.name.localeCompare(b.name))
	let search = ''

	let cssViewer: Drawer
</script>

<Drawer bind:this={cssViewer} size="800px">
	<DrawerContent title="CSS Details" on:close={cssViewer.closeDrawer} />
</Drawer>

<!-- <div class="w-full text-lg font-semibold text-center text-tertiary p-2">Global Styling</div> -->
<Tabs selected="ui" on:selected={(e) => switchTab(e.detail === 'json')} class="h-full">
	<Tab value="ui" size="xs" class="w-1/2">
		<div class="m-1 center-center">
			<MousePointer2 size={16} />
			<span class="pl-1">UI</span>
		</div>
	</Tab>
	<Tab value="json" size="xs" class="w-1/2">
		<div class="m-1 center-center">
			<CurlyBraces size={16} />
			<span class="pl-1">JSON</span>
		</div>
	</Tab>
	<Tab
		value="css"
		size="xs"
		class="w-1/2"
		on:pointerdown={() => {
			$cssEditorOpen = true
		}}
	>
		<div class="m-1 center-center">
			<Code size={16} />
			<span class="pl-1">Global CSS</span>
		</div>
	</Tab>
	<div slot="content" class="h-[calc(100%-35px)] overflow-auto">
		<TabContent value="ui" class="h-full">
			<div class="p-2">
				<ClearableInput bind:value={search} placeholder="Search..." />
			</div>
			<div class="h-[calc(100%-50px)] overflow-auto relative">
				{#each search != '' ? entries.filter((x) => x.name
								.toLowerCase()
								.includes(search.toLowerCase())) : entries as { type, name, icon, ids } (name + type)}
					{#if ids.length > 0}
						<ListItem
							title={name}
							prefix={TITLE_PREFIX}
							on:open={(e) => {
								if ($app.css != undefined) {
									if (e.detail && $app.css[type] == undefined) {
										$app.css[type] = Object.fromEntries(ids.map(({ id }) => [id, {}]))
									}
								}
							}}
						>
							<div slot="title" class="flex items-center">
								<svelte:component this={icon} size={18} />
								<span class="ml-1">
									{name}
								</span>
							</div>
							<div class="pb-2">
								{#each ids as { id, forceStyle, forceClass }}
									<div class="mb-3">
										{#if $app?.css?.[type]}
											<CssProperty
												{forceClass}
												{forceStyle}
												name={id}
												bind:value={$app.css[type][id]}
											/>
										{/if}
									</div>
								{/each}
							</div>
						</ListItem>
					{/if}
				{/each}
			</div>
		</TabContent>
		<TabContent value="json" class="h-full">
			{#if !emptyString(jsonError)}
				<div
					transition:slide={{ duration: 200 }}
					bind:clientHeight={jsonErrorHeight}
					class="text-red-500 text-xs p-1"
				>
					{jsonError}
				</div>
			{/if}
			<div style="height: calc(100% - {jsonErrorHeight || 0}px);">
				<SimpleEditor class="h-full" lang="json" bind:code={rawCode} fixedOverflowWidgets={false} />
			</div>
		</TabContent>
		<TabContent value="css" class="h-full">
			{#if !emptyString(cssError)}
				<div
					transition:slide={{ duration: 200 }}
					bind:clientHeight={cssErrorHeight}
					class="text-red-500 text-xs p-1"
				>
					{cssError}
				</div>
			{/if}
			<Splitpanes horizontal>
				<Pane size={60}>
					<div style="height: calc(100% - {cssErrorHeight || 0}px);">
						<Editor
							class="h-full"
							lang="css"
							bind:code={rawCss}
							fixedOverflowWidgets={false}
							small
							automaticLayout
							bind:this={cssEditor}
							deno={false}
						/>
					</div>
				</Pane>
				<Pane size={40}>
					<CssHelperPanel
						on:insertSelector={(e) => {
							if (!cssEditor) {
								console.log('cssEditor', cssEditor)
							}

							// append in Editor$
							const code = cssEditor?.getCode()

							cssEditor?.setCode(code + '\n' + e.detail)
							$app = $app
						}}
					/>
				</Pane>
			</Splitpanes>
		</TabContent>
	</div>
</Tabs>
