<script lang="ts">
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Loader2, Pipette } from 'lucide-svelte'
	import FileUpload from './FileUpload.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Button from '../button/Button.svelte'
	import { userStore } from '$lib/stores'

	let {
		multiple,
		value = $bindable(),
		defaultValue,
		setNewValueFromCode,
		onFocus,
		onBlur,
		computeS3ForceViewerPolicies,
		workspace,
		editor = $bindable(),
		appPath
	}: {
		multiple: boolean
		value: any
		defaultValue: any
		setNewValueFromCode: (v: any) => void
		onFocus: () => void
		onBlur: () => void
		computeS3ForceViewerPolicies:
			| (() =>
					| {
							allowed_resources: string[]
							allow_user_resources: boolean
							allow_workspace_resource: boolean
							file_key_regex: string
					  }
					| undefined)
			| undefined
		workspace: string | undefined
		editor: SimpleEditor | undefined
		appPath: string | undefined
	} = $props()

	let s3FileUploadRawMode: boolean = $state(false)

	let s3FilePicker: S3FilePicker | undefined = $state(undefined)
	let fileUpload: FileUpload | undefined = $state(undefined)
</script>

{#if $userStore}
	<S3FilePicker
		bind:this={s3FilePicker}
		on:selectAndClose={(ev) => {
			if (multiple) {
				if (Array.isArray(value)) {
					value.push(ev.detail)
				} else {
					value = [ev.detail]
				}
				fileUpload?.addUpload(ev.detail)
			} else {
				value = ev.detail
				fileUpload?.setUpload(value)
			}
			editor?.setCode(JSON.stringify(value))
		}}
		readOnlyMode={false}
	/>
{/if}

<div class="flex flex-col w-full gap-1">
	<Toggle
		class="flex justify-end"
		bind:checked={s3FileUploadRawMode}
		size="xs"
		options={{ left: `Raw S3 object${multiple ? 's' : ''} input` }}
	/>
	{#if s3FileUploadRawMode}
		{#await import('$lib/components/JsonEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				bind:editor
				on:focus={(e) => {
					onFocus()
				}}
				on:blur={(e) => {
					onBlur()
				}}
				code={JSON.stringify(value ?? defaultValue ?? (multiple ? [] : { s3: '' }), null, 2)}
				on:changeValue={(e) => {
					setNewValueFromCode(e.detail)
				}}
			/>
		{/await}
	{:else}
		<FileUpload
			bind:this={fileUpload}
			{appPath}
			computeForceViewerPolicies={computeS3ForceViewerPolicies}
			{workspace}
			allowMultiple={multiple}
			randomFileKey={true}
			on:addition={(evt) => {
				const s3Object = {
					s3: evt.detail?.path ?? '',
					filename: evt.detail?.filename ?? ''
				}
				if (multiple) {
					if (Array.isArray(value)) {
						value.push(s3Object)
					} else {
						value = [s3Object]
					}
				} else {
					value = s3Object
				}
			}}
			on:deletion={(evt) => {
				if (multiple) {
					if (Array.isArray(value)) {
						value = value.filter((v) => v.s3 !== evt.detail?.path)
					}
				} else {
					value = {
						s3: ''
					}
				}
			}}
			defaultValue={defaultValue?.s3}
			initialValue={value}
		/>
	{/if}
	{#if $userStore}
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
			{multiple ? 'Add' : 'Choose'} an object from the catalog
		</Button>
	{/if}
</div>
