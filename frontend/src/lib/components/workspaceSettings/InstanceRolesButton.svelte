<script lang="ts">
	import { Badge, Button, Drawer, DrawerContent } from '../common'
	import { Users } from 'lucide-svelte'
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
	// Remounts the section on each open, so the prefilled name is the one just asked for.
	let openCount = $state(0)

	/** Opens the drawer, with `name` prefilled as the role to add. */
	export function open(name = '') {
		prefill = name
		openCount++
		drawer?.openDrawer()
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
		tooltip="A data table role is a real Postgres login on this instance, shared by every instance database. A job that names one connects as it, and Postgres decides what it may touch. Which people may use a role on a given data table, and what it may do there, is set per data table, in its roles drawer."
	>
		{#snippet titleExtra()}
			<Badge color="blue" small>Beta</Badge>
		{/snippet}
		{#key openCount}
			<DataTableRolesSection initialName={prefill} {onChanged} />
		{/key}
	</DrawerContent>
</Drawer>
