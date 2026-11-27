<script lang="ts">
	import { Download } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { downloadViaClient, shouldDownloadViaClient } from '$lib/utils/downloadFile'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		s3object: any
		workspaceId?: string | undefined
		appPath?: string | undefined
	}

	let { s3object, workspaceId = undefined, appPath = undefined }: Props = $props()

	let workspace = $derived(workspaceId ?? $operatingWorkspace)
	let filename = $derived(s3object?.s3?.split?.('/')?.pop() ?? 'unnamed_download.file')

	let apiPath = $derived(
		`${
			appPath
				? `/w/${workspace}/apps_u/download_s3_file/${appPath}`
				: `/w/${workspace}/job_helpers/download_s3_file`
		}?${appPath ? 's3' : 'file_key'}=${encodeURIComponent(s3object?.s3 ?? '')}${
			s3object?.storage ? `&storage=${s3object.storage}` : ''
		}${appPath && s3object?.presigned ? `&${s3object?.presigned}` : ''}`
	)
	let href = $derived(`/api${apiPath}`)
</script>

{#if s3object && s3object?.s3}
	{#if shouldDownloadViaClient()}
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: Download }}
			onClick={() => downloadViaClient(apiPath, filename)}
		>
			Download
		</Button>
	{:else}
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: Download }}
			{href}
			download={filename}
		>
			Download
		</Button>
	{/if}
{/if}
