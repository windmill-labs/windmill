<script lang="ts">
	import { WorkspaceService, type AclChange, type AclTarget, type DatatableAclInfo } from '$lib/gen'
	import { resource } from 'runed'
	import { sendUserToast } from '$lib/toast'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import Portal from '../Portal.svelte'
	import Select from '../select/Select.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import Cell from '../table/Cell.svelte'
	import { Trash2 } from 'lucide-svelte'
	import PgGrantBuilder from './PgGrantBuilder.svelte'
	import { grantScopeLabel, groupGrants, revokeScopeOf } from './aclScopes'
	import { ADMIN_DATATABLE_ROLE } from '../dbTypes'

	let {
		workspace,
		datatable,
		target,
		role,
		showOwner = true
	}: {
		workspace: string
		datatable: string
		/** Read and act as this role rather than the caller's default one. */
		role?: string
		/** Off where ownership is not the caller's to move — a data table's whole
		 * database, which Windmill owns. */
		showOwner?: boolean
		/** What owner and grants are read and written for. Schemas today; a table
		 * target is the same call with one more identifier. */
		target: AclTarget
	} = $props()

	const acl = resource(
		() => [workspace, datatable, JSON.stringify(target), role] as const,
		async ([ws, dt]) =>
			await WorkspaceService.getDatatableAcl({
				workspace: ws,
				datatableName: dt,
				kind: target.kind,
				schema: target.schema,
				table: target.kind === 'table' ? target.table : undefined,
				role
			})
	)

	// Nothing is written before its SQL has been shown: creating and especially
	// revoking access is not something to discover afterwards.
	let pending = $state<
		{ change: AclChange; statements: string[]; warnings: string[]; title: string } | undefined
	>(undefined)

	/** A revoke listed per object takes them all: say where one of them is
	 * managed on its own. */
	const pendingCoversObjects = $derived((pending?.change.objects?.length ?? 0) > 1)

	async function confirm(change: AclChange, title: string) {
		planning = true
		try {
			const plan = await WorkspaceService.planDatatableAcl({
				workspace,
				datatableName: datatable,
				requestBody: { target, change, role }
			})
			pending = { change, statements: plan.statements, warnings: plan.warnings, title }
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
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
				requestBody: { target, change: pending.change, role }
			})
			sendUserToast(pending.title)
			pending = undefined
			await acl.refetch()
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			applying = false
		}
	}

	const info: DatatableAclInfo | undefined = $derived(acl.current)
	const roleItems = $derived((info?.roles ?? []).map((r) => ({ value: r, label: r })))
	const grantRows = $derived(groupGrants(info?.grants ?? []))

	/** Handing it to a role the caller cannot run as: after this they can no
	 * longer change it back. */
	const pendingGivesItAway = $derived(
		pending?.change.type === 'set_owner' &&
			!!pending.change.role &&
			!(info?.usable_roles ?? []).includes(pending.change.role)
	)
	let planning = $state(false)
	let applying = $state(false)
</script>

