<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert, Badge, Button } from '$lib/components/common'
	import { UserService, type TokenForReview, type TokenRisk, type TokenRiskCounts } from '$lib/gen'
	import { displayDate } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { Trash } from 'lucide-svelte'
	import { Debounced, watch } from 'runed'
	import {
		tokenRiskSummary,
		TOKEN_RISK_INFO,
		TOKEN_RISKS
	} from '$lib/components/settings/tokenRisks'
	import { MCP_SESSION } from '$lib/components/mcp/McpAccessPicker.svelte'
	import { enterpriseLicense } from '$lib/stores'

	const perPage = 100

	let tokens: TokenForReview[] = $state([])
	let counts: TokenRiskCounts | undefined = $state(undefined)
	let hasMore = $state(false)
	let loading = $state(false)
	let loadFailed = $state(false)
	let flagFilter: TokenRisk | 'any' | 'all' = $state('any')
	let search = $state('')
	const debouncedSearch = new Debounced(() => search.trim(), 300)
	let toDelete: TokenForReview | undefined = $state(undefined)
	let deleting = $state(false)
	// Drops a response that a newer filter or search has overtaken.
	let requestId = 0
	// The last row the server returned, which the shown list may have dropped as a repeat: a
	// cursor taken from the shown list would stall on a page of repeats.
	let cursor: TokenForReview | undefined = undefined

	async function loadCounts() {
		try {
			counts = await UserService.getTokenRiskCountsAsSuperAdmin()
			loadFailed = false
		} catch (e) {
			loadFailed = true
			sendUserToast(`Failed to count tokens: ${e}`, true)
		}
	}
	loadCounts()

	async function load(more: boolean) {
		const id = ++requestId
		const last = more ? cursor : undefined
		loading = true
		try {
			const page = await UserService.listTokensAsSuperAdmin({
				flag: flagFilter === 'any' ? undefined : flagFilter,
				search: debouncedSearch.current || undefined,
				perPage,
				afterLastUsedAt: last?.last_used_at,
				afterTokenPrefix: last?.token_prefix
			})
			if (id !== requestId) return
			if (more) {
				// A token used since the previous page moved behind the cursor and comes back.
				const seen = new Set(tokens.map((t) => `${t.email}:${t.token_prefix}`))
				tokens = [...tokens, ...page.filter((t) => !seen.has(`${t.email}:${t.token_prefix}`))]
			} else {
				tokens = page
			}
			if (page.length > 0) cursor = page[page.length - 1]
			hasMore = page.length === perPage
		} catch (e) {
			if (id !== requestId) return
			sendUserToast(`Failed to load tokens: ${e}`, true)
		} finally {
			if (id === requestId) loading = false
		}
	}

	watch([() => $enterpriseLicense, () => flagFilter, () => debouncedSearch.current], () => {
		if ($enterpriseLicense) load(false)
	})

	let riskSummary = $derived(tokenRiskSummary(counts))
	let flagItems = $derived([
		{ value: 'any', label: 'Any flag' },
		...TOKEN_RISKS.map((r) => ({
			value: r,
			label: counts ? `${TOKEN_RISK_INFO[r].badge} (${counts[r]})` : TOKEN_RISK_INFO[r].badge
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
			if (counts) {
				const next = { ...counts }
				for (const r of t.risks) next[r] = Math.max(0, next[r] - 1)
				counts = next
			}
			sendUserToast(`Deleted token ${t.token_prefix}**** of ${t.email}`)
			toDelete = undefined
		} catch (e) {
			sendUserToast(`Failed to delete token: ${e}`, true)
		} finally {
			deleting = false
		}
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
{:else if counts && !loadFailed}
	<div class="mb-4">
		<Alert type="success" size="xs" title="No token needs a review" />
	</div>
{/if}

{#if !$enterpriseLicense}
	<Alert type="info" size="xs" title="The token list requires Enterprise Edition">
		With an Enterprise license, this page lists every user's tokens, filters them by flag, owner
		or label, and deletes any of them.
	</Alert>
{:else}
	<div class="flex flex-row gap-2 items-center mb-2">
		<TextInput
			inputProps={{ placeholder: 'Search by email or label' }}
			bind:value={search}
			class="w-60"
		/>
		<Select items={flagItems} bind:value={flagFilter} class="w-52" />
	</div>

	<DataTable loading={loading && tokens.length === 0}>
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
			{#each tokens as token, i (`${token.email}:${token.token_prefix}`)}
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
							{#each token.risks as risk (risk)}
								<span title={TOKEN_RISK_INFO[risk].tooltip}>
									<Badge color={TOKEN_RISK_INFO[risk].color} small
										>{TOKEN_RISK_INFO[risk].badge}</Badge
									>
								</span>
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

	{#if hasMore}
		<div class="flex justify-center mt-2">
			<Button variant="default" unifiedSize="sm" {loading} onclick={() => load(true)}>
				Load more
			</Button>
		</div>
	{/if}
{/if}

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
