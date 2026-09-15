<script lang="ts">
	import { untrack } from 'svelte'
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
	import { appDatatableRole } from './dataTableRefUtils'

	const getOpWs = getRawAppOperatingWorkspace()
	let opWs = $derived(getOpWs?.() ?? $workspaceStore)

	interface Props {
		/** Currently selected datatable */
		datatable: string | undefined
		/** Currently selected schema */
		schema: string | undefined
		/** The role the app uses each data table through: schemas are listed as that role. */
		roles?: Record<string, string>
		/** Callback when either value changes */
		onChange?: (datatable: string | undefined, schema: string | undefined) => void
		/** Description text to show in the popover */
		description?: string
	}

	let {
		datatable,
		schema,
		roles,
		onChange,
		description = 'Set the default datatable and schema for new tables. This is where AI will create new tables when needed.'
	}: Props = $props()

	const role = $derived(datatable ? appDatatableRole(roles, datatable) : undefined)

	// Load available datatables and schemas using shared utilities
	const datatables = createDatatablesResource(() => opWs)
	const access = createDatatableAccessResource(
		() => datatable,
		() => role,
		() => opWs
	)

	const datatableItems = $derived(toDatatableItems(datatables.current))
	// Until the answer is for this data table and role, the schemas in hand belong to another.
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

	// The app's role on the data table can change without the data table changing (tables added
	// under another role): a default schema that role no longer reaches is where the AI could not
	// create tables, so it is unset once the answer for this role is in. Only after a role change
	// made here: opening an app must not modify it.
	// svelte-ignore state_referenced_locally
	const roleOnOpen = role
	$effect(() => {
		const answer = access.current
		if (
			role !== roleOnOpen &&
			schema !== undefined &&
			answer.datatable === datatable &&
			answer.role === role &&
			!answer.failed &&
			!answer.schemas.includes(schema)
		) {
			untrack(() => onChange?.(datatable, undefined))
		}
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
				{#if role}
					<span class="text-2xs text-tertiary">Used as role {role}</span>
				{/if}
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
