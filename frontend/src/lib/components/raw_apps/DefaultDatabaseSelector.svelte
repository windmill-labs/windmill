<script lang="ts">
	import { Settings } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { workspaceStore, dbSchemas } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import { resource } from 'runed'
	import { getDbSchemas } from '$lib/components/apps/components/display/dbtable/metadata'
	import Select from '$lib/components/select/Select.svelte'

	interface Props {
		/** Currently selected datatable */
		datatable: string | undefined
		/** Currently selected schema */
		schema: string | undefined
		/** Callback when either value changes */
		onChange?: (datatable: string | undefined, schema: string | undefined) => void
		/** Description text to show in the popover */
		description?: string
	}

	let {
		datatable,
		schema,
		onChange,
		description = 'Set the default database and schema for new tables. This is where AI will create new tables when needed.'
	}: Props = $props()

	// Load available datatables from workspace
	const datatables = resource<string[]>([], async () => {
		if (!$workspaceStore) return []
		try {
			return await WorkspaceService.listDataTables({ workspace: $workspaceStore })
		} catch (e) {
			console.error('Failed to load datatables:', e)
			return []
		}
	})

	// Load schemas for the selected datatable
	const schemas = resource<string[]>([], async () => {
		if (!datatable || !$workspaceStore) return []

		const resourcePath = `datatable://${datatable}`
		let dbSchema = $dbSchemas[resourcePath]

		if (!dbSchema) {
			try {
				await getDbSchemas('postgresql', resourcePath, $workspaceStore, $dbSchemas, (msg) =>
					console.error('Schema error:', msg)
				)
				dbSchema = $dbSchemas[resourcePath]
			} catch (e) {
				console.error(`Failed to load schema for ${datatable}:`, e)
				return []
			}
		}

		if (!dbSchema?.schema) return []
		return Object.keys(dbSchema.schema)
	})

	const datatableItems = $derived(
		datatables.current.map((dt) => ({
			value: dt,
			label: dt
		}))
	)

	const schemaItems = $derived(
		schemas.current.map((s) => ({
			value: s,
			label: s
		}))
	)

	// Track datatable changes to reset schema
	let previousDatatable = $state<string | undefined>(undefined)
	$effect(() => {
		if (previousDatatable !== undefined && datatable !== previousDatatable) {
			// Reset schema when datatable changes
			onChange?.(datatable, undefined)
		}
		previousDatatable = datatable
	})
</script>

<Popover>
	<svelte:fragment slot="trigger">
		<button
			class="pt-1.5 pb-0.5 px-1 hover:bg-surface-hover rounded transition-colors"
			title="Configure default database & schema"
		>
			<Settings size={12} class="text-tertiary" />
		</button>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div class="flex flex-col gap-3 p-2 min-w-64 max-w-80">
			<div class="text-xs font-medium text-primary">Default Database & Schema</div>

			<p class="text-2xs text-tertiary leading-relaxed">
				{description}
			</p>

			<div class="flex flex-col gap-1">
				<span class="text-2xs text-tertiary">Database</span>
				<Select
					items={datatableItems}
					bind:value={
						() => datatable,
						(v) => onChange?.(v, schema)
					}
					placeholder="Select database"
					size="sm"
				/>
			</div>

			<div class="flex flex-col gap-1">
				<span class="text-2xs text-tertiary">Schema</span>
				<Select
					items={schemaItems}
					bind:value={
						() => schema ?? '',
						(v) => onChange?.(datatable, v || undefined)
					}
					placeholder="public"
					size="sm"
				/>
			</div>
		</div>
	</svelte:fragment>
</Popover>
