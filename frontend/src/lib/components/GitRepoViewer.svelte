<script lang="ts">
	import { HelpersService, SettingService } from '$lib/gen'
	import { onMount } from 'svelte'
	import { Alert } from './common'
	import S3FilePickerInner from './S3FilePickerInner.svelte'

	let s3FilePicker: S3FilePickerInner | undefined = $state()

	interface Props {
		gitRepoResourcePath: string
		commitHash?: string
	}

	let {
		gitRepoResourcePath,
		commitHash,
	}: Props = $props()

	onMount(async () => {
		//request to get the hash
		if (!commitHash) {

		}
	})
</script>

{#if commitHash}
	<S3FilePickerInner
		bind:this={s3FilePicker}
		readOnlyMode
		hideS3SpecificDetails
		rootPath={`gitrepos/wwwww/${gitRepoResourcePath}/${commitHash}/`}
		listStoredFilesRequest={HelpersService.listGitRepoFiles}
		loadFilePreviewRequest={HelpersService.loadGitRepoFilePreview}
		testConnectionRequest={(async (_d) => {
			const bucketConfig: any = await SettingService.getGlobal({ key: 'object_store_cache_config' })
			return SettingService.testObjectStorageConfig({
				requestBody: bucketConfig
			})
		}) as any}
		loadFileMetadataRequest={HelpersService.loadGitRepoFileMetadata}
	>
		{#snippet replaceUnauthorizedWarning()}
			<div class="mb-2">
				<Alert type="error" title="Cannot view git repo">
					<p>
						The git repo resource you are trying to access either doesn't exist or you don't have
						access to it. Make sure the resource path is correct and that you have visibility over the
						resource.
					</p>
				</Alert>
			</div>
		{/snippet}
	</S3FilePickerInner>
{/if}
