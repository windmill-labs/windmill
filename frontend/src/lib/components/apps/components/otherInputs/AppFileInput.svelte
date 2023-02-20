<script lang="ts">
	import { getContext } from 'svelte'
	import { FileInput } from '../../../common'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

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
</script>

<InputValue {id} input={configuration.acceptedFileTypes} bind:value={acceptedFileTypes} />
<InputValue {id} input={configuration.allowMultiple} bind:value={allowMultiple} />
<InputValue {id} input={configuration.text} bind:value={text} />

<div class="w-full h-full p-1">
	<FileInput
		accept={acceptedFileTypes?.length ? acceptedFileTypes?.join(', ') : undefined}
		multiple={allowMultiple}
		convertToBase64
		on:change={({detail}) => {handleChange(detail)}}
		class="w-full h-full"
	>
		{text}
	</FileInput>
</div>
