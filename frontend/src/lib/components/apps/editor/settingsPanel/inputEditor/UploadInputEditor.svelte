<script lang="ts">
	import { FileInput } from '../../../../common'
	import type { UploadAppInput } from '../../../inputType'

	export let componentInput: UploadAppInput | undefined
</script>

<FileInput
	accept={componentInput?.fileUpload?.accept}
	multiple={componentInput?.fileUpload?.multiple}
	convertTo={componentInput?.fileUpload?.convertTo}
	iconSize={24}
	class="text-sm py-4"
	on:change={({ detail }) => {
		if (componentInput) {
			componentInput.value = componentInput?.fileUpload?.multiple ? detail : detail?.[0]
		}
	}}
>
	<svelte:fragment slot="selected-title">
		<!-- Removing the title when there is a selected file -->
		<span />
	</svelte:fragment>
</FileInput>
