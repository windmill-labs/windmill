<script lang="ts">
	import { WorkspaceService, type AclChange, type AclTarget, type DatatableAclInfo } from '$lib/gen'
	import { resource } from 'runed'
	import type { Snippet } from 'svelte'
	import { Tooltip } from '../meltComponents'
	import { KeyRound, Trash2 } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { Alert, Button } from '../common'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import Select from '../select/Select.svelte'
	import PgGrantBuilder from './PgGrantBuilder.svelte'
	import Badge from '../common/badge/Badge.svelte'
	import {
		ADMIN_ROLE,
		blockingSources,
		grantKey,
		grantCoverage,
		groupGrants,
		revocablePrivileges,
		revokeScopeOf,
		uncoveredCreators,
		type AclScope,
		type GroupedGrant
	} from './aclScopes'

	let {
		workspace,
		datatable,
		target,
		disabledReason,
		manageRoles
	}: {
		workspace: string
		datatable: string
		/** What owner and grants are read and written for. */
		target: AclTarget
		/** Shows owner and grants without letting them change, with this reason on hover. */
		disabledReason?: string
		/** Offered at the bottom of every role picker. `blocker` keeps the entry, disabled, with why. */
		manageRoles?: { open: () => void; blocker?: string }
	} = $props()

	const acl = resource(
		() => [workspace, datatable, target] as const,
		async ([ws, dt, t]) =>
			await WorkspaceService.getDatatableAcl({
				workspace: ws,
				datatableName: dt,
				kind: t.kind,
				schema: t.kind === 'database' ? undefined : t.schema,
				table: t.kind === 'table' ? t.table : undefined
			})
	)

	/** Re-read owner, grants and the roles they can name, e.g. after the data table's roles changed. */
	export function refresh() {
		acl.refetch()
	}

	// Nothing is written before its SQL has been shown, and the apply runs exactly that SQL: the
	// server plans again and refuses if the result differs.
	let pending = $state<
		{ change: AclChange; statements: string[]; warnings: string[]; title: string } | undefined
	>(undefined)
	let planning = $state(false)
	let applying = $state(false)

	const info: DatatableAclInfo | undefined = $derived(acl.current)
	const controlsDisabled = $derived(planning || applying || !!disabledReason)
	const grantRows = $derived(groupGrants(info?.grants ?? []))
	// Several tags of one kind widen the list; tags of both kinds narrow it to grants matching each.
	let roleFilters = $state<string[]>([])
	let privilegeFilters = $state<string[]>([])
	// A selected tag stays offered after its last grant goes, so it can still be cleared.
	const roleChips = $derived(
		[...new Set([...grantRows.map((g) => g.grantee), ...roleFilters])].sort()
	)
	const privilegeChips = $derived(
		[...new Set([...grantRows.flatMap((g) => g.privileges), ...privilegeFilters])].sort()
	)
	const shownGrantRows = $derived(
		grantRows.filter(
			(g) =>
				(roleFilters.length === 0 || roleFilters.includes(g.grantee)) &&
				(privilegeFilters.length === 0 || g.privileges.some((p) => privilegeFilters.includes(p)))
		)
	)
	const toggled = (list: string[], value: string) =>
		list.includes(value) ? list.filter((v) => v !== value) : [...list, value]
	const ownerItems = $derived(
		info
			? (info.roles.includes(info.owner) ? info.roles : [info.owner, ...info.roles]).map((r) => ({
					value: r,
					label: r
				}))
			: []
	)
	/** A revoke listed per object takes them all: say that it does. */
	const pendingCoversObjects = $derived(
		pending?.change.type === 'revoke' && (pending.change.objects?.length ?? 0) > 1
	)

	/** Why a row's delete button is disabled, or undefined when it can revoke. */
	function unrevocableReason(
		grant: GroupedGrant,
		revokeScope: AclScope | undefined,
		revocable: string[],
		blocked: string[]
	): string | undefined {
		// What `admin` holds is what every role here connects through.
		if (grant.grantee === ADMIN_ROLE)
			return 'admin is the connection every role reaches this data table through, so its grants are not changed here'
		if (!info?.roles.includes(grant.grantee))
			return `Only grants to this data table's roles are revoked here, and ${grant.grantee} is not one`
		if (!revokeScope) return 'Grants on types are read only here'
		if (revocable.length === 0)
			return grant.future
				? 'Default privileges set database-wide are not revoked from here'
				: "CONNECT follows the data table's roles and TEMPORARY is left as Postgres sets it, so nothing here can be revoked"
		if (blocked.length > 0)
			return `Only ${blocked.join(', ')} can revoke this: Postgres takes a grant back through the role that made it`
		return undefined
	}

	function errorText(e: any): string {
		return e?.body ?? e?.message ?? String(e)
	}

	async function confirm(change: AclChange, title: string) {
		planning = true
		try {
			const plan = await WorkspaceService.planDatatableAcl({
				workspace,
				datatableName: datatable,
				requestBody: { target, change }
			})
			pending = { change, statements: plan.statements, warnings: plan.warnings, title }
		} catch (e) {
			sendUserToast(errorText(e), true)
		} finally {
			planning = false
		}
	}

	async function apply() {
		if (!pending) return
		applying = true
		try {
			await WorkspaceService.applyDatatableAcl({
				workspace,
				datatableName: datatable,
				requestBody: { target, change: pending.change, statements: pending.statements }
			})
			sendUserToast(pending.title)
			pending = undefined
			await acl.refetch()
		} catch (e) {
			sendUserToast(errorText(e), true)
		} finally {
			applying = false
		}
	}
