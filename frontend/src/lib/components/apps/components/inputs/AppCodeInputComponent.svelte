<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { type AppViewerContext, type RichConfigurations } from '../../types'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let render: boolean

	const { componentControl, worldStore, selectedComponent, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['codeinputcomponent'].initialData.configuration,
		configuration
	)

	let code: string | undefined = undefined
	let lang = resolvedConfig?.lang ?? 'javascript'
	let placeholder: string | undefined = undefined
	let defaultValue: string | undefined = undefined

	let outputs = initOutput($worldStore, id, {
		result: ''
	})

	$: if (resolvedConfig?.defaultValue !== undefined) {
		defaultValue = resolvedConfig.defaultValue
	}

	$: if (resolvedConfig?.lang) {
		lang = resolvedConfig.lang
	}

	$: handleDefault(defaultValue)

	function handleDefault(defaultValue: string | undefined) {
		if (defaultValue !== undefined) {
			code = defaultValue
			setOutput()
		}
	}

	function setOutput() {
		if (code !== undefined) {
			outputs?.result.set(code)
		}
	}

	$componentControl[id] = {
		...$componentControl[id],
		setValue(value: string) {
			code = value
			setOutput()
		}
	}

	$: code !== undefined && setOutput()
</script>

<InputValue key="placeholder" {id} input={configuration.placeholder} bind:value={placeholder} />
<InputValue key="value" {id} input={configuration.defaultValue} bind:value={defaultValue} />
<InitializeComponent {id} />

{#each Object.keys(components['codeinputcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#if render}
	<div
		class="h-full flex-col flex max-h-full overflow-scroll editor-wrapper rounded-md border border-gray-300 dark:border-gray-500 wm-code-editor"
		on:pointerdown|stopPropagation={() => {
			if (!$connectingInput.opened) {
				$selectedComponent = [id]
			}
		}}
	>
		<SimpleEditor
			bind:code
			{lang}
			automaticLayout={true}
			autoHeight={true}
			{placeholder}
			disableSuggestions={resolvedConfig?.disableSuggestions ?? false}
			disableLinting={resolvedConfig?.disableLinting ?? false}
			hideLineNumbers={resolvedConfig?.hideLineNumbers ?? false}
			fixedOverflowWidgets={false}
		/>
	</div>
{/if}

<style lang="postcss">
	:global(.monaco-editor) {
		min-height: 100%;
	}

	:global(.suggest-widget) {
		position: fixed !important;
	}
</style>
