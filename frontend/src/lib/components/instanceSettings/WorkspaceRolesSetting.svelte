<script lang="ts">
	import type { Writable } from 'svelte/store'
	import { untrack } from 'svelte'
	import { Alert, Badge, Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import RoleCapabilitiesEditor from '../settings/RoleCapabilitiesEditor.svelte'
	import type { RoleCapabilities, RoleCapabilitiesPatch, RoleDefinition } from '$lib/gen'
	import {
		applyCapabilitiesPatch,
		authorLevelLabel,
		baseCapabilitiesFor,
		DEFAULT_INSTANCE_ROLES,
		DEVELOPER_ROLE,
		isDefaultInstanceRoles,
		isEmptyPatch,
		workspaceRolesErrors
	} from '$lib/workspaceRoles'
	import {
		ArrowDown,
		ArrowUp,
		ChevronDown,
		ChevronRight,
		Plus,
		RotateCcw,
		Trash2
	} from 'lucide-svelte'

	interface Props {
		values: Writable<Record<string, any>>
		loading?: boolean
		/** Without a valid license the stored list keeps applying but can only be reset. */
		unlicensed?: boolean
	}

	let { values, loading = false, unlicensed = false }: Props = $props()

	const disabled = $derived(loading || unlicensed)

	const KEY = 'workspace_roles'

	type Row = {
		id: number
		name: string
		label: string
		capabilities: RoleCapabilitiesPatch
		open: boolean
	}

	let nextId = 0
	let rows = $state<Row[]>([])
	// The JSON last read from or written to `$values[KEY]`, to tell our own writes from external
	// ones (the async load, Save, Discard) and re-hydrate only on the latter.
	let lastSynced: string | undefined = undefined

	function toRows(v: unknown): Row[] {
		const list = Array.isArray(v) && v.length > 0 ? (v as RoleDefinition[]) : DEFAULT_INSTANCE_ROLES
		return list.map((r) => ({
			id: nextId++,
			name: r.name ?? '',
			label: r.label ?? '',
			capabilities: structuredClone(r.capabilities ?? {}),
			open: false
		}))
	}

	function serialize(rs: Row[]): RoleDefinition[] | undefined {
		const list: RoleDefinition[] = rs.map((r) => ({
			name: r.name.trim(),
			...(r.label.trim() ? { label: r.label.trim() } : {}),
			...(isEmptyPatch(r.capabilities) ? {} : { capabilities: r.capabilities })
		}))
		return isDefaultInstanceRoles(list) ? undefined : list
	}

	$effect(() => {
		const cur = JSON.stringify($values[KEY] ?? null)
		if (cur === lastSynced) return
		lastSynced = cur
		const open = new Set(rows.filter((r) => r.open).map((r) => r.name))
		rows = toRows($values[KEY]).map((r) => ({
			...r,
			open: open.has(r.name)
		}))
	})

	$effect(() => {
		const next = serialize($state.snapshot(rows) as Row[])
		if (lastSynced === undefined || disabled) return
		const nextStr = JSON.stringify(next ?? null)
		// Compared with the stored value in serialized form, so one spelled differently (the
		// defaults written out, an empty `capabilities`, another key order) is not rewritten
		// and the category not marked unsaved before anything is edited.
		const stored = untrack(() => $values[KEY])
		if (nextStr === JSON.stringify(serialize(toRows(stored)) ?? null)) return
		lastSynced = nextStr
		$values[KEY] = next
	})

	const errors = $derived(workspaceRolesErrors(serialize(rows) ?? DEFAULT_INSTANCE_ROLES))

	function move(i: number, by: -1 | 1) {
		const next = [...rows]
		;[next[i], next[i + by]] = [next[i + by], next[i]]
		rows = next
	}

	function addRole() {
		rows = [...rows, { id: nextId++, name: '', label: '', capabilities: {}, open: true }]
	}

	function summary(caps: RoleCapabilities): string {
		return [
			caps.code ? 'Code' : 'No code',
			`Flows: ${authorLevelLabel(caps.flows).toLowerCase()}`,
			`Apps: ${authorLevelLabel(caps.apps).toLowerCase()}`
		].join(' · ')
	}
</script>

<div class="flex flex-col gap-3">
	{#if unlicensed}
		<Alert
			type="info"
			size="xs"
			title="Changing workspace roles requires a valid Windmill Enterprise Edition license"
		>
			The roles below keep applying as they are. A workspace admin can still override what they
			allow in that workspace's settings.
			{#if serialize(rows) !== undefined}
				<div class="mt-2">
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: RotateCcw }}
						disabled={loading}
						onclick={() => ($values[KEY] = undefined)}
					>
						Reset to the default roles
					</Button>
				</div>
			{/if}
		</Alert>
	{/if}
	<span class="text-xs text-secondary">
		Roles members can hold in every workspace, in order: when several instance groups map a member
		to different roles, the role listed first wins. A capability left unset uses the default:
		everything for a role named <code>developer</code>, the operator rights for any other name.
		Workspace admins can override a role in their workspace. A role still held by a member, invite,
		auto-invite or instance group mapping cannot be removed.
	</span>

	{#each rows as row, i (row.id)}
		{@const base = baseCapabilitiesFor(row.name)}
		{@const caps = applyCapabilitiesPatch(base, row.capabilities)}
		<div class="flex flex-col rounded-md border bg-surface-tertiary">
			<div class="flex items-center gap-2 p-2">
				<div class="flex flex-col">
					<Button
						variant="subtle"
						unifiedSize="2xs"
						iconOnly
						startIcon={{ icon: ArrowUp }}
						aria-label="Move up"
						disabled={disabled || i === 0}
						onclick={() => move(i, -1)}
					/>
					<Button
						variant="subtle"
						unifiedSize="2xs"
						iconOnly
						startIcon={{ icon: ArrowDown }}
						aria-label="Move down"
						disabled={disabled || i === rows.length - 1}
						onclick={() => move(i, 1)}
					/>
				</div>
				<div class="flex flex-col gap-0.5 w-40 shrink-0">
					<TextInput
						size="sm"
						bind:value={row.name}
						error={errors[`${i}.name`]}
						inputProps={{ placeholder: 'name', disabled, 'aria-label': 'Role name' }}
					/>
				</div>
				<div class="w-48 shrink-0">
					<TextInput
						size="sm"
						bind:value={row.label}
						inputProps={{
							placeholder: row.name || 'label',
							disabled,
							'aria-label': 'Role label'
						}}
					/>
				</div>
				<span class="text-2xs text-secondary truncate grow">{summary(caps)}</span>
				{#if i === 0}
					<Badge color="gray" verySmall>Highest</Badge>
				{/if}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: row.open ? ChevronDown : ChevronRight }}
					onclick={() => (row.open = !row.open)}
				>
					Capabilities
				</Button>
				<Button
					variant="subtle"
					unifiedSize="sm"
					destructive
					iconOnly
					startIcon={{ icon: Trash2 }}
					aria-label="Remove role"
					title={rows.length === 1 ? 'Keep at least one role' : 'Remove role'}
					disabled={disabled || rows.length === 1}
					onclick={() => (rows = rows.filter((r) => r.id !== row.id))}
				/>
			</div>
			{#if errors[`${i}.name`]}
				<span class="text-2xs text-red-600 dark:text-red-400 px-2 pb-2 -mt-1">
					{errors[`${i}.name`]}
				</span>
			{/if}
			{#if row.open}
				<div class="border-t p-3">
					<RoleCapabilitiesEditor
						{base}
						patch={row.capabilities}
						baseName={row.name === DEVELOPER_ROLE ? 'developer default' : 'operator default'}
						setText="Set"
						inheritedText="Default"
						{disabled}
						onchange={(p) => (row.capabilities = p)}
					/>
				</div>
			{:else if errors[`${i}.capabilities`]}
				<span class="text-2xs text-red-600 dark:text-red-400 px-2 pb-2">
					{errors[`${i}.capabilities`]}
				</span>
			{/if}
		</div>
	{/each}

	{#if !disabled}
		<div>
			<Button variant="default" unifiedSize="sm" startIcon={{ icon: Plus }} onclick={addRole}>
				Add role
			</Button>
		</div>
	{/if}
</div>