</script>

<!-- A disabled control receives no hover, so the tooltip's trigger is a wrapper the hover falls
     through to. -->
{#snippet withDisabledReason(content: Snippet, widthClass: string)}
	{#if disabledReason}
		<Tooltip class="block {widthClass}" anchor="cursor">
			<div class="pointer-events-none">{@render content()}</div>
			{#snippet text()}{disabledReason}{/snippet}
		</Tooltip>
	{:else}
		{@render content()}
	{/if}
{/snippet}

<!-- The home page's filter chips, any number of them selected at once. -->
{#snippet filterChip(label: string, selected: boolean, toggle: () => void)}
	<Badge color="transparent" clickable {selected} onclick={toggle}>
		{label}
		{#if selected}&cross;{/if}
	</Badge>
{/snippet}

<!-- Styled like the picker's tags in the row that creates a grant. -->
{#snippet tag(text: string, strong = false)}
	<span
		class="inline-flex items-center min-h-6 px-2 border bg-surface rounded-full text-emphasis {strong
			? 'font-semibold'
			: ''}"
	>
		{text}
	</span>
{/snippet}

{#snippet manageRolesEntry({ close }: { close: () => void })}
	<div class="flex items-center gap-1 border-t p-1">
		<Button
			unifiedSize="sm"
			variant="subtle"
			startIcon={{ icon: KeyRound }}
			disabled={!!manageRoles?.blocker}
			btnClasses="w-full justify-start"
			wrapperClasses="grow"
			onClick={() => {
				close()
				manageRoles?.open()
			}}
		>
			Manage roles
		</Button>
		{#if manageRoles?.blocker}
			<span class="pr-2 flex">
				<Tooltip>
					{#snippet text()}{manageRoles?.blocker}{/snippet}
				</Tooltip>
			</span>
		{/if}
	</div>
{/snippet}

{#if acl.error}
	<Alert type="error" title="Could not read access" size="xs">{errorText(acl.error)}</Alert>
{:else if !info}
	<span class="text-xs text-secondary">Loading…</span>
{:else}
	<div class="flex flex-col gap-4">
		{#if info.clone}
			<span class="text-xs text-secondary">
				Read only: this data table is a clone, and its owners and grants stay as they were copied
				from the data table it was cloned from.
			</span>
		{:else if !info.editable}
			<span class="text-xs text-secondary">
				Read only: access is changed by the admins of the workspace that governs this data table, on
				Windmill Enterprise Edition.
			</span>
		{/if}

		{#if target.kind !== 'database'}
			<section class="flex flex-col gap-2">
				<div class="flex flex-col gap-0.5">
					<span class="text-xs font-semibold text-emphasis">Owner</span>
					<span class="text-xs text-secondary">
						{target.kind === 'schema'
							? 'The role that owns the schema and everything already in it, except what belongs to an extension, which stays with the extension. Changing it also keeps the new owner in reach of what the current roles create here from then on; a role added afterwards is not covered.'
							: 'The role that owns the table. Its owner may always read and write it, and is who ALTER and DROP answer to.'}
					</span>
				</div>
				{#if info.editable}
					{#snippet ownerSelect()}
						<Select
							items={ownerItems}
							disabled={controlsDisabled}
							size="sm"
							class="w-64"
							bind:value={
								() => info.owner,
								(role) => {
									// The select shows what the database says; a pick is a request, and only the
									// applied change moves it.
									if (role && role !== info.owner) {
										confirm({ type: 'set_owner', role }, `Ownership transferred to ${role}`)
									}
								}
							}
							bottomSnippet={manageRoles ? manageRolesEntry : undefined}
						/>
					{/snippet}
					{@render withDisabledReason(ownerSelect, 'w-64')}
				{:else}
					<span class="font-mono text-xs">{info.owner}</span>
				{/if}
			</section>
		{/if}

		<section class="flex flex-col gap-2">
			<div class="flex flex-col gap-0.5">
				<span class="text-xs font-semibold text-emphasis">Grants</span>
				<span class="text-xs text-secondary">
					{target.kind === 'database'
						? 'What each role may do on the database itself — CREATE is the right to create schemas in it — and what default privileges set database-wide give it on what is created later, in every schema. No schema can take those back.'
						: 'What each role may do here, beyond what it owns.'}
				</span>
			</div>
			<!-- A set filter keeps its chips, or a revoke that leaves one grant would strand it. -->
			{#if grantRows.length > 1 || roleFilters.length > 0 || privilegeFilters.length > 0}
				<div class="flex flex-wrap items-center gap-2">
					{#each roleChips as role (`role:${role}`)}
						{@render filterChip(role, roleFilters.includes(role), () => {
							roleFilters = toggled(roleFilters, role)
						})}
					{/each}
					{#each privilegeChips as privilege (`privilege:${privilege}`)}
						{@render filterChip(privilege, privilegeFilters.includes(privilege), () => {
							privilegeFilters = toggled(privilegeFilters, privilege)
						})}
					{/each}
				</div>
			{/if}
			<div class="flex flex-col border rounded-md divide-y">
				{#each shownGrantRows as grant (grantKey(grant))}
					{@const revokeScope = revokeScopeOf(grant)}
					{@const revocable = revocablePrivileges(grant, target)}
					{@const blocked = blockingSources(grant, revocable)}
					{@const uncovered = uncoveredCreators(grant, info.roles)}
					{@const coverage = grantCoverage(grant, target)}
					{@const unrevocable = unrevocableReason(grant, revokeScope, revocable, blocked)}
					<div class="flex items-center gap-2 px-3 py-2 min-h-12">
						<div class="flex flex-col gap-0.5 grow min-w-0">
							<div class="flex flex-wrap items-center gap-x-1.5 gap-y-1 text-xs">
								{@render tag(grant.grantee, true)}
								{#each grant.privileges as privilege (privilege)}
									{@render tag(privilege)}
								{/each}
							</div>
							{#if coverage || blocked.length > 0 || uncovered.length > 0}
								<span class="text-2xs text-secondary break-words">
									{[
										coverage && `on ${coverage}`,
										blocked.length > 0 && `granted by ${blocked.join(', ')}`,
										uncovered.length > 0 &&
											`not for what ${uncovered.join(', ')} ${uncovered.length === 1 ? 'creates' : 'create'}: a default privilege only covers the roles it was granted for`
									]
										.filter(Boolean)
										.join(' · ')}
								</span>
							{/if}
						</div>
						{#if info.editable}
							{#snippet revokeButton()}
								<Button
									unifiedSize="xs"
									variant="subtle"
									iconOnly
									startIcon={{ icon: Trash2 }}
									title={unrevocable ?? `Revoke ${revocable.join(', ')}`}
									disabled={controlsDisabled || !!unrevocable}
									onClick={() => {
										if (!revokeScope) return
										confirm(
											{
												type: 'revoke',
												role: grant.grantee,
												privileges: revocable,
												scope: revokeScope,
												objects: grant.objects
											},
											`Revoked ${revocable.join(', ')} from ${grant.grantee}`
										)
									}}
								/>
							{/snippet}
							{#if unrevocable && !disabledReason}
								<Tooltip>
									<div class="pointer-events-none">{@render revokeButton()}</div>
									{#snippet text()}{unrevocable}{/snippet}
								</Tooltip>
							{:else}
								{@render withDisabledReason(revokeButton, 'w-fit')}
							{/if}
						{/if}
					</div>
				{:else}
					<span class="flex items-center px-3 py-2 min-h-12 text-xs text-secondary">
						{grantRows.length === 0 ? 'No grants yet.' : 'No grant matches the filters.'}
					</span>
				{/each}
				{#if info.editable}
					<!-- The grant to create: a row like the others, set apart until it exists. -->
					<div class="px-3 py-2 bg-surface-secondary border-dashed rounded-b-md">
						{#snippet grantBuilder()}
							<PgGrantBuilder
								{target}
								roles={info.roles}
								disabled={controlsDisabled}
								manageRoles={manageRoles ? manageRolesEntry : undefined}
								supportsMaintain={info.supports_maintain}
								onAdd={({ role, privileges, scope }) =>
									confirm(
										{ type: 'grant', role, privileges, scope },
										`Granted ${privileges.join(', ')} to ${role}`
									)}
							/>
						{/snippet}
						{@render withDisabledReason(grantBuilder, 'w-full')}
					</div>
				{/if}
			</div>
		</section>
	</div>
{/if}

<ConfirmationModal
	open={!!pending}
	title="Run the following?"
	confirmationText="Run"
	type="info"
	alwaysPortal
	loading={applying}
	onConfirmed={apply}
	onCanceled={() => (pending = undefined)}
>
	<div class="flex flex-col gap-3 min-w-0">
		{#if pendingCoversObjects}
			<Alert type="info" title="This covers every listed object" size="xs">
				The same privileges on several objects read as one row, and are revoked together.
			</Alert>
		{/if}
		{#each pending?.warnings ?? [] as warning (warning)}
			<Alert type="warning" title="Warning" size="xs">{warning}</Alert>
		{/each}
		<span class="text-sm text-secondary">
			Runs against <span class="font-mono">{datatable}</span> in a single transaction:
		</span>
		<pre class="overflow-auto text-xs bg-surface-secondary p-3 rounded select-all max-h-80"
			>{(pending?.statements ?? []).join(';\n')};</pre
		>
	</div>
</ConfirmationModal>
