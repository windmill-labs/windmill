<script lang="ts">
	import { Button } from '$lib/components/common'
	import CollapseLink from '$lib/components/CollapseLink.svelte'
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

	type Report = { key: string; needs: Needs; reads?: Set<string> | 'all' }

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
	let granting = $state(false)
	let toggled: { key: string; open: boolean } | undefined = $state()
	// Responses are not ordered: a slow answer for the previous identity must not land.
	let requestId = 0

	async function check(): Promise<void> {
		const id = ++requestId
		const forKey = key
		if (!forKey) return
		const principal = mode === 'checked' ? account : undefined
		try {
			const [needs, access] = await Promise.all([
				detectNeeds(workspace, kind, itemPath ?? '', needsSource?.()),
				principal
					? GranularAclService.getPrincipalAccess({ workspace, principal })
					: Promise.resolve(undefined)
			])
			const reads = access
				? access.is_admin
					? ('all' as const)
					: new Set(access.items.map((i) => `${i.kind}:${i.path}`))
				: undefined
			if (id === requestId) report = { key: forKey, needs, reads }
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
	const missing = $derived(readable ? items.filter((i) => !readable(i)) : [])
	const open = $derived(toggled && toggled.key === key ? toggled.open : missing.length > 0)

	function plural(n: number, word: string): string {
		return `${n} ${word}${n === 1 ? '' : 's'}`
	}
	const summary = $derived.by(() => {
		const resources = items.filter((i) => i.kind === 'resource').length
		const variables = items.length - resources
		const counts = [
			resources ? plural(resources, 'resource') : undefined,
			variables ? plural(variables, 'variable') : undefined
		]
			.filter(Boolean)
			.join(' and ')
		return readable && !missing.length
			? `Reads ${counts}, all readable by ${account}`
			: `Reads ${counts}`
	})

	async function grantRead(): Promise<void> {
		if (!account) return
		const principal = account
		granting = true
		const errors: Record<string, string> = {}
		for (const item of missing) {
			try {
				await setGrant(workspace, item.kind, item.path, principal, 'read')
			} catch (e) {
				errors[`${item.kind}:${item.path}`] = `${e?.body ?? e}`
			}
		}
		grantErrors = errors
		toggled = undefined
		await check()
		granting = false
	}
</script>

{#if items.length}
	<CollapseLink
		text={summary}
		class="mb-0 text-2xs font-normal text-secondary"
		bind:open={() => open, (o) => key && (toggled = { key, open: o })}
	>
		<div class="flex flex-col gap-1.5 text-2xs">
			<div
				class="grid {readable
					? 'grid-cols-[max-content_minmax(0,max-content)_max-content_minmax(0,1fr)]'
					: 'grid-cols-[minmax(0,max-content)_max-content]'} items-center gap-x-3 gap-y-0.5"
			>
				{#each items as item (item.kind + ':' + item.path)}
					{@const itemKey = `${item.kind}:${item.path}`}
					{@const ok = readable?.(item)}
					{#if readable}
						{#if ok}
							<Check size={12} class="text-green-600 dark:text-green-400" />
						{:else}
							<AlertTriangle size={12} class="text-yellow-500" />
						{/if}
					{/if}
					<span class="font-mono text-primary truncate">{item.path}</span>
					<span class="text-secondary">{item.kind}</span>
					{#if readable}
						{#if grantErrors[itemKey]}
							<span class="text-red-600 dark:text-red-400 truncate" title={grantErrors[itemKey]}>
								{grantErrors[itemKey]}
							</span>
						{:else if !ok}
							<span class="text-yellow-600 dark:text-yellow-400 truncate">
								{account}
								{mode === 'pending' ? "won't be able to read it" : "can't read it"}
							</span>
						{:else}
							<span></span>
						{/if}
					{/if}
				{/each}
			</div>
			{#if missing.length && mode === 'checked' && isAdmin}
				<div class="flex justify-end">
					<Button variant="default" unifiedSize="xs" loading={granting} onClick={grantRead}>
						Grant read
					</Button>
				</div>
			{/if}
			{#if mode === 'caller'}
				<span class="text-secondary">
					It runs as whoever runs it, so each of them needs read access to these.
				</span>
			{/if}
			{#if shown?.needs.others.length}
				<span class="text-secondary">
					Also uses {shown.needs.others.map((o) => `${o.kind} ${o.path}`).join(', ')}; access to
					these is set on the storage or data table itself.
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
	</CollapseLink>
{/if}
