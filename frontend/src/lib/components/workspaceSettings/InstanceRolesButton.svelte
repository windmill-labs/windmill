<script lang="ts">
	import { Badge, Button, Drawer, DrawerContent } from '../common'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { Users } from 'lucide-svelte'
	import { SettingService, type DatatableRoleCluster } from '$lib/gen'
	import DataTableRolesSection from './DataTableRolesSection.svelte'

	let {
		hideTrigger = false,
		onChanged
	}: {
		/** Mount the drawer without its button, for a caller that opens it with `open()`. */
		hideTrigger?: boolean
		/** Called after every change to the instance roles. */
		onChanged?: () => void
	} = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let prefill = $state('')
	let cluster = $state<DatatableRoleCluster>('instance')
	let externalConfigured = $state(false)
	// Remounts the section on each open, so the prefilled name is the one just asked for.
	let openCount = $state(0)

	/** Opens the drawer on `targetCluster`'s catalog, with `name` prefilled as the role to add. */
	export function open(name = '', targetCluster: DatatableRoleCluster = 'instance') {
		prefill = name
		cluster = targetCluster
		openCount++
		drawer?.openDrawer()
		SettingService.getExternalInstancePgStatus()
			.then((s) => (externalConfigured = s.configured))
			.catch(() => (externalConfigured = false))
	}
</script>

{#if !hideTrigger}
	<Button unifiedSize="sm" variant="default" startIcon={{ icon: Users }} on:click={() => open()}>
		Instance roles
	</Button>
{/if}

<Drawer bind:this={drawer} size="700px">
	<DrawerContent
		title="Instance roles"
		on:close={() => drawer?.closeDrawer()}
		tooltip="A data table role is a real Postgres login on one cluster Windmill manages: this instance's own, shared by every instance database, or the external instance cluster, shared by every external instance database. A job that names one connects as it, and Postgres decides what it may touch. Which people may use a role on a given data table, and what it may do there, is set per data table, in its roles drawer."
	>
		{#snippet titleExtra()}
			<Badge color="blue" small>Beta</Badge>
		{/snippet}
		<div class="flex flex-col gap-4">
			{#if externalConfigured || cluster === 'external_instance'}
				<ToggleButtonGroup
					noWFull
					selected={cluster}
					onSelected={(v) => {
						prefill = ''
						cluster = v
					}}
				>
					{#snippet children({ item })}
						<ToggleButton value="instance" label="Windmill instance" {item} />
						<ToggleButton value="external_instance" label="External instance" {item} />
					{/snippet}
				</ToggleButtonGroup>
			{/if}
			{#key `${openCount}:${cluster}`}
				<DataTableRolesSection {cluster} initialName={prefill} {onChanged} />
			{/key}
		</div>
	</DrawerContent>
</Drawer>
