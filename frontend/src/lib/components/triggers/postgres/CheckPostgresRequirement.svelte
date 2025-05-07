<script lang="ts">
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { PostgresTriggerService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { emptyString } from '$lib/utils'

	let loadingConfiguration = false

	const checkDatabaseConfiguration = async () => {
		if (emptyString(postgres_resource_path)) {
			sendUserToast('You must first pick a database resource', true)
			return
		}
		try {
			const invalidConfig = !(await PostgresTriggerService.isValidPostgresConfiguration({
				workspace: $workspaceStore!,
				path: postgres_resource_path
			}))

			let msg = 'Database is in logical mode. Triggers can be used.'

			if (invalidConfig) {
				msg =
					'Database is NOT in logical mode. Triggers cannot be used. Refer to the PostgreSQL documentation for configuration requirements.'
			}

			sendUserToast(msg, invalidConfig)
		} catch (error) {
			sendUserToast(error.body, true)
		}
		
		loadingConfiguration = false
	}

	const checkConnectionAndDatabaseConfiguration = async () => {
		try {
			loadingConfiguration = true
			if (checkConnection) {
				await checkConnection()
			}
			await checkDatabaseConfiguration()
		} catch (error) {
			sendUserToast(error.body, true)
		}
		loadingConfiguration = false
	}

	export let can_write: boolean
	export let postgres_resource_path: string
	export let checkConnection: any | undefined = undefined
</script>

{#if postgres_resource_path}
	<div class="flex flex-col justify-end mt-1 gap-2">
		<Button
			disabled={!can_write}
			loading={loadingConfiguration}
			on:click={checkConnectionAndDatabaseConfiguration}
			size="xs"
			color="light"
			spacingSize="sm"
			variant="border"
		>
			{`Check database configuration ${checkConnection ? 'and connection' : ''}`}
			<Tooltip
				documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#requirements"
			>
				<p class="text-sm">
					Verifies whether the database is configured with the required <strong>settings</strong>.
					{checkConnection && 'Also checks whether the connection to the database is working.'}
				</p>
			</Tooltip>
		</Button>
	</div>
{/if}
