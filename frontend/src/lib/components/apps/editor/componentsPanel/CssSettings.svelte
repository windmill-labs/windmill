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
	import { validate } from 'csstree-validator'
	import { sanitizeCss } from './cssUtils'

	const STATIC_ELEMENTS = ['app'] as const
	const TITLE_PREFIX = 'Css.' as const

	const authorizedClassnames = [
		'app-component-container', // Migrated
		// List
		'app-component-list-wrapper',
		'app-component-list',

		'app-component-divider-x',
		'app-component-divider-y',
		'app-component-drawer',
		'app-component-vertical-split-panes',
		'app-component-horizontal-split-panes',
		'app-component-modal',
		'app-component-stepper',

		'app-component-tabs',
		'app-component-conditional-tabs',
		'app-component-sidebar-tabs',
		'app-component-invisible-tabs',

		'app-component-button',
		'app-component-button-wrapper',
		'app-component-button-container',

		'app-component-submit',
		'app-component-modal-form',
		'app-component-download-button',

		'app-component-form',
		'app-component-text-input',
		'app-component-textarea',
		'app-component-rich-text-editor',
		'app-component-password',
		'app-component-email-input',
		'app-component-number',
		'app-component-currency',
		'app-component-slider',
		'app-component-range',
		'app-component-date',
		'app-component-file-input',
		'app-component-toggle',
		'app-component-select',
		'app-component-resource-select',
		'app-component-multiselect',
		'app-component-select-tab',
		'app-component-select-step',

		'app-component-table',
		'app-component-aggrid-table',

		'app-component-text',
		'app-component-icon',
		'app-component-image',
		'app-component-map',
		'app-component-html',
		'app-component-pdf',
		'app-component-rich-result',
		'app-component-log',
		'app-component-flow-status',

		'app-component-bar-line-chart',
		'app-component-pie-chart',
		'app-component-vega-lite',
		'app-component-plotly',
		'app-component-scatter-chart',
		'app-component-timeseries',
		'app-component-chartjs'
	]

	type CustomCSSType = (typeof STATIC_ELEMENTS)[number] | keyof typeof components

	interface CustomCSSEntry {
		type: CustomCSSType
		name: string
		icon: any
		ids: { id: string; forceStyle: boolean; forceClass: boolean }[]
	}

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	let rawCode = ''
	let rawCss = `
/* You can only define CSS rules for those classes. Any other classes will be ignored. */
/* Containers */

/* Applied to the root element of the app */
.windmill-app-container {}\n
/* Applied to the root element of the app when in grid mode */
.windmill-app-grid {}\n
.windmill-component-wrapper {}\n

.app-component-container {}\n
.app-component-list {}\n
.app-component-divider-x {}\n
.app-component-divider-y {} 
.app-component-drawer {}\n
.app-component-vertical-split-panes {}\n
.app-component-horizontal-split-panes {}\n
.app-component-modal {}\n
.app-component-stepper {}

/* Tabs */
.app-component-tabs {}\n
.app-component-conditional-tabs {}\n
.app-component-sidebar-tabs {}\n
.app-component-invisible-tabs {}

/* Buttons */
.app-component-button-wrapper {}\n
.app-component-button-wrapper > .app-component-button-container {}\n
.app-component-button-wrapper > .app-component-button-container > .app-component-button {}\n
.app-component-submit {}\n
.app-component-modal-form {}\n
.app-component-download-button {}\n

/* Inputs */
.app-component-form {}\n
.app-component-text-input {}\n
.app-component-textarea {}\n
.app-component-rich-text-editor {}\n
.app-component-password {}\n
.app-component-email-input {}\n
.app-component-number {}\n
.app-component-currency {}\n
.app-component-slider {}\n
.app-component-range {}\n
.app-component-date {}\n
.app-component-file-input {}\n
.app-component-toggle {}\n
.app-component-select {}\n
.app-component-resource-select {}\n
.app-component-multiselect {}\n
.app-component-select-tab {}\n
.app-component-select-step {}\n

/* Tables */
.app-component-table {}\n
.app-component-aggrid-table {}\n

/* Display */
.app-component-text {}\n
.app-component-icon {}\n
.app-component-image {}\n
.app-component-map {}\n
.app-component-html {}\n
.app-component-pdf {}\n
.app-component-rich-result {}\n
.app-component-log {}\n
.app-component-flow-status {}\n

/* Charts */
.app-component-bar-line-chart {}\n
.app-component-pie-chart {}\n
.app-component-vega-lite {}\n
.app-component-plotly {}\n
.app-component-scatter-chart {}\n
.app-component-timeseries {}\n
.app-component-chartjs {}\n
`

	$: rawCode && parseJson()
	$: rawCss && parseCss()
	let jsonError = ''
	let jsonErrorHeight: number
	let cssError = ''
	let cssErrorHeight: number

	let cssEditor: SimpleEditor | undefined = undefined

	function parseJson() {
		try {
			$app.css = JSON.parse(rawCode ?? '')
			jsonError = ''
		} catch (e) {
			jsonError = e.message
		}
	}

	function parseCss() {
		cssError = ''
		const errors: string[] = []

		const { css, removedClassNames } = sanitizeCss(rawCss, authorizedClassnames)
		errors.push(...validate(rawCss).map((e) => e.message))

		const newCss = css.replaceAll('}', '}\n')

		if (removedClassNames.length > 0) {
			errors.push('Some css properties were removed because they are not allowed')
			errors.push(...removedClassNames.map((r) => `  - ${r}`))
		}

		if (errors.length > 0) {
			cssError = errors.join('\n')
		}

		$app.cssString = newCss
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
</script>

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
	<Tab value="css" size="xs" class="w-1/2">
		<div class="m-1 center-center">
			<CurlyBraces size={16} />
			<span class="pl-1">CSS</span>
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
			<div style="height: calc(100% - {cssErrorHeight || 0}px);">
				<SimpleEditor
					class="h-full"
					lang="css"
					bind:code={rawCss}
					fixedOverflowWidgets={false}
					small
					automaticLayout
					bind:editor={cssEditor}
				/>
			</div>
		</TabContent>
	</div>
</Tabs>
