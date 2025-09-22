<script lang="ts">
	import { FileInput } from '../../../../common'
	import type {
		UploadAppInput,
		UploadS3AppInput,
		FileUploadData,
		StaticInput
	} from '../../../inputType'
	import type { ReadFileAs } from '../../../../common/fileInput/model'
	import FileUpload from '$lib/components/common/fileUpload/FileUpload.svelte'
	import { writable, type Writable } from 'svelte/store'

	interface Props {
		componentInput: UploadAppInput | UploadS3AppInput | StaticInput<any> | undefined
		fileUpload: UploadAppInput['fileUpload'] | UploadS3AppInput['fileUploadS3'] | undefined
		s3?: boolean | undefined
		prefix?: string | undefined
		workspace?: string | undefined
		s3FileUploadRawMode?: boolean
	}

	let {
		componentInput = $bindable(),
		fileUpload,
		s3 = false,
		prefix = undefined,
		workspace = undefined,
		s3FileUploadRawMode = $bindable(false)
	}: Props = $props()

	let fileUploads: Writable<FileUploadData[]> = writable([])

	function hasConvertTo(upload: any): upload is { convertTo: ReadFileAs } {
		return upload && 'convertTo' in upload
	}
</script>

{#if s3}
	<FileUpload
		acceptedFileTypes={[fileUpload?.accept ?? '*']}
		allowMultiple={fileUpload?.multiple}
		containerText={'Drag and drop a file'}
		customResourceType="s3"
		iconSize={24}
		customClass="text-sm py-4"
		{fileUploads}
		{workspace}
		pathTransformer={({ file }) => {
			const cleanPrefix = prefix ? `${prefix.replace(/^\/+|\/+$/g, '')}/` : ''
			return `${cleanPrefix}${file.name}`
		}}
		on:addition={({ detail }) => {
			// @ts-ignore
			componentInput = {
				...componentInput,
				type: 'uploadS3',
				value: { s3: detail.path }
			}
			s3FileUploadRawMode = true
		}}
	/>
{:else}
	<FileInput
		accept={fileUpload?.accept}
		multiple={fileUpload?.multiple}
		convertTo={hasConvertTo(fileUpload) ? fileUpload.convertTo : undefined}
		iconSize={24}
		class="text-sm py-4"
		on:change={({ detail }) => {
			componentInput = {
				...componentInput,
				type: 'static',
				value: fileUpload?.multiple ? detail : detail?.[0]
			}
		}}
	>
		<!-- @migration-task: migrate this slot by hand, `selected-title` is an invalid identifier -->
		<svelte:fragment slot="selected-title">
			<!-- Removing the title when there is a selected file -->
			<span></span>
		</svelte:fragment>
	</FileInput>
{/if}
