<script lang="ts">
	import { untrack } from 'svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { displayDate } from '$lib/utils'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import CollapseLink from '../CollapseLink.svelte'
	import TokenInspector from './TokenInspector.svelte'
	import { UserService, type TokenRisk, type TokenRiskCounts, type TruncatedToken } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import CreateToken from './CreateToken.svelte'
	import EditTokenScopesModal from './EditTokenScopesModal.svelte'
	import Button from '../common/button/Button.svelte'
	import Badge from '../common/badge/Badge.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import { Pen, Trash } from 'lucide-svelte'
	import { MCP_SESSION } from '../mcp/McpAccessPicker.svelte'
	import { tokenRiskSummary, TOKEN_RISK_INFO } from './tokenRisks'

	// --- Props ---
	interface Props {
		defaultNewTokenLabel?: string
		defaultNewTokenWorkspace?: string
		scopes?: string[]
		onTokenCreated: (token: string) => void
	}

	let { defaultNewTokenLabel, defaultNewTokenWorkspace, scopes, onTokenCreated }: Props = $props()

	// --- Local State ---
	let tokens = $state<TruncatedToken[]>([])
	let tokenPage = $state(1)
	let newTokenLabel = $state<string | undefined>(untrack(() => defaultNewTokenLabel))
	let editingToken = $state<
		| {
				prefix: string
				label: string | undefined
				scopes: string[] | undefined
				workspaceId: string | undefined
				expiration: string | undefined
				allowedIps: string[] | undefined
		  }
		| undefined
	>(undefined)
	let editModalOpen = $state(false)
	let testOpen = $state(false)

	$effect(() => {
		listTokens()
	})

	// Mirror of the canonical `is_user_token` in backend/windmill-common/src/auth.rs.
	// When updating this filter, also update that function and the SQL `WHERE`
	// mirror in `update_token_label` (backend/windmill-api-users/src/users.rs).
	function isUserToken(label: string | undefined): boolean {
		if (!label) return true
		return (
			label !== 'session' &&
			label !== 'guest_session' &&
			!label.toLowerCase().startsWith('ephemeral') &&
			label !== 'debugger-token' &&
			!label.startsWith('mcp-oauth-') &&
			!label.startsWith('embed_app:') &&
			!label.startsWith('sdk_app:') &&
			!label.startsWith('impersonation:') &&
			!label.startsWith('cli-login:') &&
			!label.startsWith('remote-deploy:')
		)
	}

	function daysUntilExpiration(expiration: string | undefined): number | null {
		if (!expiration) return null
		const today = new Date()
		today.setHours(0, 0, 0, 0)
		const exp = new Date(expiration)
		exp.setHours(0, 0, 0, 0)
		return Math.round((exp.getTime() - today.getTime()) / 86400000)
	}

	function expirationBadge(
		expiration: string | undefined,
		label: string | undefined
	): {
		color: 'red' | 'orange' | 'yellow' | 'gray'
		text: string
	} | null {
		if (!isUserToken(label)) return null
		const days = daysUntilExpiration(expiration)
		if (days === null) return null
		if (days < 0) return { color: 'red', text: 'Expired' }
		if (days === 0) return { color: 'red', text: 'Expires today' }
		if (days === 1) return { color: 'orange', text: 'Expires tomorrow' }
		if (days <= 7) return { color: 'orange', text: `Expires in ${days}d` }
		if (days <= 30) return { color: 'yellow', text: `Expires in ${days}d` }
		return null
	}

	let expiringSoonCount = $derived(
		tokens.filter((t) => {
			if (!isUserToken(t.label)) return false
			const days = daysUntilExpiration(t.expiration)
			return days !== null && days >= 0 && days <= 7
		}).length
	)

	// Over all the user's tokens, not only the page shown.
	let riskCounts: TokenRiskCounts | undefined = $state(undefined)
	let riskSummary = $derived(tokenRiskSummary(riskCounts))

	// The access column already shows a token's workspace and rights.
	function rowBadgeRisks(risks: TokenRisk[]): TokenRisk[] {
		return risks.filter((r) => r === 'never_expires' || r === 'unused')
	}

	function handleTokenCreated(token: string) {
		onTokenCreated(token)
		listTokens()
	}

	async function handleDeleteClick(tokenPrefix: string) {
		await UserService.deleteToken({ tokenPrefix })
		sendUserToast('Successfully deleted token')
		listTokens()
	}

	function shownScopes(scopes: string[] | null | undefined): string {
		return (scopes ?? []).filter((s) => s !== MCP_SESSION).join(', ')
	}

	function handleEditClick(
		tokenPrefix: string,
		tokenLabel: string | undefined,
		tokenScopes: string[] | undefined,
		tokenWorkspaceId: string | undefined,
		tokenExpiration: string | undefined,
		tokenAllowedIps: string[] | undefined
	) {
		editingToken = {
			prefix: tokenPrefix,
			label: tokenLabel,
			scopes: tokenScopes,
			workspaceId: tokenWorkspaceId,
			expiration: tokenExpiration,
			allowedIps: tokenAllowedIps
		}
		editModalOpen = true
	}

	export async function listTokens(): Promise<void> {
		;[tokens, riskCounts] = await Promise.all([
			UserService.listTokens({ excludeEphemeral: true, page: tokenPage, perPage: 100 }),
			UserService.getTokenRiskCounts()
		])
	}

	function handleNextPage() {
		tokenPage += 1
		listTokens()
	}

	function handlePreviousPage() {
		tokenPage -= 1
		listTokens()
	}
