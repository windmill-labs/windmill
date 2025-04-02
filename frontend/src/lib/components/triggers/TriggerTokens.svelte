<script lang="ts">
	import { FlowService, ScriptService, UserService, type TruncatedToken } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getContext } from 'svelte'
	import { Skeleton } from '../common'
	import Label from '../Label.svelte'
	import type { TriggerContext } from '../triggers'
	import { capitalize } from '$lib/utils'

	export let isFlow: boolean
	export let path: string
	export let labelPrefix: 'email' | 'webhook'

	const { triggersCount } = getContext<TriggerContext>('TriggerContext')

	let tokens: TruncatedToken[] | undefined = undefined

	export async function listTokens() {
		tokens = (
			isFlow
				? await FlowService.listTokensOfFlow({ workspace: $workspaceStore!, path })
				: await ScriptService.listTokensOfScript({ workspace: $workspaceStore!, path })
		).filter((x) => x.label && x.label.startsWith(labelPrefix + '-'))
		if (labelPrefix == 'email') {
			$triggersCount = { ...($triggersCount ?? {}), email_count: tokens?.length }
		} else {
			$triggersCount = { ...($triggersCount ?? {}), webhook_count: tokens?.length }
		}
	}

	async function deleteToken(tokenPrefix: string) {
		await UserService.deleteToken({
			tokenPrefix: tokenPrefix
		})
		await listTokens()
	}

	$: $workspaceStore && listTokens()
</script>

<div class="flex flex-col gap-2">
	<Label label="Existing {capitalize(labelPrefix)} Tokens">
		{#if tokens}
			{#if tokens.length == 0}
				<div class="text-xs text-secondary">No {labelPrefix} specific tokens found</div>
			{:else}
				<div class="flex flex-col divide-y pt-2">
					<div class="grid grid-cols-6 text-2xs items-center py-2">
						<div class="col-span-2 truncate font-semibold">Prefix</div>
						<div class="col-span-2 truncate font-semibold">Label</div>
						<div class="col-span-1 truncate font-semibold">Owner</div>
						<div class="col-span-1"></div>
					</div>
					{#each tokens as token (token.token_prefix)}
						<div class="grid grid-cols-6 text-2xs items-center py-2">
							<div class="col-span-2 truncate">{token.token_prefix}***</div>
							<div class="col-span-2 truncate">
								{token.label}
							</div>
							<div class="col-span-1 truncate">
								{token.email}
							</div>
							{#if token.email == $userStore?.email}
								<button
									on:click={() => deleteToken(token.token_prefix)}
									class="text-xs text-secondary">delete</button
								>
							{:else}
								<div class="col-span-1"></div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		{:else}
			<Skeleton layout={[[8]]} />
		{/if}
	</Label>
</div>
