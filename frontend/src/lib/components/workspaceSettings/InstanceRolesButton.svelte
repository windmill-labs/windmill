<script lang="ts">
	import { Badge, Button, Drawer, DrawerContent } from '../common'
	import { Users } from 'lucide-svelte'
	import DataTableRolesSection from './DataTableRolesSection.svelte'

	let drawer: Drawer | undefined = $state(undefined)
</script>

<Button
	unifiedSize="sm"
	variant="default"
	startIcon={{ icon: Users }}
	on:click={() => drawer?.openDrawer()}
>
	Instance roles
</Button>

<Drawer bind:this={drawer} size="700px">
	<DrawerContent
		title="Instance roles"
		on:close={() => drawer?.closeDrawer()}
		tooltip="A data table role is a real Postgres login on this instance, shared by every instance database. A job that names one connects as it, and Postgres decides what it may touch. Which people may use a role on a given data table, and what it may do there, is set per data table, in its roles drawer."
	>
		{#snippet titleExtra()}
			<Badge color="blue" small>Beta</Badge>
		{/snippet}
		<DataTableRolesSection />
	</DrawerContent>
</Drawer>
