<script lang="ts">
	import type { ScriptBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'

	let showWhiteLabelUI: 'no' | 'old' | 'new' = 'new'
	const showWhiteLabelOptions = [
		{ value: 'no', label: 'No whitelabel' },
		{ value: 'old', label: 'Old whitelabel' },
		{ value: 'new', label: 'New whitelabel' }
	]

	const noWhiteLabelUIConfig: ScriptBuilderWhitelabelCustomUi = {}
	const oldWhiteLabelUIConfig: ScriptBuilderWhitelabelCustomUi = {
		topBar: {
			path: false,
			settings: false,
			extraDeployOptions: false,
			editableSummary: true,
			diff: false
		},
		editorBar: {
			contextVar: false,
			variable: false,
			type: false,
			assistants: false,
			multiplayer: false,
			autoformatting: false,
			vimMode: true,
			aiGen: false,
			aiCompletion: false,
			library: false,
			useVsCode: false
		}
	}
	const newWhiteLabelUIConfig: ScriptBuilderWhitelabelCustomUi = {
		topBar: {
			path: true,
			settings: false,
			extraDeployOptions: false,
			editableSummary: true,
			diff: true
		},
		editorBar: {
			contextVar: false,
			variable: false,
			resource: false,
			type: false,
			assistants: false,
			multiplayer: false,
			autoformatting: false,
			vimMode: true,
			aiGen: false,
			aiCompletion: false,
			library: false,
			useVsCode: false
		}
	}

	$: customUI =
		showWhiteLabelUI === 'old'
			? oldWhiteLabelUIConfig
			: showWhiteLabelUI === 'new'
			? newWhiteLabelUIConfig
			: noWhiteLabelUIConfig
</script>

<select bind:value={showWhiteLabelUI} placeholder="Select UI Type">
	{#each showWhiteLabelOptions as option}
		<option value={option.value}>{option.label}</option>
	{/each}
</select>

<ScriptBuilder
	script={{
		summary: 'foo',
		path: 'u/admin/foo',
		description: 'foo',
		language: 'python3',
		content: 'def main():\n\tprint("Hello, World!")'
	}}
	neverShowMeta={true}
	customUi={customUI}
/>
