<script lang="ts">
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { displayDate } from '$lib/utils'
	import { UserService, type TruncatedToken } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import CreateToken from './CreateToken.svelte'
	import Button from '../common/button/Button.svelte'
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
	<div class="text-xs text-primary mb-2">
		Authenticate to the Windmill API with access tokens.
	</div>
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
				<th>prefix</th>
				<th>label</th>
				<th>expiration</th>
				<th>scopes</th>
				<th></th>
			</tr>
			{#snippet body()}
				<tbody>
					{#if tokens && tokens.length > 0}
						{#each tokens as { token_prefix, expiration, label, scopes }}
							<tr>
								<td class="w-32">{token_prefix}****</td>
								<td class="min-w-0 max-w-32 truncate">{label ?? ''}</td>
								<td class="w-24 whitespace-nowrap">{displayDate(expiration ?? '')}</td>
								<td class="min-w-0 max-w-48 truncate" title={scopes?.join(', ') ?? ''}
									>{scopes?.join(', ') ?? ''}</td
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
							<td class="text-secondary italic text-xs"> There are no tokens yet</td>
						</tr>
					{:else}
						<tr><td>Loading...</td></tr>
					{/if}
				</tbody>
			{/snippet}
		</TableCustom>
		<div class="flex flex-row-reverse gap-2 w-full">
			{#if tokens?.length == 100}
				<button
					class="p-1 underline text-sm whitespace-nowrap text-center"
					onclick={handleNextPage}
				>
					Next
				</button>
			{/if}
			{#if tokenPage > 1}
				<button
					class="p-1 underline text-sm whitespace-nowrap text-center"
					onclick={handlePreviousPage}
				>
					Previous
				</button>
			{/if}
		</div>
	</div>
</div>
