<script lang="ts">
	import { Settings } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		createDatatableAccessResource,
		createDatatablesResource,
		toDatatableItems,
		toSchemaItems
	} from './datatableUtils.svelte'
	import { Button } from '../common'
	import { getRawAppOperatingWorkspace } from './rawAppWorkspace'

	const getOpWs = getRawAppOperatingWorkspace()
	let opWs = $derived(getOpWs?.() ?? $workspaceStore)

	interface Props {
		/** Currently selected datatable */
		datatable: string | undefined
		/** Currently selected schema */
		schema: string | undefined
		/** The data table role the app's queries run as, if it names one. What a
		 * role may see is what the schema list has to be read as. */
		role?: string | undefined
		/** Callback when either value changes */
		onChange?: (datatable: string | undefined, schema: string | undefined) => void
		/** Description text to show in the popover */
		description?: string
	}

	let {
		datatable,
		schema,
		role = undefined,
		onChange,
		description = 'Set the default datatable and schema for new tables. This is where AI will create new tables when needed.'
	}: Props = $props()

	// Load available datatables and schemas using shared utilities
	const datatables = createDatatablesResource(() => opWs)
	const access = createDatatableAccessResource(
		() => datatable,
		() => role,
		() => opWs
	)

	const datatableItems = $derived(toDatatableItems(datatables.current))
	// The answer says what it answers for: switching the database above leaves the
	// previous one's schemas in hand until the refetch lands, and picking from
	// them would name a schema this data table may not have.
	const schemaItems = $derived(
		access.current.datatable === datatable && access.current.role === role
			? toSchemaItems(access.current.schemas)
			: []
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
	{#snippet trigger()}
		<Button
			title="Configure default datatable & schema"
			unifiedSize="xs"
			variant="subtle"
			nonCaptureEvent
			btnClasses="px-1"
		>
			<Settings size={12} />
		</Button>
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col gap-3 p-4 min-w-64 max-w-80">
			<div class="text-xs font-medium text-primary">Default Datatable & Schema</div>

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
	{/snippet}
</Popover>
