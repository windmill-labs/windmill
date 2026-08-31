<script lang="ts">
	import type { AclScope } from './aclScopes'
	import { privilegesOf, scopeSql, scopesOf } from './aclScopes'
	import type { AclTarget } from '$lib/gen'
	import Select from '../select/Select.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import Button from '../common/button/Button.svelte'
	import { Plus } from 'lucide-svelte'

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

	const scopeItems = $derived(
		scopesOf(target.kind).map((s) => ({ value: s.value, label: s.label }))
	)
	const available = $derived(privilegesOf(scope, target.kind, supportsMaintain))

	// A privilege only exists for some objects: SELECT means nothing on a
	// function, so drop what the new scope cannot carry rather than send it.
	$effect(() => {
		const allowed = privilegesOf(scope, target.kind, supportsMaintain)
		const kept = privileges.filter((p) => allowed.includes(p))
		if (kept.length !== privileges.length) privileges = kept
	})

	const statement = $derived(
		privileges.length && role
			? `GRANT ${privileges.join(', ')} ON ${scopeSql(scope, target, dbname)} TO ${role}`
			: undefined
	)
	const canAdd = $derived(!!role && privileges.length > 0)
</script>

<div class="flex flex-col gap-2 border rounded-md p-3">
	<div class="flex flex-wrap items-center gap-2 text-xs text-secondary">
		<span class="font-mono text-primary">GRANT</span>
		<MultiSelect
			bind:value={privileges}
			items={available.map((p) => ({ value: p, label: p }))}
			placeholder="privileges"
			{disabled}
			class="min-w-56"
		/>
		<span class="font-mono text-primary">ON</span>
		<Select bind:value={scope} items={scopeItems} clearable={false} {disabled} class="w-52" />
		<span class="font-mono text-primary">TO</span>
		<Select
			bind:value={role}
			items={roles.map((r) => ({ value: r, label: r }))}
			placeholder="role"
			{disabled}
			class="w-40"
		/>
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: Plus }}
			disabled={disabled || !canAdd}
			on:click={() => {
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
