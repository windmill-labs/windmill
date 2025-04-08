<script lang="ts">
	import { dbSchemas, workspaceStore, type DBSchema } from '$lib/stores'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { sendUserToast } from '$lib/utils'
	import { Loader2, RefreshCcw } from 'lucide-svelte'
	import { getDbSchemas, scripts, type DbType } from './apps/components/display/dbtable/utils'
	import DbExplorer from './DBExplorer.svelte'
	import { Alert } from './common'
	import DbSchemaExplorer from './DBSchemaExplorer.svelte'

	type Props = {
		resourceType: DbType
		resourcePath: string
	}

	let { resourceType, resourcePath }: Props = $props()

	let dbSchema: DBSchema | undefined = $derived(
		resourcePath in $dbSchemas ? $dbSchemas[resourcePath] : undefined
	)
	let drawerRef: Drawer | undefined = $state()
	let mode: 'db-explorer' | 'schema-explorer' = $state('db-explorer')

	let shouldDisplayError = $derived(
		resourcePath && resourcePath in $dbSchemas && !$dbSchemas[resourcePath]
	)

	let refreshing = $state(false)
	$effect(() => {
		if (refreshing) getSchema()
	})

	async function getSchema() {
		if ($dbSchemas[resourcePath] && !refreshing) return
		try {
			await getDbSchemas(
				resourceType,
				resourcePath,
				$workspaceStore,
				$dbSchemas,
				(message: string) => {
					if (drawerRef?.isOpen()) {
						sendUserToast(message, true)
					}
				}
			)
			$dbSchemas = $dbSchemas
		} catch (e) {
			console.error(e)
		} finally {
			refreshing = false
		}
	}

	$effect(() => {
		if (resourcePath && Object.keys(scripts).includes(resourceType)) {
			getSchema()
		}
	})
</script>

{#if !dbSchema}
	<Loader2 size={14} class="animate-spin" />
{:else if shouldDisplayError}
	<Alert type="error" size="xs" title="Schema not available" class="mt-2">
		Schema could not be loaded. Please check the permissions of the resource.
	</Alert>
{:else}
	<Button
		size="xs"
		variant="border"
		color="blue"
		spacingSize="xs2"
		btnClasses="mt-1"
		on:click={drawerRef?.openDrawer ?? (() => {})}
	>
		Explore database
	</Button>
	<Drawer bind:this={drawerRef} size={mode === 'db-explorer' ? '1320px' : '500px'}>
		<DrawerContent
			title={{
				'db-explorer': 'Database Explorer',
				'schema-explorer': 'Schema Explorer'
			}[mode]}
			on:close={drawerRef.closeDrawer}
			noPadding={mode === 'db-explorer'}
		>
			{#if refreshing}
				<div class="h-full flex justify-center items-center">
					<Loader2 size={24} class="animate-spin" />
				</div>
			{:else if mode === 'db-explorer'}
				<DbExplorer {dbSchema} {resourceType} {resourcePath} />
			{:else if mode === 'schema-explorer'}
				<DbSchemaExplorer {dbSchema} />
			{/if}
			<svelte:fragment slot="actions">
				<Button
					btnClasses="!font-normal hover:text-primary text-primary/70"
					size="xs"
					color="light"
					on:click={() => (mode = mode === 'db-explorer' ? 'schema-explorer' : 'db-explorer')}
				>
					{mode === 'db-explorer' ? 'Explore schema' : 'Explore database'}
				</Button>

				<Button
					loading={refreshing}
					on:click={() => (refreshing = true)}
					startIcon={{ icon: RefreshCcw }}
					size="xs"
					color="light"
				>
					Refresh
				</Button>
			</svelte:fragment>
		</DrawerContent>
	</Drawer>
{/if}
