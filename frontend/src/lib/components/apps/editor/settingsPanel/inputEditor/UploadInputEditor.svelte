<script lang="ts">
	import { FileInput } from '../../../../common'
	import type { UploadAppInput } from '../../../inputType'

	export let componentInput: UploadAppInput | undefined
	export let fileUpload: UploadAppInput['fileUpload'] | undefined
</script>

<FileInput
	accept={fileUpload?.accept}
	multiple={fileUpload?.multiple}
	convertTo={fileUpload?.convertTo}
	iconSize={24}
	class="text-sm py-4"
	on:change={({ detail }) => {
		if (componentInput) {
			componentInput.value = fileUpload?.multiple ? detail : detail?.[0]
		}
	}}
>
	<svelte:fragment slot="selected-title">
		<!-- Removing the title when there is a selected file -->
		<span />
	</svelte:fragment>
</FileInput>
