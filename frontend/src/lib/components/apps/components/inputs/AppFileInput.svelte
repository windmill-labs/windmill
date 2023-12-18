<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { FileInput } from '../../../common'
	import { initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'fileinputcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let acceptedFileTypes: string[] | undefined = undefined
	let allowMultiple: boolean | undefined = undefined
	let text: string | undefined = undefined
	let includeMimeType: boolean | undefined = undefined
	let submittedFileText: string | undefined = undefined

	let outputs = initOutput($worldStore, id, {
		result: [] as { name: string; data: string }[] | undefined
	})

	// Receives Base64 encoded strings from the input component
	async function handleChange(files: { name: string; data: string }[] | undefined) {
		if (includeMimeType === false) {
			files = files?.map((file) => {
				const [_, data] = file.data.split('base64,')
				return { name: file.name, data }
			})
		}
		outputs?.result.set(files)
	}

	let css = initCss($app.css?.fileinputcomponent, customCss)
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

<InputValue
	key="accepted"
	{id}
	input={configuration.acceptedFileTypes}
	bind:value={acceptedFileTypes}
/>
<InputValue key="multiple" {id} input={configuration.allowMultiple} bind:value={allowMultiple} />
<InputValue key="text" {id} input={configuration.text} bind:value={text} />
<InputValue key="mime" {id} input={configuration.includeMimeType} bind:value={includeMimeType} />
<InputValue
	key="submittedFileText"
	{id}
	input={configuration.submittedFileText}
	bind:value={submittedFileText}
/>

<InitializeComponent {id} />

{#if render}
	<div class="w-full h-full p-1">
		<FileInput
			accept={acceptedFileTypes?.length ? acceptedFileTypes?.join(', ') : undefined}
			multiple={allowMultiple}
			convertTo="base64"
			returnFileNames
			on:change={({ detail }) => {
				handleChange(detail)
			}}
			class={twMerge('w-full h-full', css?.container?.class, 'wm-file-input')}
			style={css?.container?.style}
			submittedText={submittedFileText}
		>
			{text}
		</FileInput>
	</div>
{/if}
