<script lang="ts">
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import { GranularAclService } from '$lib/gen'
	import { AlertTriangle } from 'lucide-svelte'
	import { resource } from 'runed'
	import { setGrant } from './access'
	import { detectNeeds, type NeededItem, type NeedsSource, type RunnableKind } from './detectNeeds'
	import { useServiceAccounts } from './serviceAccounts.svelte'

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
		/** Read when the account changes or the note mounts, so unsaved edits count. */
		needsSource?: () => NeedsSource | undefined
		/** `u/<username>` the item runs as. */
		account: string | undefined
	} = $props()

	type Report = { account: string; items: NeededItem[]; missing: NeededItem[] }

	const operatingUser = useOperatingUser()
	const serviceAccounts = useServiceAccounts(() => workspace)

	// Reading another principal's access is admin-only, and a pending account gets what the item
	// reads from the form that made it.
	const checked = $derived(
		!!operatingUser.current?.is_admin &&
			!!account &&
			(!!itemPath || !!needsSource) &&
			serviceAccounts.current.some((a) => !a.pending && `u/${a.username}` === account)
			? account
			: undefined
	)

	let report: Report | undefined = $state()
	let grantErrors: Record<string, string> = $state({})
	let granting = $state(false)
	// Responses are not ordered: a slow answer for the previously picked account must not land.
	let requestId = 0

	async function check(principal: string): Promise<void> {
		const id = ++requestId
		try {
			const [needs, access] = await Promise.all([
				detectNeeds(workspace, kind, itemPath ?? '', needsSource?.()),
				GranularAclService.getPrincipalAccess({ workspace, principal })
			])
			const reads = new Set(access.items.map((i) => `${i.kind}:${i.path}`))
			const missing = access.is_admin
				? []
				: needs.items.filter((i) => !reads.has(`${i.kind}:${i.path}`))
			if (id === requestId) report = { account: principal, items: needs.items, missing }
		} catch {
			if (id === requestId) report = undefined
		}
	}

	resource(
		() => checked,
		(principal) => {
			grantErrors = {}
			return principal ? check(principal) : Promise.resolve()
		}
	)

	const shown = $derived(report && report.account === checked ? report : undefined)

	async function grantRead(): Promise<void> {
		if (!shown) return
		const { account: principal, missing } = shown
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
		await check(principal)
		granting = false
	}
</script>

{#if shown?.missing.length}
	{@const n = shown.missing.length}
	<div class="flex flex-col gap-1.5 text-2xs">
		<div class="flex items-center gap-1.5 text-yellow-600 dark:text-yellow-400">
			<AlertTriangle size={14} class="shrink-0 text-yellow-500" />
			<span>
				{shown.account} can't read {n}
				{n === 1 ? 'thing' : 'things'} this {kind} reads
			</span>
			<Tooltip small>A path the code builds while it runs can't be found.</Tooltip>
		</div>
		<div
			class="grid grid-cols-[minmax(0,max-content)_max-content_minmax(0,1fr)] gap-x-3 gap-y-0.5 pl-5"
		>
			{#each shown.missing as item (item.kind + ':' + item.path)}
				{@const error = grantErrors[`${item.kind}:${item.path}`]}
				<span class="font-mono text-primary truncate">{item.path}</span>
				<span class="text-secondary">{item.kind}</span>
				<span class="text-red-600 dark:text-red-400 truncate" title={error}>{error ?? ''}</span>
			{/each}
		</div>
		<div class="flex justify-end">
			<Button variant="default" unifiedSize="xs" loading={granting} onClick={grantRead}>
				Grant read
			</Button>
		</div>
	</div>
{:else if shown?.items.length}
	{@const n = shown.items.length}
	<span class="text-2xs text-secondary">
		{shown.account} reads the {n === 1 ? shown.items[0].kind : `${n} resources and variables`} this {kind}
		uses.
	</span>
{/if}
