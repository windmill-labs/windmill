<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { FileInput } from '../../../common'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'

	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { RunnableComponent, RunnableWrapper } from '../helpers'
	import type { AppInput, RunnableByName } from '../../inputType'
	import { Preview } from '$lib/gen'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'fileinputcomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined

	let input: AppInput | undefined = undefined
	let runnableComponent: RunnableComponent
	let loading = false

	let resolvedConfig = initConfig(
		components['s3fileinputcomponent'].initialData.configuration,
		configuration
	)

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, id, {
		result: [] as { name: string; data: string }[] | undefined,
		loading: false,
		jobId: undefined
	})

	// Receives Base64 encoded strings from the input component
	async function handleChange(files: { name: string; data: string }[] | undefined) {
		if (resolvedConfig.includeMimeType === false) {
			files = files?.map((file) => {
				const [_, data] = file.data.split('base64,')
				return { name: file.name, data }
			})
		}

		const fileUploadRunnable: RunnableByName = {
			name: 'AppDbExplorer',
			type: 'runnableByName',
			inlineScript: {
				content: ``,
				language: Preview.language.DENO,
				schema: {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {},
					required: ['database'],
					type: 'object'
				}
			}
		}

		input = {
			runnable: fileUploadRunnable,
			fields: {
				database: {
					type: 'static',
					value: resolvedConfig.resource,
					fieldType: 'object',
					format: 'resource-s3'
				}
			},
			type: 'runnable',
			fieldType: 'object'
		}

		outputs?.result.set(files)
	}

	let css = initCss($app.css?.fileinputcomponent, customCss)

	let done: boolean = false
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

{#each Object.keys(components['buttoncomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#if render}
	<div class="w-full h-full p-1">
		{#if done}
			<div class="flex items-center justify-center h-full flex-col gap-2">
				{resolvedConfig.submittedFileText}
				<Button
					size="xs"
					on:click={() => {
						done = false
						outputs?.result.set(undefined)
					}}
				>
					Restart
				</Button>
			</div>
		{:else}
			<FileInput
				accept={resolvedConfig.acceptedFileTypes?.length
					? resolvedConfig.acceptedFileTypes?.join(', ')
					: undefined}
				multiple={resolvedConfig.allowMultiple}
				convertTo="base64"
				returnFileNames
				on:change={({ detail }) => {
					handleChange(detail)
					done = true
				}}
				class={twMerge('w-full h-full', css?.container?.class, 'wm-file-input')}
				style={css?.container?.style}
			>
				{resolvedConfig.text}
			</FileInput>
		{/if}
	</div>
{/if}

<RunnableWrapper
	noInitialize
	bind:runnableComponent
	bind:loading
	componentInput={input}
	autoRefresh={false}
	render={false}
	id={`${id}_update`}
	{outputs}
/>
