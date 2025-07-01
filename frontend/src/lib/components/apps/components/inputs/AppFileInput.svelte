<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { FileInput } from '../../../common'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'fileinputcomponent'> | undefined
		render: boolean
		onFileChange?: string[] | undefined
		extraKey?: string | undefined
	}

	let {
		id,
		configuration,
		customCss = undefined,
		render,
		onFileChange = undefined,
		extraKey = undefined
	}: Props = $props()

	const { app, worldStore, componentControl, mode, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, id, {
		result: [] as { name: string; data: string }[] | undefined
	})

	let resolvedConfig = $state(
		initConfig(components['fileinputcomponent'].initialData.configuration, configuration)
	)

	// Receives Base64 encoded strings from the input component
	async function handleChange(files: { name: string; data: string }[] | undefined) {
		if (resolvedConfig?.includeMimeType === false) {
			files = files?.map((file) => {
				const [_, data] = file.data.split('base64,')
				return { name: file.name, data }
			})
		}
		outputs?.result.set(files)
		onFileChange?.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb?.()))
	}

	let css = $state(initCss($app.css?.fileinputcomponent, customCss))

	let fileInput: FileInput | undefined = $state(undefined)

	$componentControl[id] = {
		clearFiles: () => {
			fileInput?.clearFiles()
		}
	}

	let files: File[] | undefined = $state(undefined)

	function preFillFiles() {
		const data = outputs?.result?.peak()

		if (data && Array.isArray(data) && data.length > 0) {
			files = data.map((file: { name: any }) => new File([], file?.name))
		}
	}

	$effect.pre(() => {
		outputs.result && files === undefined && $mode === 'dnd' && untrack(() => preFillFiles())
	})
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.fileinputcomponent}
	/>
{/each}

{#each Object.keys(components['fileinputcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />

{#if render}
	<div class="w-full h-full">
		<FileInput
			bind:this={fileInput}
			accept={resolvedConfig?.acceptedFileTypes?.length
				? resolvedConfig?.acceptedFileTypes?.join(', ')
				: undefined}
			multiple={resolvedConfig?.allowMultiple}
			convertTo="base64"
			returnFileNames
			on:change={({ detail }) => {
				handleChange(detail)
			}}
			class={twMerge('w-full h-full', css?.container?.class, 'wm-file-input')}
			style={css?.container?.style}
			submittedText={resolvedConfig?.submittedFileText}
			disabled={resolvedConfig.disabled}
			bind:files
		>
			{resolvedConfig?.text}
		</FileInput>
	</div>
{/if}
