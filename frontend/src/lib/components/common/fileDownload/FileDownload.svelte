<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { Download } from 'lucide-svelte'
	import { base } from '$lib/base'

	interface Props {
		s3object: any
		workspaceId?: string | undefined
		appPath?: string | undefined
	}

	let { s3object, workspaceId = undefined, appPath = undefined }: Props = $props()
</script>

{#if s3object && s3object?.s3}
	<a
		class="relative center-center flex w-full text-center font-normal text-primary text-sm
border border-dashed border-gray-400 hover:border-blue-500
focus-within:border-blue-500 hover:bg-blue-50 dark:hover:bg-frost-900 focus-within:bg-blue-50
duration-200 rounded-lg p-1 gap-2"
		href={`${base}/api/w/${workspaceId ?? $workspaceStore}${
			appPath ? `/apps_u/download_s3_file/${appPath}` : '/job_helpers/download_s3_file'
		}?${appPath ? 's3' : 'file_key'}=${encodeURIComponent(s3object?.s3 ?? '')}${
			s3object?.storage ? `&storage=${s3object.storage}` : ''
		}${appPath && s3object?.presigned ? `&${s3object?.presigned}` : ''}`}
		download={s3object?.s3?.split?.('/')?.pop() ?? 'unnamed_download.file'}
	>
		<Download />
		<span>
			{s3object?.storage ? `s3://${s3object.storage}/${s3object.s3}` : `s3:///${s3object.s3}`}
		</span>
	</a>
{/if}
