<script lang="ts">
	import { Alert, Badge, Button, Section } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RoleCapabilitiesEditor from './RoleCapabilitiesEditor.svelte'
	import {
		WorkspaceService,
		type EffectiveRole,
		type RoleCapabilities,
		type RoleOverride,
		type RoleOverrides,
		type WorkspaceRoles
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import {
		applyCapabilitiesPatch,
		authorLevelLabel,
		baseCapabilitiesFor,
		capabilitiesError,
		isEmptyPatch,
		roleNameError,
		VISIBILITY_KEYS
	} from '$lib/workspaceRoles'
	import { ChevronDown, ChevronRight, Plus, RotateCcw, SaveIcon, Trash2 } from 'lucide-svelte'
	import { SvelteMap } from 'svelte/reactivity'

	interface Props {
		workspace: string
		/** The workspace's roles as last read; undefined while loading. */
		data: WorkspaceRoles | undefined
		onSaved: () => void
	}

	let { workspace, data, onSaved }: Props = $props()

	// Maps, not objects: a role name is any slug, `constructor` and `__proto__` included.
	const draft = new SvelteMap<string, RoleOverride>()
	let loadedFrom: WorkspaceRoles | undefined = undefined
	const open = new SvelteMap<string, boolean>()
	let newRoleName = $state('')
	let confirmSeatOpen = $state(false)
	let saving = $state(false)

	$effect(() => {
		if (data && data !== loadedFrom) {
			loadedFrom = data
			draft.clear()
			for (const [name, o] of Object.entries(data.overrides ?? {})) {
				draft.set(name, structuredClone($state.snapshot(o)))
			}
		}
	})

	const instanceRoles = $derived((data?.roles ?? []).filter((r) => r.source === 'instance'))
	const workspaceOnlyNames = $derived(
		[...draft.keys()].filter((n) => !instanceRoles.some((r) => r.name === n)).sort()
	)

	function overriddenKeys(o: RoleOverride | undefined): string[] {
		const c = o?.capabilities ?? {}
		const keys = ['code', 'flows', 'apps', 'manage_schedules', 'manage_triggers'].filter(
			(k) => c[k] != null
		)
		for (const { key } of VISIBILITY_KEYS) {
			if (c.visibility?.[key] != null) keys.push(`visibility.${key}`)
		}
		return keys
	}

	/**
	 * The instance layer of a role. The listing gives only capabilities with the stored override
	 * applied, so a field that override sets has no known instance value: `unknown` lists them.
	 */
	function baseFor(name: string): { base: RoleCapabilities; unknown: string[] } {
		const role = instanceRoles.find((r) => r.name === name)
		if (!role) return { base: baseCapabilitiesFor(name), unknown: [] }
		const stored = data && Object.hasOwn(data.overrides, name) ? data.overrides[name] : undefined
		return { base: role.capabilities, unknown: overriddenKeys(stored) }
	}

	function effectiveFor(name: string): RoleCapabilities {
		return applyCapabilitiesPatch(baseFor(name).base, draft.get(name)?.capabilities)
	}

	const AUTHORING_KEYS = ['code', 'flows', 'apps']

	/** Whether saving makes an authoring field of `name` fall back to an unknown instance value. */
	function authoringUnknown(name: string): boolean {
		const set = overriddenKeys(draft.get(name))
		return baseFor(name).unknown.some((k) => AUTHORING_KEYS.includes(k) && !set.includes(k))
	}

	function isAuthor(c: RoleCapabilities): boolean {
		return c.code || c.flows !== 'none' || c.apps !== 'none'
	}

	function summary(c: RoleCapabilities): string {
		return [
			c.code ? 'Code' : 'No code',
			`Flows: ${authorLevelLabel(c.flows).toLowerCase()}`,
			`Apps: ${authorLevelLabel(c.apps).toLowerCase()}`
		].join(' · ')
	}

	/** The overrides to store: instance roles left as the instance has them are dropped. */
	function cleaned(entries: Iterable<[string, RoleOverride]>): RoleOverrides {
		const out: [string, RoleOverride][] = []
		for (const [name, v] of entries) {
			const label = v.label?.trim() || undefined
			const isInstance = instanceRoles.some((r) => r.name === name)
			if (isInstance && !label && isEmptyPatch(v.capabilities)) continue
			out.push([
				name,
				{
					...(label ? { label } : {}),
					capabilities: isEmptyPatch(v.capabilities) ? {} : v.capabilities
				}
			])
		}
		return Object.fromEntries(out)
	}

	const cleanedDraft = $derived(cleaned(draft))

	const dirty = $derived(
		data != undefined &&
			JSON.stringify(cleanedDraft) !== JSON.stringify(cleaned(Object.entries(data.overrides)))
	)

	const errors = $derived.by(() => {
		const e: Record<string, string> = Object.create(null)
		for (const name of [...instanceRoles.map((r) => r.name), ...workspaceOnlyNames]) {
			// Left to the server, which knows the instance value.
			if (authoringUnknown(name)) continue
			const err = capabilitiesError(effectiveFor(name))
			if (err) e[name] = err
		}
		return e
	})

	// A role that starts authoring moves each member from half a seat to a full one. One that
	// falls back to an unknown instance value may, so it is confirmed too.
	const newAuthorRoles = $derived(
		[
			...instanceRoles.map((r) => r.name),
			...(data?.custom_roles_enabled ? workspaceOnlyNames : [])
		].filter((name) => {
			const before = data?.roles.find((r) => r.name === name)?.capabilities
			if (before && isAuthor(before)) return false
			return isAuthor(effectiveFor(name)) || authoringUnknown(name)
		})
	)

	function setPatch(name: string, capabilities: RoleOverride['capabilities']) {
		draft.set(name, { ...(draft.get(name) ?? {}), capabilities })
	}

	function setLabel(name: string, label: string) {
		draft.set(name, { ...(draft.get(name) ?? {}), label })
	}

	function resetRole(name: string) {
		draft.delete(name)
	}

	const newRoleError = $derived.by(() => {
		const name = newRoleName.trim()
		if (!name) return undefined
		if (instanceRoles.some((r) => r.name === name) || draft.has(name)) {
			return 'This role already exists'
		}
		return roleNameError(name)
	})

	function addRole() {
		const name = newRoleName.trim()
		if (!name || newRoleError) return
		draft.set(name, { capabilities: {} })
		open.set(name, true)
		newRoleName = ''
	}

	function onSaveClicked() {
		if (newAuthorRoles.length > 0) confirmSeatOpen = true
		else save()
	}

	async function save() {
		saving = true
		try {
			await WorkspaceService.updateWorkspaceRoleOverrides({
				workspace,
				requestBody: cleanedDraft
			})
			sendUserToast('Workspace roles saved')
			onSaved()
		} catch (e) {
			sendUserToast(`Failed to save workspace roles: ${e?.body ?? e}`, true)
		} finally {
			saving = false
		}
	}
</script>

{#snippet roleCard(name: string, role: EffectiveRole | undefined)}
	{@const { base, unknown } = baseFor(name)}
	{@const caps = applyCapabilitiesPatch(base, draft.get(name)?.capabilities)}
	{@const overridden = Object.hasOwn(cleanedDraft, name)}
	{@const frozen = !role && !data?.custom_roles_enabled}
	<div class="flex flex-col rounded-md border bg-surface-tertiary">
		<div class="flex items-center gap-2 p-2">
			<div class="flex flex-col w-40 shrink-0 min-w-0">
				<span class="text-xs font-semibold text-emphasis truncate">
					{draft.get(name)?.label?.trim() || role?.label || name}
				</span>
				<span class="text-2xs text-secondary font-mono truncate">{name}</span>
			</div>
			{#if role}
				<Badge color="gray" verySmall>Instance</Badge>
				{#if overridden}
					<Badge color="blue" verySmall>Overridden here</Badge>
				{/if}
			{:else if frozen}
				<Badge color="orange" verySmall>Read-only without a license</Badge>
			{:else}
				<Badge color="indigo" verySmall>This workspace only</Badge>
			{/if}
			<span class="text-2xs text-secondary truncate grow">
				{summary(caps)}
			</span>
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: open.get(name) ? ChevronDown : ChevronRight }}
				onclick={() => open.set(name, !open.get(name))}
			>
				Capabilities
			</Button>
			{#if role}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: RotateCcw }}
					disabled={!overridden}
					title="Use the instance definition of this role"
					onclick={() => resetRole(name)}
				>
					Reset role
				</Button>
			{:else}
				<Button
					variant="subtle"
					unifiedSize="sm"
					destructive
					iconOnly
					startIcon={{ icon: Trash2 }}
					aria-label="Remove role"
					title="Remove role"
					onclick={() => resetRole(name)}
				/>
			{/if}
		</div>
		{#if errors[name] && !open.get(name)}
			<span class="text-2xs text-red-600 dark:text-red-400 px-2 pb-2">{errors[name]}</span>
		{/if}
		{#if open.get(name)}
			<div class="border-t p-3 flex flex-col gap-3">
				<div class="flex items-center gap-2">
					<span class="text-xs font-semibold text-emphasis w-24">Label</span>
					<div class="w-60">
						<TextInput
							size="sm"
							value={draft.get(name)?.label ?? ''}
							inputProps={{
								placeholder: role?.label ?? name,
								disabled: frozen,
								'aria-label': 'Label in this workspace',
								oninput: (e) => setLabel(name, e.currentTarget.value)
							}}
						/>
					</div>
				</div>
				<RoleCapabilitiesEditor
					{base}
					unknownBase={unknown}
					patch={draft.get(name)?.capabilities ?? {}}
					baseName={role ? 'instance' : 'default'}
					setText={role ? 'Overridden' : 'Set'}
					inheritedText={role ? 'From instance' : 'Default'}
					disabled={frozen}
					onchange={(p) => setPatch(name, p)}
				/>
			</div>
		{/if}
	</div>
{/snippet}

<Section
	label="Workspace roles"
	collapsable={true}
	description="The roles members of this workspace can hold, as the instance defines them. Override what a role allows in this workspace; a field you do not override keeps following the instance. Workspace admins hold everything regardless of their role."
>
	{#snippet action()}
		<Button
			onclick={onSaveClicked}
			startIcon={{ icon: SaveIcon }}
			disabled={!dirty || saving || Object.keys(errors).length > 0}
			variant="accent"
		>
			Save workspace roles
		</Button>
	{/snippet}

	{#if !data}
		<span class="text-xs text-secondary">Loading roles...</span>
	{:else}
		<div class="flex flex-col gap-3">
			{#each instanceRoles as role (role.name)}
				{@render roleCard(role.name, role)}
			{/each}
			{#each workspaceOnlyNames as name (name)}
				{@render roleCard(name, undefined)}
			{/each}

			{#if data.custom_roles_enabled}
				<div class="flex items-start gap-2">
					<div class="flex flex-col gap-0.5 w-60">
						<TextInput
							size="sm"
							bind:value={newRoleName}
							error={newRoleError}
							inputProps={{
								placeholder: 'new role name',
								'aria-label': 'New role name',
								onkeydown: (e) => e.key === 'Enter' && addRole()
							}}
						/>
						{#if newRoleError}
							<span class="text-2xs text-red-600 dark:text-red-400">{newRoleError}</span>
						{/if}
					</div>
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: Plus }}
						disabled={!newRoleName.trim() || !!newRoleError}
						onclick={addRole}
					>
						Add workspace role
					</Button>
				</div>
				<span class="text-2xs text-secondary">
					A workspace role exists only here. Its unset capabilities default to the operator rights.
					One still held by a member, invite, auto-invite or instance group mapping cannot be
					removed.
				</span>
			{:else}
				<Alert
					type="info"
					size="xs"
					title="Adding roles requires a valid Windmill Enterprise Edition license"
				>
					This workspace can still override what the instance roles allow. Roles added here earlier
					keep applying but are read-only; they can still be removed.
				</Alert>
			{/if}
		</div>
	{/if}
</Section>

<ConfirmationModal
	open={confirmSeatOpen}
	title="Roles that now author consume a full seat"
	type="info"
	confirmationText="Save"
	onCanceled={() => (confirmSeatOpen = false)}
	onConfirmed={async () => {
		confirmSeatOpen = false
		await save()
	}}
>
	<div class="flex flex-col gap-2 text-sm">
		<span>
			With this change, members holding
			<b>{newAuthorRoles.join(', ')}</b> can write code or build flows or apps in this workspace, or
			may again if the instance definition allows it.
		</span>
		<span>
			Each of them then consumes a full seat instead of half a seat, which changes what this
			instance is billed. They can author wherever their folder permissions already let them write.
		</span>
	</div>
</ConfirmationModal>
