<script lang="ts">
	import { Button } from '$lib/components/common'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import { OidcService } from '$lib/gen'
	import type { ExternalJwtToken } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { displayDate } from '$lib/utils'
	import { Loader2, RefreshCw, ChevronLeft, ChevronRight, Check, X } from 'lucide-svelte'
	import { onMount } from 'svelte'

	let loading = $state(false)
	let tokens: ExternalJwtToken[] = $state([])
	let page = $state(1)
	const perPage = 100

	async function loadTokens() {
		loading = true
		try {
			tokens = await OidcService.listExtJwtTokens({
				page: page,
				perPage: perPage
			})
		} catch (e) {
			sendUserToast(`Failed to load external JWT tokens: ${e}`, true)
			tokens = []
		} finally {
			loading = false
		}
	}

	onMount(() => {
		loadTokens()
	})
</script>

<SettingsPageHeader
	title="External JWTs"
	description="List of unique external JWT tokens that have been used for authentication. Tokens are deduplicated by their claims (email, username, permissions)."
/>

<div class="flex items-center justify-between mb-4">
	<div class="flex items-center gap-2">
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: RefreshCw }}
			onclick={loadTokens}
			disabled={loading}
		>
			Refresh
		</Button>
	</div>
	<div class="flex items-center gap-2">
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: ChevronLeft }}
			iconOnly
			disabled={page <= 1}
			onclick={() => {
				page -= 1
				loadTokens()
			}}
		/>
		<span class="text-xs text-secondary">Page {page}</span>
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: ChevronRight }}
			iconOnly
			disabled={tokens.length < perPage}
			onclick={() => {
				page += 1
				loadTokens()
			}}
		/>
	</div>
</div>

{#if loading}
	<div class="flex items-center justify-center py-8">
		<Loader2 class="animate-spin" size={20} />
		<span class="ml-2 text-sm text-secondary">Loading tokens...</span>
	</div>
{:else if tokens.length === 0}
	<div class="text-center py-8 text-sm text-tertiary"> No external JWT tokens found </div>
{:else}
	<div class="overflow-x-auto border rounded-md">
		<table class="w-full text-sm">
			<thead>
				<tr class="border-b bg-surface-secondary text-left text-xs text-secondary">
					<th class="px-3 py-2">Email</th>
					<th class="px-3 py-2">Username</th>
					<th class="px-3 py-2">Admin</th>
					<th class="px-3 py-2">Operator</th>
					<th class="px-3 py-2">Workspace</th>
					<th class="px-3 py-2">Label</th>
					<th class="px-3 py-2">Scopes</th>
					<th class="px-3 py-2">Last Used</th>
				</tr>
			</thead>
			<tbody>
				{#each tokens as token (token.jwt_hash)}
					<tr class="border-b last:border-b-0 hover:bg-surface-hover">
						<td class="px-3 py-2 font-mono text-xs">{token.email}</td>
						<td class="px-3 py-2">{token.username}</td>
						<td class="px-3 py-2">
							{#if token.is_admin}
								<Check size={14} class="text-green-600" />
							{:else}
								<X size={14} class="text-tertiary" />
							{/if}
						</td>
						<td class="px-3 py-2">
							{#if token.is_operator}
								<Check size={14} class="text-green-600" />
							{:else}
								<X size={14} class="text-tertiary" />
							{/if}
						</td>
						<td class="px-3 py-2 text-xs">{token.workspace_id ?? '-'}</td>
						<td class="px-3 py-2 text-xs">{token.label ?? '-'}</td>
						<td class="px-3 py-2 text-xs">
							{#if token.scopes && token.scopes.length > 0}
								{token.scopes.join(', ')}
							{:else}
								-
							{/if}
						</td>
						<td class="px-3 py-2 text-xs whitespace-nowrap">{displayDate(token.last_used_at)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
