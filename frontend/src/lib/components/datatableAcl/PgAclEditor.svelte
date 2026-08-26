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
	import { grantScopeLabel, type AclScope } from './aclScopes'

	let {
		workspace,
		datatable,
		target
	}: {
		workspace: string
		datatable: string
		/** What owner and grants are read and written for. Schemas today; a table
		 * target is the same call with one more identifier. */
		target: AclTarget
	} = $props()

	const acl = resource(
		() => [workspace, datatable, JSON.stringify(target)] as const,
		async ([ws, dt]) =>
			await WorkspaceService.getDatatableAcl({
				workspace: ws,
				datatableName: dt,
				kind: target.kind,
				schema: target.schema,
				table: target.kind === 'table' ? target.table : undefined
			})
	)

	// Nothing is written before its SQL has been shown: creating and especially
	// revoking access is not something to discover afterwards.
	let pending = $state<
		{ change: AclChange; statements: string[]; warnings: string[]; title: string } | undefined
	>(undefined)
	let planning = $state(false)
	let applying = $state(false)

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
				requestBody: { target, change: pending.change }
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
</script>

{#if acl.error}
	<Alert type="error" title="Could not read access" size="xs">{String(acl.error)}</Alert>
{:else if !info}
	<span class="text-sm text-tertiary">Loading...</span>
{:else}
	<div class="flex flex-col gap-6">
		<section class="flex flex-col gap-2">
			<div class="flex flex-col gap-0.5">
				<span class="text-sm font-semibold text-primary">Owner</span>
				<span class="text-xs text-secondary">
					The role that owns {target.kind === 'schema' ? 'the schema' : 'the table'} and everything already
					in it. Changing it also lets the new owner reach what the other roles create here later.
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

		<section class="flex flex-col gap-2">
			<div class="flex flex-col gap-0.5">
				<span class="text-sm font-semibold text-primary">Grants</span>
				<span class="text-xs text-secondary"> What each role may do, beyond what it owns. </span>
			</div>
			<PgGrantBuilder
				{target}
				roles={info.roles}
				disabled={planning || applying}
				onAdd={({ role, privileges, scope }) =>
					confirm(
						{ type: 'grant', role, privileges, scope },
						`Granted ${privileges.join(', ')} to ${role}`
					)}
			/>
			{#if info.grants.length === 0}
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
						{#each info.grants as grant (grant.grantee + (grant.object?.name ?? '') + (grant.future ?? ''))}
							<Row>
								<Cell first>{grant.grantee}</Cell>
								<Cell><span class="font-mono text-2xs">{grant.privileges.join(', ')}</span></Cell>
								<Cell>{grantScopeLabel(grant)}</Cell>
								<Cell last>
									{#if info.roles.includes(grant.grantee)}
										<Button
											size="xs"
											color="light"
											variant="border"
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
														scope: grant.future
															? (`future_${grant.future.toLowerCase()}` as AclScope)
															: 'target',
														object: grant.object
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
		<div class="flex flex-col gap-3">
			{#each pending?.warnings ?? [] as warning}
				<Alert type="warning" title="Warning" size="xs">{warning}</Alert>
			{/each}
			<span class="text-sm text-secondary">
				The following runs against <span class="font-mono">{datatable}</span> in a single transaction:
			</span>
			<pre
				class="whitespace-pre-wrap overflow-y-auto text-xs bg-surface-secondary p-3 rounded select-all max-h-80"
				>{(pending?.statements ?? []).join('\n')}</pre
			>
		</div>
	</ConfirmationModal>
</Portal>
