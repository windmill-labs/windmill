<script lang="ts">
	import { superadmin } from '$lib/stores'
	import { JobService } from '$lib/gen'
	import { pollJobResult } from '$lib/components/jobs/utils'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { sendUserToast } from '$lib/toast'
	import Button from './common/button/Button.svelte'

	interface Props {
		onSynced?: () => void
	}

	let { onSynced = undefined }: Props = $props()

	let hubRtSync = usePromise(
		async () => {
			let jobUuid = await JobService.runScriptByPath({
				workspace: 'admins',
				path: 'u/admin/hub_sync',
				requestBody: {}
			})
			await pollJobResult(jobUuid, 'admins')
			sendUserToast('Hub resource types sync completed')
			onSynced?.()
		},
		{ loadInit: false }
	)
</script>

{#if $superadmin}
	<Button
		loading={hubRtSync.status === 'loading'}
		onClick={() => hubRtSync.refresh()}
		size="xs"
		variant="default"
	>
		Sync resource types with Hub
	</Button>
	{#if hubRtSync.status === 'error'}
		<span class="text-red-400 dark:text-red-500 text-xs">
			Error syncing resource types: {JSON.stringify(hubRtSync.error)}
		</span>
	{/if}
{/if}
