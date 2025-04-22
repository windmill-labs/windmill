<script lang="ts">
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { displayDate, copyToClipboard } from '$lib/utils'
	import type { TruncatedToken, NewToken } from '$lib/gen'
	import { UserService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { Clipboard, Plus } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'

	// --- Props ---
	interface Props {
		tokens?: TruncatedToken[]
		defaultNewTokenLabel?: string
		defaultNewTokenWorkspace?: string
		onDeleteToken: (tokenPrefix: string) => void
		tokenPage?: number
		onNextPage: () => void
		onPreviousPage: () => void
		onListTokens: () => void
		scopes?: string[]
	}

	let {
		tokens = [],
		onDeleteToken,
		defaultNewTokenLabel,
		defaultNewTokenWorkspace,
		tokenPage = 1,
		onNextPage,
		onPreviousPage,
		onListTokens,
		scopes
	}: Props = $props()

	// --- Local State ---
	let newTokenLabel = $state<string | undefined>(defaultNewTokenLabel)
	let newToken = $state<string | undefined>(undefined)
	let newTokenExpiration = $state<number | undefined>(undefined)
	let newTokenWorkspace = $state<string | undefined>(defaultNewTokenWorkspace)
	let displayCreateToken = $state(scopes != undefined)

	const dispatch = createEventDispatcher()

	// --- Functions ---
	async function createToken(): Promise<void> {
		try {
			let date: Date | undefined
			if (newTokenExpiration) {
				date = new Date(new Date().getTime() + newTokenExpiration * 1000)
			}

			newToken = await UserService.createToken({
				requestBody: {
					label: newTokenLabel,
					expiration: date?.toISOString(),
					scopes,
					workspace_id: newTokenWorkspace || $workspaceStore
				} as NewToken
			})

			dispatch('tokenCreated', newToken)
			onListTokens()
			displayCreateToken = false
		} catch (err) {
			console.error('Failed to create token:', err)
		}
	}

	function handleCreateClick() {
		displayCreateToken = !displayCreateToken
		newToken = undefined
		newTokenExpiration = undefined
		newTokenLabel = undefined
	}

	function handleCopyClick() {
		copyToClipboard(newToken ?? '')
	}

	function handleDeleteClick(tokenPrefix: string) {
		onDeleteToken(tokenPrefix)
	}
</script>

<div class="grid grid-cols-2 pt-8 pb-1" class:pt-8={scopes == undefined}>
	<h2 class="py-0 my-0 border-b pt-3">Tokens</h2>
	<div class="flex justify-end border-b pb-1">
		<Button
			size="sm"
			startIcon={{ icon: Plus }}
			btnClasses={displayCreateToken ? 'hidden' : ''}
			on:click={handleCreateClick}
		>
			Create token
		</Button>
	</div>
</div>
<div class="text-2xs text-tertiary italic pb-6">
	Authenticate to the Windmill API with access tokens.
</div>

<div>
	{#if newToken}
		<div
			class="border rounded-md mb-6 px-2 py-2 bg-green-50 dark:bg-green-200 dark:text-green-800 flex flex-row flex-wrap"
		>
			<div>
				Added token: <button onclick={handleCopyClick} class="inline-flex gap-2 items-center">
					{newToken}
					<Clipboard size={12} />
				</button>
			</div>
			<div class="pt-1 text-xs ml-2">
				Make sure to copy your personal access token now. You won't be able to see it again!
			</div>
		</div>
	{/if}

	<!-- Token creation interface -->
	{#if displayCreateToken}
		<div class="py-3 px-3 border rounded-md mb-6 bg-surface-secondary min-w-min">
			<h3 class="pb-3 font-semibold">Add a new token</h3>
			{#if scopes != undefined}
				{#each scopes as scope}
					<div class="flex flex-col mb-4">
						<label for="label">Scope</label>
						<input disabled type="text" value={scope} />
					</div>
				{/each}
			{/if}
			<div class="flex flex-row flex-wrap gap-x-2 w-full justify-between">
				<div class="flex flex-col">
					<label for="label">Label <span class="text-xs text-tertiary">(optional)</span></label>
					<input type="text" bind:value={newTokenLabel} />
				</div>
				<div class="flex flex-col">
					<label for="expires"
						>Expires In &nbsp;<span class="text-xs text-tertiary">(optional)</span>
					</label>
					<select bind:value={newTokenExpiration}>
						<option value={undefined}>No expiration</option>
						<option value={15 * 60}>15m</option>
						<option value={30 * 60}>30m</option>
						<option value={1 * 60 * 60}>1h</option>
						<option value={1 * 24 * 60 * 60}>1d</option>
						<option value={7 * 24 * 60 * 60}>7d</option>
						<option value={30 * 24 * 60 * 60}>30d</option>
						<option value={90 * 24 * 60 * 60}>90d</option>
					</select>
				</div>
				<div class="flex items-end">
					<Button btnClasses="!mt-2" on:click={createToken}>New token</Button>
				</div>
			</div>
		</div>
	{/if}
</div>

<div class="overflow-auto">
	<TableCustom>
		<tr slot="header-row">
			<th>prefix</th>
			<th>label</th>
			<th>expiration</th>
			<th>scopes</th>
			<th></th>
		</tr>
		<tbody slot="body">
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
	</TableCustom>
	<div class="flex flex-row-reverse gap-2 w-full">
		{#if tokens?.length == 100}
			<button class="p-1 underline text-sm whitespace-nowrap text-center" onclick={onNextPage}>
				Next
			</button>
		{/if}
		{#if tokenPage > 1}
			<button class="p-1 underline text-sm whitespace-nowrap text-center" onclick={onPreviousPage}>
				Previous
			</button>
		{/if}
	</div>
</div>
