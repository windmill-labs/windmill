<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { FileInput } from '../../../common'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['result']
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let acceptedFileTypes: string[] | undefined = undefined
	let allowMultiple: boolean | undefined = undefined
	let text: string | undefined = undefined

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string[] | undefined>
	}

	// Receives Base64 encoded strings from the input component
	async function handleChange(files: string[] | undefined) {
		outputs?.result.set(files)
	}

	$: css = concatCustomCss($app.css?.fileinputcomponent, customCss)
</script>

<InputValue {id} input={configuration.acceptedFileTypes} bind:value={acceptedFileTypes} />
<InputValue {id} input={configuration.allowMultiple} bind:value={allowMultiple} />
<InputValue {id} input={configuration.text} bind:value={text} />

{#if render}
	<div class="w-full h-full p-1">
		<FileInput
			accept={acceptedFileTypes?.length ? acceptedFileTypes?.join(', ') : undefined}
			multiple={allowMultiple}
			convertTo="base64"
			on:change={({ detail }) => {
				handleChange(detail)
			}}
			class={twMerge('w-full h-full', css?.container?.class)}
			style={css?.container?.style}
		>
			{text}
		</FileInput>
	</div>
{/if}
