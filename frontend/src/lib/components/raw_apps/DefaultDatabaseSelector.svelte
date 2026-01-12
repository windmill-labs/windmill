<script lang="ts">
	import { Settings } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		createDatatablesResource,
		createSchemasResource,
		toDatatableItems,
		toSchemaItems
	} from './datatableUtils.svelte'
	import { Button } from '../common'

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
		description = 'Set the default datatable and schema for new tables. This is where AI will create new tables when needed.'
	}: Props = $props()

	// Load available datatables and schemas using shared utilities
	const datatables = createDatatablesResource(() => $workspaceStore)
	const schemas = createSchemasResource(() => datatable)

	const datatableItems = $derived(toDatatableItems(datatables.current))
	const schemaItems = $derived(toSchemaItems(schemas.current))

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
		<Button
			title="Configure default datatable & schema"
			unifiedSize="xs"
			variant="subtle"
			nonCaptureEvent
			btnClasses="px-1"
		>
			<Settings size={12} />
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div class="flex flex-col gap-3 p-4 min-w-64 max-w-80">
			<div class="text-xs font-medium text-primary">Default Datatable & Schemass</div>

			<p class="text-2xs text-tertiary leading-relaxed">
				{description}
			</p>

			<div class="flex flex-col gap-1">
				<span class="text-2xs text-tertiary">Database</span>
				<Select
					items={datatableItems}
					bind:value={() => datatable, (v) => onChange?.(v, schema)}
					placeholder="Select database"
					size="sm"
				/>
			</div>

			<div class="flex flex-col gap-1">
				<span class="text-2xs text-tertiary">Schema</span>
				<Select
					items={schemaItems}
					bind:value={() => schema ?? '', (v) => onChange?.(datatable, v || undefined)}
					placeholder="public"
					size="sm"
				/>
			</div>
		</div>
	</svelte:fragment>
</Popover>