{#if acl.error}
	<Alert type="error" title="Could not read access" size="xs">{String(acl.error)}</Alert>
{:else if !info}
	<span class="text-sm text-tertiary">Loading...</span>
{:else}
	<div class="flex flex-col gap-6">
		{#if showOwner}
			<section class="flex flex-col gap-2">
				<div class="flex flex-col gap-0.5">
					<span class="text-sm font-semibold text-primary">Owner</span>
					<span class="text-xs text-secondary">
						{target.kind === 'schema'
							? 'The role that owns the schema and everything already in it. Changing it also lets the new owner reach what the other roles create here later.'
							: 'The role that owns the table. Its owner may always read and write it, and is who ALTER and DROP answer to.'}
					</span>
				</div>
				<Select
					items={roleItems}
					clearable={false}
					disabled={planning || applying}
					class="w-64"
					bind:value={
						() => info.owner,
						(role) => {
							// The select shows what the database says; a pick is a request,
							// and only the applied change moves it.
							if (role && role !== info.owner) {
								confirm({ type: 'set_owner', role }, `Ownership transferred to ${role}`)
							}
						}
					}
				/>
				{#if !info.roles.includes(info.owner)}
					<span class="text-xs text-tertiary">
						Currently owned by <span class="font-mono">{info.owner}</span>, which is not one of this
						data table's roles.
					</span>
				{/if}
			</section>
		{/if}

		<section class="flex flex-col gap-2">
			<div class="flex flex-col gap-0.5">
				<span class="text-sm font-semibold text-primary">Grants</span>
				<span class="text-xs text-secondary">
					{target.kind === 'database'
						? 'What each role may do on the database itself — CREATE is the right to create schemas in it.'
						: 'What each role may do, beyond what it owns.'}
				</span>
			</div>
			<PgGrantBuilder
				{target}
				roles={info.roles}
				disabled={planning || applying || !info.can_manage}
				supportsMaintain={info.supports_maintain}
				dbname={info.dbname}
				onAdd={({ role, privileges, scope }) =>
					confirm(
						{ type: 'grant', role, privileges, scope },
						`Granted ${privileges.join(', ')} to ${role}`
					)}
			/>
			{#if grantRows.length === 0}
				<span class="text-xs text-tertiary">No grants yet.</span>
			{:else}
				<DataTable size="xs">
					<Head>
						<tr>
							<Cell head first>Role</Cell>
							<Cell head>Privileges</Cell>
							<Cell head>On</Cell>
							<Cell head last></Cell>
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each grantRows as grant (grant.grantee + grant.objects
								.map((o) => `${o.name}(${o.args ?? ''})`)
								.join() + (grant.future ?? ''))}
							<Row>
								<Cell first>{grant.grantee}</Cell>
								<Cell><span class="font-mono text-2xs">{grant.privileges.join(', ')}</span></Cell>
								<Cell>{grantScopeLabel(grant)}</Cell>
								<Cell last>
									{@const revokeScope = revokeScopeOf(grant)}
									<!-- Reading a target's access needs no ownership of it, so a row
										on one the caller does not own has nothing to offer. And what
										`admin` holds is what every role here connects through, so it
										is not the drawer's to take away wherever it appears. -->
									{#if info.roles.includes(grant.grantee) && revokeScope && info.can_manage && grant.grantee !== ADMIN_DATATABLE_ROLE}
										<Button
											unifiedSize="xs"
											variant="default"
											iconOnly
											startIcon={{ icon: Trash2 }}
											title="Revoke"
											disabled={planning || applying}
											on:click={() =>
												confirm(
													{
														type: 'revoke',
														role: grant.grantee,
														privileges: grant.privileges,
														scope: revokeScope,
														objects: grant.objects
													},
													`Revoked ${grant.privileges.join(', ')} from ${grant.grantee}`
												)}
										/>
									{/if}
								</Cell>
							</Row>
						{/each}
					</tbody>
				</DataTable>
			{/if}
		</section>
	</div>
{/if}

<Portal>
	<ConfirmationModal
		open={!!pending}
		title="Confirm running the following"
		confirmationText="Run"
		type="info"
		loading={applying}
		onConfirmed={apply}
		onCanceled={() => (pending = undefined)}
	>
		<div class="flex flex-col gap-3 min-w-0">
			{#if pendingGivesItAway}
				<Alert type="warning" title="You will lose access to this" size="xs">
					<span class="font-mono">{pending?.change.role}</span> is not a role you can run as, so once
					it owns this you can no longer change its access — only a member of that role can hand it back.
				</Alert>
			{/if}
			{#if pendingCoversObjects}
				<Alert type="info" title="This covers every listed object" size="xs">
					Permissions on a single table are managed in that table's own permissions drawer.
				</Alert>
			{/if}
			{#each pending?.warnings ?? [] as warning}
				<Alert type="warning" title="Warning" size="xs">{warning}</Alert>
			{/each}
			<span class="text-sm text-secondary">
				The following runs against <span class="font-mono">{datatable}</span> in a single transaction:
			</span>
			<pre class="overflow-auto text-xs bg-surface-secondary p-3 rounded select-all max-h-80"
				>{(pending?.statements ?? []).join('\n')}</pre
			>
		</div>
	</ConfirmationModal>
</Portal>
