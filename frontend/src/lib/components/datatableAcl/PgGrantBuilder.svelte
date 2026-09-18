<script lang="ts">
	import type { AclTarget } from '$lib/gen'
	import { Plus } from 'lucide-svelte'
	import { Button } from '../common'
	import Select from '../select/Select.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { privilegesOf, scopeSql, scopesOf, type AclScope } from './aclScopes'

	let {
		target,
		roles,
		supportsMaintain = false,
		dbname,
		disabled = false,
		onAdd
	}: {
		target: AclTarget
		/** Roles the grant can be handed to. */
		roles: string[]
		/** Postgres 17+, which has one more table privilege to offer. */
		supportsMaintain?: boolean
		/** Names the database in the statement a database target builds. */
		dbname?: string
		disabled?: boolean
		onAdd: (grant: { role: string; privileges: string[]; scope: AclScope }) => void
	} = $props()

	let role = $state<string | undefined>(undefined)
	let scope = $state<AclScope>('target')
	let privileges = $state<string[]>([])

	const available = $derived(privilegesOf(scope, target.kind, supportsMaintain))
	const statement = $derived(
		privileges.length && role
			? `GRANT ${privileges.join(', ')} ON ${scopeSql(scope, target, dbname)} TO ${role}`
			: undefined
	)
</script>

<div class="flex flex-col gap-2 border rounded-md p-3">
	<div class="flex flex-wrap items-center gap-2 text-xs text-secondary">
		<span class="font-mono text-primary">GRANT</span>
		<MultiSelect
			bind:value={privileges}
			items={available.map((p) => ({ value: p, label: p }))}
			placeholder="privileges"
			{disabled}
			size="sm"
			class="min-w-48"
		/>
		<span class="font-mono text-primary">ON</span>
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
			items={scopesOf(target.kind)}
			{disabled}
			size="sm"
			class="w-52"
		/>
		<span class="font-mono text-primary">TO</span>
		<Select
			bind:value={role}
			items={roles.map((r) => ({ value: r, label: r }))}
			placeholder="role"
			{disabled}
			size="sm"
			class="w-40"
		/>
		<Button
			unifiedSize="sm"
			variant="default"
			startIcon={{ icon: Plus }}
			disabled={disabled || !role || privileges.length === 0}
			onClick={() => {
				if (!role) return
				onAdd({ role, privileges: [...privileges], scope })
				privileges = []
			}}
		>
			Add
		</Button>
	</div>
	{#if statement}
		<pre class="text-2xs text-tertiary overflow-x-auto">{statement}</pre>
	{/if}
</div>
