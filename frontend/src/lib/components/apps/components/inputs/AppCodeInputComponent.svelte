<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { type AppViewerContext, type RichConfigurations } from '../../types'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	let { id, configuration, render } = $props<{
		id: string
		configuration: RichConfigurations
		render: boolean
	}>()

	const { componentControl, worldStore, selectedComponent, connectingInput, mode } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = $state(initConfig(
		components['codeinputcomponent'].initialData.configuration,
		configuration
	))

	let code = $state<string | undefined>(undefined)
	let placeholder = $state<string | undefined>(undefined)
	let defaultValue = $state<string | undefined>(undefined)
	let editorInstance = $state<any>(null)
	let lastDefaultValue = $state<string | undefined>(undefined)

	let lang = $derived(resolvedConfig?.lang ?? 'javascript')
	let outputs = $state(initOutput($worldStore, id, {
		result: ''
	}))

	$effect(() => {
		if (defaultValue !== lastDefaultValue) {
			code = defaultValue
			editorInstance?.setCode(defaultValue)
			lastDefaultValue = defaultValue
		}
		if (code !== undefined) {
			outputs?.result.set(code)
		}
	})

	$componentControl[id] = {
		...$componentControl[id],
		setValue(value: string) {
			code = value
			editorInstance?.setCode(value)
		}
	}
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
		onpointerdown={(e) => {
			e.stopPropagation()
			if (!$connectingInput.opened) {
				$selectedComponent = [id]
			}
		}}
	>
		{#await import('$lib/components/SimpleEditor.svelte')}
			<div class="flex items-center justify-center h-full">
				<div class="text-gray-500 dark:text-gray-400">Loading editor...</div>
			</div>
		{:then Module}
			<Module.default
				bind:this={editorInstance}
				bind:code
				{lang}
				class="h-full"
				automaticLayout={true}
				autoHeight={false}
				{placeholder}
				disableSuggestions={resolvedConfig?.disableSuggestions ?? false}
				disableLinting={resolvedConfig?.disableLinting ?? false}
				hideLineNumbers={resolvedConfig?.hideLineNumbers ?? false}
				fixedOverflowWidgets={$mode == 'dnd' ? false : true}
			/>
		{/await}
	</div>
{/if}

<style lang="postcss">
	:global(.suggest-widget) {
		position: fixed !important;
	}
</style>
