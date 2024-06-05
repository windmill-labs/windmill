<script lang="ts">
	import { Pipette } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import JsonEditor from '../../apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'
	import { Button } from '../../common'
	import SimpleEditor from '../../SimpleEditor.svelte'
	import Toggle from '../../Toggle.svelte'
	import S3FilePicker from '../../S3FilePicker.svelte'
	import FileUpload from '../../common/fileUpload/FileUpload.svelte'

	export let value: any
	export let editor: SimpleEditor | undefined = undefined
	export let defaultValue: any = undefined

	const dispatch = createEventDispatcher()

	let s3FilePicker: S3FilePicker
	let s3FileUploadRawMode: false
	let rawValue: string | undefined = undefined
</script>

<S3FilePicker
	bind:this={s3FilePicker}
	bind:selectedFileKey={value}
	on:close={() => {
		rawValue = JSON.stringify(value, null, 2)
		editor?.setCode(rawValue)
	}}
	readOnlyMode={false}
/>

<div class="flex flex-col w-full gap-1">
	<Toggle
		class="flex justify-end"
		bind:checked={s3FileUploadRawMode}
		size="xs"
		options={{ left: 'Raw S3 object input' }}
	/>
	{#if s3FileUploadRawMode}
		<JsonEditor
			bind:editor
			on:focus={(e) => {
				dispatch('focus')
			}}
			on:blur={(e) => {
				dispatch('blur')
			}}
			code={JSON.stringify(value ?? defaultValue ?? { s3: '' }, null, 2)}
			bind:value
		/>
		<Button
			variant="border"
			color="light"
			size="xs"
			btnClasses="mt-1"
			on:click={() => {
				s3FilePicker?.open?.(value)
			}}
			startIcon={{ icon: Pipette }}
		>
			Choose an object from the catalog
		</Button>
	{:else}
		<FileUpload
			allowMultiple={false}
			randomFileKey={true}
			on:addition={(evt) => {
				value = {
					s3: evt.detail?.path ?? ''
				}
			}}
			on:deletion={(evt) => {
				value = {
					s3: ''
				}
			}}
			defaultValue={defaultValue?.s3}
		/>
	{/if}
</div>
