<script lang="ts">
	import { getContext } from 'svelte'
	import { LayoutDashboardIcon, MousePointer2, CurlyBraces } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { emptyString } from '$lib/utils'
	import { ClearableInput, Tab, TabContent, Tabs } from '../../../common'
	import type { AppViewerContext } from '../../types'
	import ListItem from './ListItem.svelte'
	import CssProperty from './CssProperty.svelte'
	import { ccomponents, components } from '../component'
	import { slide } from 'svelte/transition'

	const STATIC_ELEMENTS = ['app'] as const
	const TITLE_PREFIX = 'Css.' as const

	type CustomCSSType = (typeof STATIC_ELEMENTS)[number] | keyof typeof components

	interface CustomCSSEntry {
		type: CustomCSSType
		name: string
		icon: any
		ids: { id: string; forceStyle: boolean; forceClass: boolean }[]
	}

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	let rawCode = ''

	$: rawCode && parseJson()
	let jsonError = ''
	let jsonErrorHeight: number

	function parseJson() {
		try {
			$app.css = JSON.parse(rawCode ?? '')
			jsonError = ''
		} catch (e) {
			jsonError = e.message
		}
	}

	function switchTab(asJson: boolean) {
		if (asJson) {
			rawCode = JSON.stringify($app.css, null, 2)
		} else {
			parseJson()
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
</script>

<!-- <div class="w-full text-lg font-semibold text-center text-gray-600 p-2">Global Styling</div> -->
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
	<div slot="content" class="h-[calc(100%-35px)] overflow-auto">
		<TabContent value="ui">
			<div class="sticky z-10 top-0 left-0 w-full bg-white p-2">
				<ClearableInput bind:value={search} placeholder="Search..." />
			</div>
			{#each search != '' ? entries.filter((x) => x.name
							.toLowerCase()
							.includes(search.toLowerCase())) : entries as { type, name, icon, ids } (name)}
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
	</div>
</Tabs>