</script>

<div class="flex flex-col">
	<div class="text-xs text-secondary mb-2">
		Authenticate to the Windmill API with access tokens.
	</div>
	{#if riskSummary.length > 0}
		<div class="mb-2">
			<Alert type="warning" title="Some of your tokens need a review" size="xs">
				<ul class="list-disc pl-4">
					{#each riskSummary as line (line)}
						<li>{line}</li>
					{/each}
				</ul>
				A leaked token keeps working until it expires or is deleted. Delete tokens you no longer use,
				and give the ones you keep an expiration, the narrowest scopes they need and no admin rights
				unless they require them.
			</Alert>
		</div>
	{/if}
	{#if expiringSoonCount > 0}
		<div class="mb-2">
			<Alert
				type="warning"
				title="{expiringSoonCount} token{expiringSoonCount > 1 ? 's' : ''} expiring within 7 days"
				size="xs"
			/>
		</div>
	{/if}
	<CreateToken
		bind:newTokenLabel
		{defaultNewTokenWorkspace}
		{scopes}
		onTokenCreated={handleTokenCreated}
	/>
	<div class="overflow-auto grow min-h-64 max-h-2/3">
		<TableCustom>
			{#snippet headerRow()}
				<tr>
					<th>Prefix</th>
					<th>Label</th>
					<th>Expiration</th>
					<th>Access</th>
					<th>Scopes</th>
					<th></th>
				</tr>
			{/snippet}
			{#snippet body()}
				<tbody>
					{#if tokens && tokens.length > 0}
						{#each tokens as { token_prefix, expiration, label, scopes, workspace_id, read_only, allowed_ips, admin, super_admin, risks } (token_prefix)}
							{@const rowRisks = rowBadgeRisks(risks)}
							{@const badge = expirationBadge(expiration, label)}
							<tr>
								<td class="w-32 text-xs text-primary">{token_prefix}****</td>
								<td class="min-w-0 max-w-40 text-xs text-primary">
									<div class="truncate">{label ?? ''}</div>
									{#if rowRisks.length > 0}
										<div class="flex flex-wrap gap-1 mt-0.5">
											{#each rowRisks as risk (risk)}
												<span title={TOKEN_RISK_INFO[risk].tooltip}>
													<Badge color={TOKEN_RISK_INFO[risk].color} small>
														{TOKEN_RISK_INFO[risk].badge}
													</Badge>
												</span>
											{/each}
										</div>
									{/if}
								</td>
								<td class="w-40 whitespace-nowrap text-xs text-secondary">
									<div class="flex items-center gap-1.5">
										{displayDate(expiration ?? '')}
										{#if badge}
											<Badge color={badge.color} small>{badge.text}</Badge>
										{/if}
									</div>
								</td>
								<td class="whitespace-nowrap text-xs text-secondary">
									<div class="flex items-center gap-1.5">
										<span
											title={workspace_id
												? `Usable only in workspace ${workspace_id}`
												: 'Usable in every workspace you are a member of'}
										>
											{workspace_id ?? 'All workspaces'}
										</span>
										{#if super_admin}
											<Badge color="red" small>Superadmin</Badge>
										{:else if admin}
											<Badge color="indigo" small>Admin</Badge>
										{/if}
									</div>
								</td>
								<td
									class="min-w-0 max-w-48 truncate text-xs text-secondary"
									title={shownScopes(scopes)}
								>
									<div class="flex items-center gap-1.5 truncate">
										{#if read_only}
											<Badge color="blue" small>Read-only</Badge>
										{/if}
										{#if $enterpriseLicense && allowed_ips && allowed_ips.length > 0}
											<span title="Usable only from {allowed_ips.join(', ')}">
												<Badge color="gray" small>
													{allowed_ips.length === 1
														? allowed_ips[0]
														: `${allowed_ips.length} IP ranges`}
												</Badge>
											</span>
										{/if}
										<span class="truncate">{shownScopes(scopes)}</span>
									</div>
								</td>
								<td class="w-24 text-center">
									<div class="flex items-center justify-center gap-1">
										<Button
											variant="subtle"
											title="Edit token"
											on:click={() =>
												handleEditClick(
													token_prefix,
													label ?? undefined,
													scopes ?? undefined,
													workspace_id ?? undefined,
													expiration ?? undefined,
													allowed_ips ?? undefined
												)}
											size="xs"
											startIcon={{ icon: Pen }}
											iconOnly
										/>
										<Button
											variant="subtle"
											destructive
											on:click={() => handleDeleteClick(token_prefix)}
											size="xs"
											startIcon={{ icon: Trash }}
											iconOnly
										/>
									</div>
								</td>
							</tr>
						{/each}
					{:else if tokens && tokens.length === 0}
						<tr class="px-6">
							<td class="text-secondary italic text-2xs"> There are no tokens yet</td>
						</tr>
					{:else}
						<tr><td class="text-secondary text-xs">Loading...</td></tr>
					{/if}
				</tbody>
			{/snippet}
		</TableCustom>
		<div class="flex flex-row-reverse gap-2 w-full mt-2">
			{#if tokens?.length == 100}
				<Button variant="subtle" size="xs" on:click={handleNextPage}>Next</Button>
			{/if}
			{#if tokenPage > 1}
				<Button variant="subtle" size="xs" on:click={handlePreviousPage}>Previous</Button>
			{/if}
		</div>
	</div>
	<div class="mt-4">
		<CollapseLink text="Test a token" bind:open={testOpen}>
			<div class="text-xs text-secondary mb-2">
				See who a token acts as, what it is scoped to and whether it can call a route{$workspaceStore
					? ` in ${$workspaceStore}`
					: ''}.
			</div>
			<TokenInspector workspace={$workspaceStore} />
		</CollapseLink>
	</div>
</div>

<EditTokenScopesModal
	bind:open={editModalOpen}
	tokenPrefix={editingToken?.prefix}
	initialLabel={editingToken?.label}
	labelEditable={isUserToken(editingToken?.label)}
	initialScopes={editingToken?.scopes}
	tokenWorkspaceId={editingToken?.workspaceId}
	initialExpiration={editingToken?.expiration}
	initialAllowedIps={editingToken?.allowedIps}
	onSaved={listTokens}
/>
