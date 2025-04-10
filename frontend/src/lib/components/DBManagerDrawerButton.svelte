<script lang="ts">
	import { dbSchemas, workspaceStore, type DBSchema } from '$lib/stores'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { sendUserToast } from '$lib/utils'
	import { Database, Expand, Loader2, Minimize, RefreshCcw } from 'lucide-svelte'
	import {
		getDbSchemas,
		loadTableMetaData,
		scripts,
		type DbType
	} from './apps/components/display/dbtable/utils'
	import DbManager from './DBManager.svelte'
	import { Alert } from './common'
	import DbSchemaExplorer from './DBSchemaExplorer.svelte'
	import { dbDeleteTableActionWithPreviewScript, dbTableOpsWithPreviewScripts } from './dbOps'
	import { makeCreateTableQuery } from './apps/components/display/dbtable/queries/createTable'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import { type ScriptLang } from '$lib/gen'

	type Props = {
		resourceType: DbType
		resourcePath: string
		class?: string
	}

	let { resourceType, resourcePath, class: className = '' }: Props = $props()

	let dbSchema: DBSchema | undefined = $derived(
		resourcePath in $dbSchemas ? $dbSchemas[resourcePath] : undefined
	)

	let isDrawerOpen: boolean = $state(false)
	let mode: 'db-manager' | 'schema-explorer' = $state('db-manager')

	let shouldDisplayError = $derived(
		resourcePath && resourcePath in $dbSchemas && !$dbSchemas[resourcePath]
	)

	let refreshing = $state(false)
	$effect(() => {
		if (refreshing) getSchema()
	})

	let expand = $state(false)
	$effect(() => {
		if (!isDrawerOpen) expand = false
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
					if (isDrawerOpen) {
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

	let windowWidth = $state(window.innerWidth)
</script>

<svelte:window bind:innerWidth={windowWidth} />

{#if !dbSchema || !$workspaceStore}
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
		btnClasses={'mt-1 w-fit ' + className}
		on:click={() => (isDrawerOpen = true)}
	>
		<Database size={18} /> Manager
	</Button>
	<Drawer
		bind:open={isDrawerOpen}
		size={(
			{
				'db-manager': expand ? `${windowWidth}px` : '1200px',
				'schema-explorer': '500px'
			} satisfies Record<typeof mode, `${number}px`>
		)[mode]}
	>
		<DrawerContent
			title={(
				{
					'db-manager': 'Database Manager',
					'schema-explorer': 'Schema Explorer'
				} satisfies Record<typeof mode, string>
			)[mode]}
			on:close={() => (isDrawerOpen = false)}
			noPadding={mode === 'db-manager'}
		>
			{#if refreshing}
				<div class="h-full flex justify-center items-center">
					<Loader2 size={24} class="animate-spin" />
				</div>
			{:else if mode === 'db-manager'}
				<DbManager
					{dbSchema}
					getColDefs={async (tableKey) =>
						(await loadTableMetaData(
							'$res:' + resourcePath,
							$workspaceStore,
							tableKey,
							resourceType
						)) ?? []}
					dbTableOpsFactory={({ colDefs, tableKey }) =>
						dbTableOpsWithPreviewScripts({
							colDefs,
							tableKey,
							resourcePath,
							resourceType,
							workspace: $workspaceStore
						})}
					dbTableActionsFactory={[
						dbDeleteTableActionWithPreviewScript({
							resourcePath,
							resourceType,
							workspace: $workspaceStore
						})
					]}
					refresh={() => {
						refreshing = true
					}}
					dbTableEditorPropsFactory={({ selectedSchemaKey }) => ({
						resourceType,
						previewSql: (values) => makeCreateTableQuery(values, resourceType, selectedSchemaKey),
						async onConfirm(values) {
							await runPreviewJobAndPollResult({
								workspace: $workspaceStore,
								requestBody: {
									args: { database: '$res:' + resourcePath },
									content: makeCreateTableQuery(values, resourceType, selectedSchemaKey),
									language: resourceType as ScriptLang
								}
							})
							refreshing = true
						}
					})}
				/>
			{:else if mode === 'schema-explorer'}
				<DbSchemaExplorer {dbSchema} />
			{/if}
			<svelte:fragment slot="actions">
				<Button
					btnClasses="!font-normal hover:text-primary text-primary/70"
					size="xs"
					color="light"
					on:click={() => (mode = mode === 'db-manager' ? 'schema-explorer' : 'db-manager')}
				>
					{mode === 'db-manager' ? 'Explore schema' : 'Manage database'}
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

				{#if mode === 'db-manager'}
					<Button
						on:click={() => (expand = !expand)}
						startIcon={{ icon: expand ? Minimize : Expand }}
						size="xs"
						color="light"
					/>
				{/if}
			</svelte:fragment>
		</DrawerContent>
	</Drawer>
{/if}
