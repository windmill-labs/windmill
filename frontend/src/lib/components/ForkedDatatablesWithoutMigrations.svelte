<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import Alert from './common/alert/Alert.svelte'
	import { base } from '$lib/base'
	import { resource } from 'runed'

	interface Props {
		workspaceId: string
	}
	let { workspaceId }: Props = $props()

	const datatables = resource(
		() => workspaceId,
		async (workspace) => {
			try {
				const settings = await WorkspaceService.getPublicSettings({ workspace })
				return Object.entries(settings.datatable?.datatables ?? {})
					.filter(([_, dt]) => dt.forked_from != null && dt.migrations_enabled !== true)
					.map(([name]) => name)
					.sort()
			} catch (e) {
				console.error('Failed to load data table settings', e)
				return []
			}
		}
	)

	const names = $derived(datatables.current ?? [])
</script>

{#if names.length > 0}
	<Alert
		type="warning"
		size="xs"
		title="Schema changes are not deployed for {names.join(', ')}"
		class="bg-surface-tertiary"
	>
		<span>
			A data table cloned into its own database only propagates schema changes through SQL
			migrations, which are not enabled here — schema changes made in this workspace stay in it.
			Enable migrations in
			<a href="{base}/workspace_settings?tab=windmill_data_tables">data table settings</a>
			to version schema changes and deploy them like any other item.
		</span>
	</Alert>
{/if}
