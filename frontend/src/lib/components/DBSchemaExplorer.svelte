<script lang="ts">
	import { dbSchemas, workspaceStore, type DBSchema } from '$lib/stores'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import { sendUserToast } from '$lib/utils'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import GraphqlSchemaViewer from './GraphqlSchemaViewer.svelte'
	import { RefreshCcw } from 'lucide-svelte'
	import {
		formatGraphqlSchema,
		formatSchema,
		getDbSchemas,
		scripts
	} from './apps/components/display/dbtable/utils'

	export let resourceType: string | undefined
	export let resourcePath: string | undefined = undefined
	let dbSchema: DBSchema | undefined = undefined
	let loading = false

	let drawer: Drawer | undefined

	async function getSchema() {
		if (!resourceType || !resourcePath) return
		loading = true

		await getDbSchemas(
			resourceType,
			resourcePath,
			$workspaceStore,
			$dbSchemas,
			(message: string) => {
				if (drawer?.isOpen()) {
					sendUserToast(message, true)
				}
			}
		)
	}

	$: resourcePath &&
		Object.keys(scripts).includes(resourceType || '') &&
		!$dbSchemas[resourcePath] &&
		getSchema()

	$: dbSchema = resourcePath && resourcePath in $dbSchemas ? $dbSchemas[resourcePath] : undefined
</script>

{#if dbSchema}
	<Button
		size="xs"
		variant="border"
		color="blue"
		spacingSize="xs2"
		btnClasses="mt-1"
		on:click={drawer?.openDrawer}
	>
		Explore schema
	</Button>
	<Drawer bind:this={drawer}>
		<DrawerContent title="Schema Explorer" on:close={drawer.closeDrawer}>
			<svelte:fragment slot="actions">
				<Button
					on:click={getSchema}
					startIcon={{
						icon: RefreshCcw
					}}
					{loading}
					size="xs"
					color="light"
				>
					Refresh
				</Button>
			</svelte:fragment>
			{#if dbSchema.lang !== 'graphql' && (dbSchema.schema?.public || dbSchema.schema?.PUBLIC || dbSchema.schema?.dbo)}
				<ToggleButtonGroup class="mb-4" bind:selected={dbSchema.publicOnly}>
					<ToggleButton value={true} label={dbSchema.schema.dbo ? 'Dbo' : 'Public'} />
					<ToggleButton value={false} label="All" />
				</ToggleButtonGroup>
			{/if}
			{#if dbSchema.lang === 'graphql'}
				<GraphqlSchemaViewer code={formatGraphqlSchema(dbSchema)} class="h-full" />
			{:else}
				<ObjectViewer json={formatSchema(dbSchema)} pureViewer />
			{/if}
		</DrawerContent>
	</Drawer>
{/if}
