<script lang="ts">
	import type { AclTarget } from '$lib/gen'
	import type { Snippet } from 'svelte'
	import { Plus } from 'lucide-svelte'
	import { Button } from '../common'
	import Select from '../select/Select.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { privilegesOf, scopesOf, type AclScope } from './aclScopes'

	let {
		target,
		roles,
		supportsMaintain = false,
		disabled = false,
		manageRoles,
		onAdd
	}: {
		target: AclTarget
		/** Roles the grant can be handed to. */
		roles: string[]
		/** Postgres 17+, which has one more table privilege to offer. */
		supportsMaintain?: boolean
		disabled?: boolean
		/** Rendered at the bottom of the role picker, to reach the data table's roles. */
		manageRoles?: Snippet<[{ close: () => void }]>
		onAdd: (grant: { role: string; privileges: string[]; scope: AclScope }) => void
	} = $props()

	let role = $state<string | undefined>(undefined)
	let scope = $state<AclScope>('target')
	let privileges = $state<string[]>([])

	const scopes = $derived(scopesOf(target.kind))
	const available = $derived(privilegesOf(scope, target.kind, supportsMaintain))
</script>

<!-- Laid out like the grants above it, so it reads as the statement it will run. -->
<div class="flex items-center gap-2 text-xs text-secondary">
	<span class="font-mono shrink-0">GRANT</span>
	<MultiSelect
		bind:value={privileges}
		items={available.map((p) => ({ value: p, label: p }))}
		placeholder="privileges"
		{disabled}
		size="md"
		class="min-w-0 flex-[4]"
	/>
	<!-- A database or a table has only itself to grant on. -->
	{#if scopes.length > 1}
		<span class="font-mono shrink-0">ON</span>
		<Select
			bind:value={
				() => scope,
				(s) => {
					if (!s) return
					scope = s
					// A privilege only exists for some objects — SELECT means nothing on a function —
					// so drop what the new scope cannot carry rather than send it.
					const allowed = privilegesOf(s, target.kind, supportsMaintain)
					privileges = privileges.filter((p) => allowed.includes(p))
				}
			}
			items={scopes}
			{disabled}
			size="md"
			class="min-w-0 flex-[3]"
		/>
	{/if}
	<span class="font-mono shrink-0">TO</span>
	<Select
		bind:value={role}
		items={roles.map((r) => ({ value: r, label: r }))}
		placeholder="role"
		{disabled}
		size="md"
		class="min-w-0 flex-[2]"
		bottomSnippet={manageRoles}
	/>
	<Button
		unifiedSize="md"
		variant="default"
		wrapperClasses="shrink-0"
		startIcon={{ icon: Plus }}
		disabled={disabled || !role || privileges.length === 0}
		onClick={() => {
			if (!role) return
			onAdd({ role, privileges: [...privileges], scope })
			privileges = []
		}}
	>
		Create
	</Button>
</div>
