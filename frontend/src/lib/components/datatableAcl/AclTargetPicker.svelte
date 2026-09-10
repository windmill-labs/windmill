<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { resource } from 'runed'
	import Select from '../select/Select.svelte'

	let {
		workspace,
		datatable,
		schema = $bindable(),
		table = $bindable()
	}: {
		workspace: string
		datatable: string
		/** Unset for the database itself. */
		schema?: string
		/** Unset for the whole schema. */
		table?: string
	} = $props()

	const schemas = resource(
		() => [workspace, datatable] as const,
		async ([ws, dt]) =>
			(
				await WorkspaceService.getDatatableAcl({
					workspace: ws,
					datatableName: dt,
					kind: 'database'
				})
			).children
	)
	const tables = resource(
		() => [workspace, datatable, schema] as const,
		async ([ws, dt, s]) =>
			s
				? (
						await WorkspaceService.getDatatableAcl({
							workspace: ws,
							datatableName: dt,
							kind: 'schema',
							schema: s
						})
					).children
				: []
	)
</script>

<div class="flex flex-wrap items-center gap-2">
	<Select
		items={(schemas.current ?? []).map((s) => ({ value: s, label: s }))}
		bind:value={
			() => schema,
			(s) => {
				schema = s
				table = undefined
			}
		}
		placeholder="The database itself"
		clearable
		loading={schemas.loading}
		size="sm"
		class="w-56"
	/>
	{#if schema}
		<Select
			items={(tables.current ?? []).map((t) => ({ value: t, label: t }))}
			bind:value={table}
			placeholder="The whole schema"
			clearable
			loading={tables.loading}
			size="sm"
			class="w-56"
		/>
	{/if}
</div>
