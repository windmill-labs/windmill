<script lang="ts">
	import { type DBSchema } from '$lib/stores'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { formatGraphqlSchema, formatSchema } from './apps/components/display/dbtable/utils'

	type Props = {
		dbSchema: DBSchema
	}

	let { dbSchema }: Props = $props()

	function handleSelected({ detail }: CustomEvent<string>) {
		if (dbSchema && dbSchema.lang !== 'graphql') {
			dbSchema.publicOnly = detail === 'dbo'
		}
	}
</script>

{#if dbSchema.lang !== 'graphql' && (dbSchema.schema?.public || dbSchema.schema?.PUBLIC || dbSchema.schema?.dbo)}
	<ToggleButtonGroup
		class="mb-4"
		selected={dbSchema.publicOnly ? 'dbo' : 'all'}
		on:selected={handleSelected}
	>
		{#snippet children({ item })}
			<ToggleButton value="dbo" label={dbSchema.schema.dbo ? 'Dbo' : 'Public'} {item} />
			<ToggleButton value="all" label="All" {item} />
		{/snippet}
	</ToggleButtonGroup>
{/if}
{#if dbSchema.lang === 'graphql'}
	{#await Promise.all([import('$lib/components/GraphqlSchemaViewer.svelte'), formatGraphqlSchema(dbSchema.schema)])}
		<Loader2 class="animate-spin" />
	{:then [Module, code]}
		<Module.default {code} class="h-full" />
	{/await}
{:else}
	<ObjectViewer json={formatSchema(dbSchema)} pureViewer collapseLevel={1} />
{/if}
