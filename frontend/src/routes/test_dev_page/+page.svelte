<script lang="ts">
	import { globalUiConfig } from '$lib/stores'
	import type { ScriptBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'

	let showWhiteLabelUI: 'no' | 'old' | 'new' = 'new'
	const showWhiteLabelOptions = [
		{ value: 'no', label: 'No whitelabel' },
		{ value: 'old', label: 'Old whitelabel' },
		{ value: 'new', label: 'New whitelabel' }
	]
	const noWhiteLabelUIConfig: undefined = undefined
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
			editablePath: false,
			settings: true,
			extraDeployOptions: false,
			editableSummary: true,
			diff: true
		},
		editorBar: {
			contextVar: false,
			variable: false,
			resource: false,
			type: false,
			assistants: true,
			reset: false,
			multiplayer: false,
			autoformatting: false,
			vimMode: true,
			aiGen: false,
			aiCompletion: false,
			library: false,
			useVsCode: false
		},
		settingsPanel: {
			metadata: {
				languages: ['python3'],
				scriptKind: { disabled: true },
				editableSchemaForm: { jsonOnly: true, variablePicker: { disabled: true } },
				mute: { disabled: true }
			},
			runtime: {
				disabled: true
			},

			triggers: {
				disabled: true
			}
		},
		previewPanel: {
			triggerButton: { disabled: true },
			displayResult: { aiFix: { disabled: true } },
			triggerCaptures: { disabled: true },
			history: { disabled: true },
			variablePicker: { disabled: true }
		},
		tooltips: { disabled: true }
	}

	$: customUi =
		showWhiteLabelUI === 'old'
			? oldWhiteLabelUIConfig
			: showWhiteLabelUI === 'new'
			? newWhiteLabelUIConfig
			: noWhiteLabelUIConfig

	$: if (customUi?.tooltips?.disabled) {
		globalUiConfig.set({
			...$globalUiConfig,
			tooltips: {
				disabled: true
			}
		})
	} else {
		globalUiConfig.set({ ...$globalUiConfig, tooltips: undefined })
	}
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
	{customUi}
/>
