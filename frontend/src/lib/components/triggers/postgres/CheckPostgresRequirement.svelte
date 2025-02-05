<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { PostgresTriggerService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { emptyString } from '$lib/utils'

	let config: { isLogical: boolean; show: boolean } = { isLogical: false, show: false }
	let loadingConfiguration = false

	const checkDatabaseConfiguration = async () => {
		if (emptyString(postgres_resource_path)) {
			sendUserToast('You must first pick a database resource', true)
			return
		}
		try {
			loadingConfiguration = true
			config.isLogical = await PostgresTriggerService.isValidPostgresConfiguration({
				workspace: $workspaceStore!,
				path: postgres_resource_path
			})
			config.show = true
		} catch (error) {
			sendUserToast(error.body, true)
		}
		loadingConfiguration = false
	}

	export let can_write: boolean
	export let postgres_resource_path: string
	$: if (postgres_resource_path === undefined) {
		config.show = false
	}
</script>

{#if postgres_resource_path}
	<div class="flex flex-col justify-end mt-1 gap-2">
		<Button
			disabled={!can_write}
			loading={loadingConfiguration}
			on:click={checkDatabaseConfiguration}
			size="xs"
			color="light"
			spacingSize="sm"
			variant="border"
			>Check database configuration
			<Tooltip
				documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#requirements"
			>
				<p class="text-sm">
					Verifies whether the database is configured with the required <strong>settings</strong>.
				</p>
			</Tooltip>
		</Button>
		{#if config.show}
			<Alert title="Postgres configuration" type={config.isLogical === true ? 'success' : 'error'}>
				{#if config.isLogical}
					Database is in logical mode. Triggers can be used.
				{:else}
					Database is NOT in logical mode. Triggers cannot be used. Refer to the PostgreSQL
					documentation for configuration requirements.
				{/if}
			</Alert>
		{/if}
	</div>
{/if}
