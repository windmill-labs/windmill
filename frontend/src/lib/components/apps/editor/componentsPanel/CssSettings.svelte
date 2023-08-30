<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { LayoutDashboardIcon } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { emptyString } from '$lib/utils'
	import type { AppViewerContext } from '../../types'
	import { ccomponents, components } from '../component'
	import { slide } from 'svelte/transition'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import CssHelperPanel from './CssHelperPanel.svelte'

	const STATIC_ELEMENTS = ['app'] as const

	type CustomCSSType = (typeof STATIC_ELEMENTS)[number] | keyof typeof components

	interface CustomCSSEntry {
		type: CustomCSSType
		name: string
		icon: any
		ids: { id: string; forceStyle: boolean; forceClass: boolean }[]
	}

	const { app, cssEditorOpen } = getContext<AppViewerContext>('AppViewerContext')

	let cssError = ''
	let cssErrorHeight: number

	let cssEditor: SimpleEditor | undefined = undefined

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

	onMount(() => {
		$cssEditorOpen = true
	})
</script>

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
