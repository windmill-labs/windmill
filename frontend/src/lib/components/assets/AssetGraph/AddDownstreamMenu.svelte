<!--
@component
The menu adding a pipeline script downstream of an asset: opened from the asset's
"+", or where that "+" was dropped on the blank canvas.
-->
<script lang="ts" module>
	import type { ScriptLang } from '$lib/gen'
	import type { AssetKind } from '$lib/components/assets/lib'
	import type { PipelineInsertOptions } from './PipelineInsertMenu.svelte'
	import type { PipelineOutputKind } from './pipelineTemplates'

	export type AddScriptForAsset = (
		asset: { kind: AssetKind; path: string },
		language: ScriptLang,
		scriptPath: string,
		outputKind: PipelineOutputKind,
		aiPrompt?: string,
		options?: PipelineInsertOptions
	) => void
</script>

<script lang="ts">
	import { Code2 } from 'lucide-svelte'
	import type { Snippet } from 'svelte'
	import PipelineInsertMenu, { type PipelineInsertPick } from './PipelineInsertMenu.svelte'
	import { PIPELINE_LANGUAGES } from './pipelineLanguages'

	let {
		asset,
		onAddScript,
		pathPrefix = '',
		defaultPathSuffix = '',
		openSignal,
		trigger
	}: {
		asset: { kind: AssetKind; path: string }
		onAddScript: AddScriptForAsset
		pathPrefix?: string
		defaultPathSuffix?: string
		openSignal?: number
		trigger: Snippet<[{ open: boolean }]>
	} = $props()

	function handlePick(pick: PipelineInsertPick) {
		if (pick.kindId === 'pipeline_script' && pick.language && pick.path) {
			onAddScript(
				asset,
				pick.language as ScriptLang,
				pick.path,
				(pick.outputKind ?? 'none') as PipelineOutputKind,
				pick.aiPrompt,
				{ outputAsset: pick.outputAsset }
			)
		}
	}
</script>

<PipelineInsertMenu
	kinds={[
		{
			id: 'pipeline_script',
			label: 'Add downstream pipeline script',
			description: 'Triggered when this asset changes',
			icon: Code2
		}
	]}
	languages={PIPELINE_LANGUAGES as any}
	{pathPrefix}
	{defaultPathSuffix}
	{openSignal}
	onPick={handlePick}
	{trigger}
/>
