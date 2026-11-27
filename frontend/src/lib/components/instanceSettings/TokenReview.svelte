<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert, Badge, Button } from '$lib/components/common'
	import { UserService, type TokenForReview } from '$lib/gen'
	import { displayDate } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { Trash } from 'lucide-svelte'
	import {
		countTokenRisks,
		tokenRisks,
		tokenRiskSummary,
		TOKEN_RISK_INFO,
		TOKEN_RISKS,
		type TokenRisk
	} from '$lib/components/settings/tokenRisks'
	import { MCP_SESSION } from '$lib/components/mcp/McpAccessPicker.svelte'

	const perPage = 1000

	let tokens: TokenForReview[] = $state([])
	let loading = $state(false)
	let loadFailed = $state(false)
	let flagFilter: TokenRisk | 'any' | 'all' = $state('any')
	let filter = $state('')
	let toDelete: TokenForReview | undefined = $state(undefined)
	let deleting = $state(false)

	// The listing is ordered by last use, which moves while pages are fetched: a token used
	// in between shows up on two pages, so rows already seen are dropped.
	async function loadAll() {
		loading = true
		try {
			const seen = new Set<string>()
			const all: TokenForReview[] = []
			for (let page = 1; ; page++) {
				const res = await UserService.listTokensAsSuperAdmin({ page, perPage })
				for (const t of res) {
					const key = `${t.email}:${t.token_prefix}`
					if (!seen.has(key)) {
						seen.add(key)
						all.push(t)
					}
				}
				if (res.length < perPage) break
			}
			tokens = all
			loadFailed = false
		} catch (e) {
			loadFailed = true
			sendUserToast(`Failed to load tokens: ${e}`, true)
		} finally {
			loading = false
		}
	}
	loadAll()

	let rows = $derived.by(() => {
		const now = new Date()
		const q = filter.trim().toLowerCase()
		return tokens
			.map((token) => ({ token, risks: tokenRisks(token, now) }))
			.filter(
				({ token, risks }) =>
					(flagFilter === 'all' ||
						(flagFilter === 'any' ? risks.length > 0 : risks.includes(flagFilter))) &&
					(!q ||
						(token.email ?? '').toLowerCase().includes(q) ||
						(token.label ?? '').toLowerCase().includes(q))
			)
	})
	let riskCounts = $derived(countTokenRisks(tokens))
	let riskSummary = $derived(tokenRiskSummary(riskCounts))
	let flagItems = $derived([
		{ value: 'any', label: 'Any flag' },
		...TOKEN_RISKS.map((r) => ({
			value: r,
			label: `${TOKEN_RISK_INFO[r].badge} (${riskCounts[r]})`
		})),
		{ value: 'all', label: 'All tokens' }
	])

	function shownScopes(scopes: string[] | undefined): string {
		return (scopes ?? []).filter((s) => s !== MCP_SESSION).join(', ')
	}

	async function confirmDelete() {
		const t = toDelete
		if (!t?.email) return
		deleting = true
		try {
			await UserService.deleteTokenAsSuperAdmin({ email: t.email, tokenPrefix: t.token_prefix })
			tokens = tokens.filter((x) => !(x.email === t.email && x.token_prefix === t.token_prefix))
			sendUserToast(`Deleted token ${t.token_prefix}**** of ${t.email}`)
			toDelete = undefined
		} catch (e) {
			sendUserToast(`Failed to delete token: ${e}`, true)
		} finally {
			deleting = false
		}
	}

	function riskBadges(risks: TokenRisk[]) {
		return risks.map((r) => ({ risk: r, ...TOKEN_RISK_INFO[r] }))
	}
</script>

<SettingsPageHeader
	title="Token review"
	description="API tokens of every user on the instance, least recently used first. Tokens the server expires or renews on its own (login sessions, job, app, MCP and impersonation tokens) are left out."
/>

{#if riskSummary.length > 0}
	<div class="mb-4">
		<Alert type="warning" size="xs" title="Tokens to review across the instance">
			<ul class="list-disc pl-4">
				{#each riskSummary as line (line)}
					<li>{line}</li>
				{/each}
			</ul>
		</Alert>
	</div>
{:else if !loading && !loadFailed}
	<div class="mb-4">
		<Alert type="success" size="xs" title="No token needs a review" />
	</div>
{/if}

<div class="flex flex-row gap-2 items-center mb-2">
	<TextInput
		inputProps={{ placeholder: 'Search by email or label' }}
		bind:value={filter}
		class="w-60"
	/>
	<Select items={flagItems} bind:value={flagFilter} class="w-52" />
</div>

<DataTable {loading}>
	<Head>
		<tr>
			<Cell head first>Owner</Cell>
			<Cell head>Token</Cell>
			<Cell head>Flags</Cell>
			<Cell head>Workspace</Cell>
			<Cell head>Scopes</Cell>
			<Cell head>Last used</Cell>
			<Cell head>Expiration</Cell>
			<Cell head last></Cell>
		</tr>
	</Head>
	<tbody>
		{#each rows as { token, risks }, i (`${token.email}:${token.token_prefix}`)}
			<tr class={i % 2 === 0 ? 'bg-surface-tertiary' : 'bg-surface'}>
				<Cell first><span class="text-xs">{token.email ?? '-'}</span></Cell>
				<Cell>
					<div class="flex flex-col">
						<span class="font-mono text-xs">{token.token_prefix}****</span>
						<span class="text-2xs text-secondary truncate max-w-40" title={token.label ?? ''}
							>{token.label ?? ''}</span
						>
					</div>
				</Cell>
				<Cell>
					<div class="flex flex-wrap gap-1">
						{#if token.read_only}
							<Badge color="blue" small>Read-only</Badge>
						{/if}
						{#each riskBadges(risks) as b (b.risk)}
							<span title={b.tooltip}><Badge color={b.color} small>{b.badge}</Badge></span>
						{/each}
					</div>
				</Cell>
				<Cell><span class="text-xs">{token.workspace_id ?? 'All'}</span></Cell>
				<Cell>
					<span class="text-xs truncate max-w-48 block" title={shownScopes(token.scopes)}
						>{shownScopes(token.scopes) || '-'}</span
					>
				</Cell>
				<Cell><span class="whitespace-nowrap text-xs">{displayDate(token.last_used_at)}</span></Cell
				>
				<Cell>
					<span class="whitespace-nowrap text-xs"
						>{token.expiration ? displayDate(token.expiration) : 'Never'}</span
					>
				</Cell>
				<Cell last>
					<Button
						variant="subtle"
						destructive
						unifiedSize="xs"
						title="Delete token"
						startIcon={{ icon: Trash }}
						iconOnly
						disabled={!token.email}
						onclick={() => (toDelete = token)}
					/>
				</Cell>
			</tr>
		{:else}
			{#if !loading}
				<tr>
					<Cell first last colspan={8}>
						<span class="text-secondary italic text-xs">
							{flagFilter === 'all' ? 'No tokens' : 'No token has this flag'}
						</span>
					</Cell>
				</tr>
			{/if}
		{/each}
	</tbody>
</DataTable>

<ConfirmationModal
	open={toDelete !== undefined}
	title="Delete token"
	confirmationText="Delete"
	loading={deleting}
	onConfirmed={confirmDelete}
	onCanceled={() => (toDelete = undefined)}
>
	The token {toDelete?.token_prefix}**** {toDelete?.label ? `(${toDelete.label}) ` : ''}of
	{toDelete?.email} stops working immediately. Anything authenticating with it will fail.
</ConfirmationModal>
