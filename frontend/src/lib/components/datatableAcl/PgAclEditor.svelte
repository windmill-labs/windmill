<script lang="ts">
	import { WorkspaceService, type AclChange, type AclTarget, type DatatableAclInfo } from '$lib/gen'
	import { resource } from 'runed'
	import { Trash2 } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { Alert, Button } from '../common'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import Select from '../select/Select.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import Cell from '../table/Cell.svelte'
	import PgGrantBuilder from './PgGrantBuilder.svelte'
	import {
		ADMIN_ROLE,
		grantKey,
		grantScopeLabel,
		groupGrants,
		revocablePrivileges,
		revokeScopeOf,
		unreachableSources
	} from './aclScopes'

	let {
		workspace,
		datatable,
		target,
		onLoaded
	}: {
		workspace: string
		datatable: string
		/** What owner and grants are read and written for. */
		target: AclTarget
		/** Each read, with the target it was made for: it also lists what the target holds. */
		onLoaded?: (target: AclTarget, info: DatatableAclInfo) => void
	} = $props()

	const acl = resource(
		() => [workspace, datatable, target] as const,
		async ([ws, dt, t]) => {
			const loaded = await WorkspaceService.getDatatableAcl({
				workspace: ws,
				datatableName: dt,
				kind: t.kind,
				schema: t.schema,
				table: t.kind === 'table' ? t.table : undefined
			})
			onLoaded?.(t, loaded)
			return loaded
		}
	)

	// Nothing is written before its SQL has been shown, and the apply runs exactly that SQL: the
	// server plans again and refuses if the result differs.
	let pending = $state<
		{ change: AclChange; statements: string[]; warnings: string[]; title: string } | undefined
	>(undefined)
	let planning = $state(false)
	let applying = $state(false)

	const info: DatatableAclInfo | undefined = $derived(acl.current)
	const grantRows = $derived(groupGrants(info?.grants ?? []))
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

{#if acl.error}
	<Alert type="error" title="Could not read access" size="xs">{errorText(acl.error)}</Alert>
{:else if !info}
	<span class="text-xs text-secondary">Loading…</span>
{:else}
	<div class="flex flex-col gap-4">
		{#if !info.editable}
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
							? 'The role that owns the schema and everything already in it, except what belongs to an extension, which stays with the extension. Changing it also keeps the new owner in reach of what the other roles create here later.'
							: 'The role that owns the table. Its owner may always read and write it, and is who ALTER and DROP answer to.'}
					</span>
				</div>
				{#if info.editable}
					<Select
						items={ownerItems}
						disabled={planning || applying}
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
					/>
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
						? 'What each role may do on the database itself: CREATE is the right to create schemas in it.'
						: 'What each role may do here, beyond what it owns.'}
				</span>
			</div>
			{#if info.editable}
				<PgGrantBuilder
					{target}
					roles={info.roles}
					disabled={planning || applying}
					supportsMaintain={info.supports_maintain}
					dbname={info.dbname}
					onAdd={({ role, privileges, scope }) =>
						confirm(
							{ type: 'grant', role, privileges, scope },
							`Granted ${privileges.join(', ')} to ${role}`
						)}
				/>
			{/if}
			{#if grantRows.length === 0}
				<span class="text-xs text-secondary">No grants yet.</span>
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
						{#each grantRows as grant (grantKey(grant))}
							{@const revokeScope = revokeScopeOf(grant)}
							{@const revocable = revocablePrivileges(grant, target)}
							{@const unreachable = unreachableSources(grant)}
							<Row>
								<Cell first>{grant.grantee}</Cell>
								<Cell wrap
									><span class="font-mono text-2xs">{grant.privileges.join(', ')}</span></Cell
								>
								<Cell>
									{grantScopeLabel(grant)}
									{#if unreachable.length > 0}
										<span
											class="text-2xs text-secondary"
											title="Only this role can take the grant back: Postgres revokes a grant through the role that made it"
										>
											from {unreachable.join(', ')}
										</span>
									{/if}
								</Cell>
								<Cell last>
									<!-- What `admin` holds is what every role here connects through, so it is not
									this editor's to take away. -->
									{#if info.editable && revokeScope && revocable.length > 0 && info.roles.includes(grant.grantee) && grant.grantee !== ADMIN_ROLE}
										<Button
											unifiedSize="xs"
											variant="subtle"
											iconOnly
											startIcon={{ icon: Trash2 }}
											title="Revoke {revocable.join(', ')}"
											disabled={planning || applying}
											onClick={() =>
												confirm(
													{
														type: 'revoke',
														role: grant.grantee,
														privileges: revocable,
														scope: revokeScope,
														objects: grant.objects
													},
													`Revoked ${revocable.join(', ')} from ${grant.grantee}`
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
