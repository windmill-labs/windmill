<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { AlertTriangle, Info } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import type { AppViewerContext } from '../../types'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import CssHelperPanel from './CssHelperPanel.svelte'
	import { premiumStore } from '$lib/stores'
	import Tooltip from '$lib/components/Tooltip.svelte'

	const { app, cssEditorOpen } = getContext<AppViewerContext>('AppViewerContext')

	let cssEditor: SimpleEditor | undefined = undefined

	let alertHeight: number | undefined = undefined

	onMount(() => {
		$cssEditorOpen = true
	})
</script>

<Splitpanes horizontal>
	<Pane size={60}>
		{#if !$premiumStore.premium}
			<div bind:clientHeight={alertHeight} class="p-2 flex flex-row gap-2">
				<div class="flex flex-row gap-2 items-center text-yellow-500 text-xs">
					<AlertTriangle size={16} />
					EE only
					<Tooltip light>
						App CSS editor is an exclusive feature of the Enterprise Edition. You can experiment
						with this feature in the editor, but please note that the changes will not be visible in
						the preview.
					</Tooltip>
				</div>
				<div class="flex flex-row gap-2 items-center text-blue-500 text-xs">
					<Info size={16} />
					Component styling is still available in the Community Edition
					<Tooltip light>
						App CSS editor is an exclusive feature of the Enterprise Edition. You can experiment
						with this feature in the editor, but please note that the changes will not be visible in
						the preview.
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
