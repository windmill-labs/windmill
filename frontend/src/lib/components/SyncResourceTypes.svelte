<script lang="ts">
	import { superadmin } from '$lib/stores'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { sendUserToast } from '$lib/toast'
	import Button from './common/button/Button.svelte'

	interface Props {
		onSynced?: () => void
		// When set, the endpoint returns an explicit not-found error if the hub does
		// not know this type (the sync itself still refreshes the whole list).
		resourceType?: string
	}

	let { onSynced = undefined, resourceType = undefined }: Props = $props()

	let hubRtSync = usePromise(
		async () => {
			const url = resourceType
				? `/api/settings/sync_cached_resource_types?name=${encodeURIComponent(resourceType)}`
				: '/api/settings/sync_cached_resource_types'
			const res = await fetch(url, { method: 'POST' })
			if (!res.ok) {
				const body = await res.text()
				throw new Error(body || res.statusText)
			}
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
			Error syncing resource types: {hubRtSync.error?.message ?? JSON.stringify(hubRtSync.error)}
		</span>
	{/if}
{/if}
