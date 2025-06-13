<script lang="ts">
	import { Loader2, Pipette } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	// @ts-ignore
	import type SimpleEditor from './SimpleEditor.svelte'

	import { Button } from './common'

	import Toggle from './Toggle.svelte'

	import S3FilePicker from './S3FilePicker.svelte'
	import FileUpload from './common/fileUpload/FileUpload.svelte'

	export let value: any
	export let editor: SimpleEditor | undefined = undefined

	const dispatch = createEventDispatcher()

	let s3FilePicker: S3FilePicker
	let s3FileUploadRawMode: false
	let el: HTMLTextAreaElement | undefined = undefined
	let rawValue: string | undefined = undefined

	function evalValueToRaw() {
		rawValue = JSON.stringify(value, null, 2)
	}

	evalValueToRaw()

	export function focus() {
		el?.focus()
		if (el) {
			el.style.height = '5px'
			el.style.height = el.scrollHeight + 50 + 'px'
		}
	}
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
		{#await import('$lib/components/JsonEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				bind:editor
				on:focus={(e) => {
					dispatch('focus')
				}}
				code={JSON.stringify(value ?? { s3: '' }, null, 2)}
				bind:value
			/>
		{/await}
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
			defaultValue={value?.s3}
		/>
	{/if}
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
</div>
