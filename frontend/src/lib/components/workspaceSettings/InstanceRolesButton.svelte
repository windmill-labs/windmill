<script lang="ts">
	import { Badge, Button, Drawer, DrawerContent } from '../common'
	import { Users } from 'lucide-svelte'
	import DataTableRolesSection from './DataTableRolesSection.svelte'

	let {
		hideTrigger = false,
		onChanged,
		unavailable
	}: {
		/** Mount the drawer without its button, for a caller that opens it with `open()`. */
		hideTrigger?: boolean
		/** Called after every change to the instance roles. */
		onChanged?: () => void
		/** Why the roles cannot be managed here: the button stays, disabled with this reason, so the
		 * feature can be found. `ee` marks the reason that is the edition. */
		unavailable?: { reason: string; ee: boolean }
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
	{#if unavailable}
		<!-- A disabled button shows no title of its own: the wrapper carries the reason. -->
		<span title={unavailable.reason} class="inline-flex cursor-not-allowed">
			<Button unifiedSize="sm" variant="default" startIcon={{ icon: Users }} disabled>
				Instance roles{unavailable.ee ? ' (EE)' : ''}
			</Button>
		</span>
	{:else}
		<Button unifiedSize="sm" variant="default" startIcon={{ icon: Users }} on:click={() => open()}>
			Instance roles
		</Button>
	{/if}
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
