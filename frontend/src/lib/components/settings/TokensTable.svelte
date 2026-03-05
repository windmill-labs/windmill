<script lang="ts">
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { displayDate } from '$lib/utils'
	import { UserService, type TruncatedToken } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import CreateToken from './CreateToken.svelte'
	import Button from '../common/button/Button.svelte'
	import Badge from '../common/badge/Badge.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import { Trash } from 'lucide-svelte'

	// --- Props ---
	interface Props {
		showMcpMode?: boolean
		openWithMcpMode?: boolean
		defaultNewTokenLabel?: string
		defaultNewTokenWorkspace?: string
		scopes?: string[]
		onTokenCreated: (token: string) => void
	}

	let {
		showMcpMode = false,
		openWithMcpMode = false,
		defaultNewTokenLabel,
		defaultNewTokenWorkspace,
		scopes,
		onTokenCreated
	}: Props = $props()

	// --- Local State ---
	let tokens = $state<TruncatedToken[]>([])
	let tokenPage = $state(1)
	let newTokenLabel = $state<string | undefined>(defaultNewTokenLabel)

	$effect(() => {
		listTokens()
	})

	function isUserToken(label: string | undefined): boolean {
		if (!label) return true
		return label !== 'session' && !label.toLowerCase().startsWith('ephemeral')
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

	function handleTokenCreated(token: string) {
		onTokenCreated(token)
		listTokens()
	}

	async function handleDeleteClick(tokenPrefix: string) {
		await UserService.deleteToken({ tokenPrefix })
		sendUserToast('Successfully deleted token')
		listTokens()
	}

	async function listTokens(): Promise<void> {
		tokens = await UserService.listTokens({
			excludeEphemeral: true,
			page: tokenPage,
			perPage: 100
		})
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

<div class="flex flex-col p-4 border border-border-light rounded-md">
	<h2 class="text-emphasis text-sm font-semibold mb-1">Tokens</h2>
	<div class="text-xs text-secondary mb-2">
		Authenticate to the Windmill API with access tokens.
	</div>
	{#if expiringSoonCount > 0}
		<div class="mb-2">
			<Alert type="warning" title="{expiringSoonCount} token{expiringSoonCount > 1 ? 's' : ''} expiring within 7 days" size="xs" />
		</div>
	{/if}
	<CreateToken
		{showMcpMode}
		{openWithMcpMode}
		bind:newTokenLabel
		{defaultNewTokenWorkspace}
		{scopes}
		onTokenCreated={handleTokenCreated}
	/>
	<div class="overflow-auto grow min-h-64 max-h-2/3">
		<TableCustom>
			<!-- @migration-task: migrate this slot by hand, `header-row` is an invalid identifier -->
			<tr slot="header-row">
				<th>Prefix</th>
				<th>Label</th>
				<th>Expiration</th>
				<th>Scopes</th>
				<th></th>
			</tr>
			{#snippet body()}
				<tbody>
					{#if tokens && tokens.length > 0}
						{#each tokens as { token_prefix, expiration, label, scopes } (token_prefix)}
							{@const badge = expirationBadge(expiration, label)}
							<tr>
								<td class="w-32 text-xs text-primary">{token_prefix}****</td>
								<td class="min-w-0 max-w-32 truncate text-xs text-primary">{label ?? ''}</td>
								<td class="w-40 whitespace-nowrap text-xs text-secondary">
									<div class="flex items-center gap-1.5">
										{displayDate(expiration ?? '')}
										{#if badge}
											<Badge color={badge.color} small>{badge.text}</Badge>
										{/if}
									</div>
								</td>
								<td
									class="min-w-0 max-w-48 truncate text-xs text-secondary"
									title={scopes?.join(', ') ?? ''}>{scopes?.join(', ') ?? ''}</td
								>
								<td class="w-16 text-center">
									<Button
										variant="subtle"
										destructive
										on:click={() => handleDeleteClick(token_prefix)}
										size="xs"
										startIcon={{ icon: Trash }}
										iconOnly
									/>
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
</div>
