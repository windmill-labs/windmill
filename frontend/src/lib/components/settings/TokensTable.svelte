<script lang="ts">
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { displayDate } from '$lib/utils'
	import type { TruncatedToken } from '$lib/gen'
	import { UserService } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import CreateToken from './CreateToken.svelte'

	// --- Props ---
	interface Props {
		showMcpMode?: boolean
		openWithMcpMode?: boolean
		defaultNewTokenLabel?: string
		defaultNewTokenWorkspace?: string
		scopes?: string[]
	}

	let {
		showMcpMode = false,
		openWithMcpMode = false,
		defaultNewTokenLabel,
		defaultNewTokenWorkspace,
		scopes
	}: Props = $props()

	// --- Local State ---
	let tokens = $state<TruncatedToken[]>([])
	let tokenPage = $state(1)
	let newTokenLabel = $state<string | undefined>(defaultNewTokenLabel)

	const dispatch = createEventDispatcher()

	$effect(() => {
		listTokens()
	})

	function handleTokenCreated(token: string) {
		dispatch('tokenCreated', token)
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

<div class="flex flex-col min-h-0">
	<div class="text-2xs text-tertiary italic pb-2 mt-2">
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
	<div class="overflow-auto flex-1 min-h-64">
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
								<td class="grow">{token_prefix}****</td>
								<td class="grow">{label ?? ''}</td>
								<td class="grow">{displayDate(expiration ?? '')}</td>
								<td class="grow">{scopes?.join(', ') ?? ''}</td>
								<td class="grow">
									<button
										class="text-red-500 text-xs underline"
										onclick={() => handleDeleteClick(token_prefix)}
									>
										Delete
									</button>
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
