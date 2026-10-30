<script lang="ts">
	import { Button } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import { GranularAclService } from '$lib/gen'
	import { AlertTriangle, Check } from 'lucide-svelte'
	import { resource } from 'runed'
	import { setGrant } from './access'
	import {
		detectNeeds,
		type NeededItem,
		type Needs,
		type NeedsSource,
		type RunnableKind
	} from './detectNeeds'
	import { getPendingServiceAccount, type PendingServiceAccount } from './serviceAccounts.svelte'

	let {
		workspace,
		kind,
		itemPath,
		needsSource,
		account
	}: {
		workspace: string
		kind: RunnableKind
		itemPath?: string
		/** Read when the identity changes or the section mounts, so unsaved edits count. */
		needsSource?: () => NeedsSource | undefined
		/** `u/<username>` the item runs as; unset when it runs as whoever runs it. */
		account: string | undefined
	} = $props()

	type Report = {
		key: string
		needs: Needs
		reads?: Set<string> | 'all'
		accessError?: string
	}

	const operatingUser = useOperatingUser()
	const me = $derived(
		operatingUser.current?.username ? `u/${operatingUser.current.username}` : undefined
	)
	const isAdmin = $derived(!!operatingUser.current?.is_admin)
	const pending = $derived(getPendingServiceAccount(workspace, account))
	// Reading another principal's access is admin-only; anyone may read their own. A pending
	// account is marked from the grants planned for it, as it has no access yet.
	const mode = $derived(
		!account ? 'caller' : pending ? 'pending' : account === me || isAdmin ? 'checked' : 'unmarked'
	)
	const key = $derived(
		itemPath || needsSource ? `${workspace}:${mode}:${account ?? ''}` : undefined
	)

	let report: Report | undefined = $state()
	let grantErrors: Record<string, string> = $state({})
	let granting: string | undefined = $state()
	let confirming: NeededItem | undefined = $state()
	// Responses are not ordered: a slow answer for the previous identity must not land.
	let requestId = 0

	async function check(): Promise<void> {
		const id = ++requestId
		const forKey = key
		if (!forKey) return
		const principal = mode === 'checked' ? account : undefined
		// A saved run-as user that is disabled or deleted fails the access check; the list still
		// matters most then, so only the scan failing hides the section.
		const access = principal
			? GranularAclService.getPrincipalAccess({ workspace, principal }).then(
					(a) => ({
						reads: a.is_admin
							? ('all' as const)
							: new Set(a.items.map((i) => `${i.kind}:${i.path}`))
					}),
					(e: any) => ({ accessError: `${e?.body ?? e}` })
				)
			: Promise.resolve({})
		try {
			const [needs, result] = await Promise.all([
				detectNeeds(workspace, kind, itemPath ?? '', needsSource?.()),
				access
			])
			if (id === requestId) report = { key: forKey, needs, ...result }
		} catch {
			if (id === requestId) report = undefined
		}
	}

	resource(
		() => key,
		() => {
			grantErrors = {}
			return check()
		}
	)

	function plannedRead(account: PendingServiceAccount, item: NeededItem): boolean {
		return account.grants.some((g) =>
			g.kind === 'folder'
				? item.path.startsWith(`f/${g.path}/`)
				: g.kind === item.kind && g.path === item.path
		)
	}

	const shown = $derived(report && report.key === key ? report : undefined)
	const items = $derived(shown?.needs.items ?? [])
	const readable = $derived.by((): ((item: NeededItem) => boolean) | undefined => {
		const reads = shown?.reads
		if (mode === 'pending' && pending) return (i) => plannedRead(pending, i)
		if (mode === 'checked' && reads) {
			return (i) => reads === 'all' || reads.has(`${i.kind}:${i.path}`)
		}
		return undefined
	})

	async function grantRead(item: NeededItem): Promise<void> {
		if (!account) return
		const itemKey = `${item.kind}:${item.path}`
		granting = itemKey
		try {
			await setGrant(workspace, item.kind, item.path, account, 'read')
			delete grantErrors[itemKey]
		} catch (e) {
			grantErrors[itemKey] = `${e?.body ?? e}`
		}
		await check()
		granting = undefined
		confirming = undefined
	}
</script>

<!-- Portalled and raised: this section also renders inside the deploy drawer and its run-as
     confirmation dialog. -->
<ConfirmationModal
	open={!!confirming}
	title="Grant read access"
	confirmationText="Grant read to {account}"
	type="info"
	loading={!!granting}
	alwaysPortal
	zIndexClass="z-[10000]"
	onConfirmed={() => confirming && grantRead(confirming)}
	onCanceled={() => (confirming = undefined)}
>
	{#if confirming}
		<p class="text-xs text-primary">
			{account} gets read access to the {confirming.kind}
			<span class="font-mono">{confirming.path}</span>. The grant applies now, before you deploy.
		</p>
	{/if}
</ConfirmationModal>

{#if items.length}
	<div class="flex flex-col gap-1.5 text-2xs">
		<span class="text-secondary">This {kind} needs read access to:</span>
		<DataTable size="xs">
			<Head>
				<tr>
					<Cell head first>Path</Cell>
					<Cell head last={!readable}>Kind</Cell>
					{#if readable}
						<Cell head>{account}</Cell>
						<Cell head last actions />
					{/if}
				</tr>
			</Head>
			<tbody class="divide-y">
				{#each items as item (item.kind + ':' + item.path)}
					{@const itemKey = `${item.kind}:${item.path}`}
					{@const ok = readable?.(item)}
					<tr>
						<Cell first><span class="font-mono text-2xs">{item.path}</span></Cell>
						<Cell last={!readable}>{item.kind}</Cell>
						{#if readable}
							<Cell>
								{#if grantErrors[itemKey]}
									<span class="text-red-600 dark:text-red-400">{grantErrors[itemKey]}</span>
								{:else if ok}
									<span class="flex items-center gap-1.5">
										<Check size={12} class="text-green-600 dark:text-green-400" />
										Can read
									</span>
								{:else}
									<span class="flex items-center gap-1.5 text-yellow-600 dark:text-yellow-400">
										<AlertTriangle size={12} class="text-yellow-500" />
										{mode === 'pending' ? "Won't be able to read" : "Can't read"}
									</span>
								{/if}
							</Cell>
							<Cell last actions>
								{#if !ok && mode === 'checked' && isAdmin}
									<Button
										variant="default"
										unifiedSize="xs"
										disabled={!!granting}
										onClick={() => (confirming = item)}
									>
										Grant read to {account}
									</Button>
								{/if}
							</Cell>
						{/if}
					</tr>
				{/each}
			</tbody>
		</DataTable>
		{#if shown?.accessError}
			<span class="text-secondary">Couldn't check {account}'s access: {shown.accessError}</span>
		{/if}
		{#if mode === 'caller'}
			<span class="text-secondary">
				It runs as whoever runs it, so each of them needs read access to these.
			</span>
		{/if}
		{#if shown?.needs.others.length}
			<span class="text-secondary">
				Also uses {shown.needs.others.map((o) => `${o.kind} ${o.path}`).join(', ')}; access to these
				is set on the storage or data table itself.
			</span>
		{/if}
		{#if shown?.needs.errors.length}
			<span class="text-secondary">
				Some of it could not be read, so this list may be incomplete: {shown.needs.errors.join(
					'; '
				)}.
			</span>
		{/if}
		<span class="text-secondary">
			Only paths written out in the code are found; a path the code builds while it runs isn't.
		</span>
	</div>
{/if}
