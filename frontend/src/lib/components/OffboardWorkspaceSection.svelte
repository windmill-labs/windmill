<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { ChevronDown, ChevronRight, Download } from 'lucide-svelte'
	import type { OffboardPreview } from '$lib/gen/types.gen'
	import OffboardItemsBox from './OffboardItemsBox.svelte'
	import OffboardReassignControls from './OffboardReassignControls.svelte'
	import { countPaths, pl, downloadCsv, flattenPaths } from './offboarding-utils'

	type Props = {
		preview: OffboardPreview
		username: string
		deleteUser?: boolean
		targetKind: 'user' | 'folder'
		selectedUser: string | undefined
		selectedFolder: string | undefined
		selectedOperator: string | undefined
		users: Array<{ label: string; value: string }>
		folders: Array<{ label: string; value: string }>
		size?: 'sm' | 'md'
		csvFilename?: string
		instanceLevel?: boolean
	}

	let {
		preview,
		username,
		deleteUser = true,
		targetKind = $bindable(),
		selectedUser = $bindable(),
		selectedFolder = $bindable(),
		selectedOperator = $bindable(),
		users,
		folders,
		size = 'md',
		csvFilename = `offboard-${username}.csv`,
		instanceLevel = false
	}: Props = $props()

	let ownedCount = $derived(countPaths(preview.owned))
	let onBehalfCount = $derived(countPaths(preview.executing_on_behalf))
	let referencingCount = $derived(countPaths(preview.referencing))

	function downloadAffectedCsv() {
		if (!preview.referencing) return
		const rows: string[][] = [['type', 'path']]
		for (const { kind, path } of flattenPaths(preview.referencing)) {
			rows.push([kind, path])
		}
		downloadCsv(rows, csvFilename)
	}

	let tokensExpanded = $state(false)
</script>

<div class="flex flex-col gap-2">
	{#if ownedCount > 0}
		<OffboardItemsBox title="u/{username}/* items ({ownedCount})" paths={preview.owned} />
	{/if}
	{#if onBehalfCount > 0}
		<OffboardItemsBox
			title="Permissioned as u/{username} ({onBehalfCount})"
			paths={preview.executing_on_behalf}
		/>
	{/if}
	{#if deleteUser && (!instanceLevel || (preview.tokens?.length ?? 0) > 0)}
		<div
			class="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-700/40 rounded-md p-3"
		>
			{#if (preview.tokens?.length ?? 0) > 0}
				<button
					class="flex items-center gap-1 w-full text-left"
					onclick={() => (tokensExpanded = !tokensExpanded)}
				>
					{#if tokensExpanded}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
					<span class="text-xs font-medium text-yellow-800 dark:text-yellow-100/90"
						>Tokens ({preview.tokens?.length})</span
					>
				</button>
				<p class="text-xs text-yellow-700 dark:text-yellow-100/70 mt-1 ml-5">
					All workspace tokens for this user will be deleted. This may break webhooks and HTTP
					triggers using these tokens.
				</p>
			{:else}
				<p class="text-xs font-medium text-yellow-800 dark:text-yellow-100/90">Tokens</p>
				<p class="text-xs text-yellow-700 dark:text-yellow-100/70 mt-1">
					All workspace tokens for this user will be deleted. This may break webhooks and HTTP
					triggers using these tokens.
				</p>
			{/if}
			{#if tokensExpanded && (preview.tokens?.length ?? 0) > 0}
				<div
					class="flex flex-col gap-0.5 text-xs text-yellow-800 dark:text-yellow-100/90 mt-1.5 ml-5"
				>
					{#each preview.tokens ?? [] as token}
						<span>{token.label || '(no label)'}: {token.scopes?.join(', ') || '(no scopes)'}</span>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
	{#if referencingCount > 0}
		<OffboardItemsBox
			title="Referencing items ({referencingCount})"
			paths={preview.referencing}
			variant="warning"
			description="These items reference u/{username}/* paths and may break after reassignment. Check them and update manually."
		>
			{#snippet headerExtra()}
				<Button
					variant="subtle"
					size="xs2"
					startIcon={{ icon: Download }}
					onclick={downloadAffectedCsv}>CSV</Button
				>
			{/snippet}
		</OffboardItemsBox>
	{/if}
</div>

{#if preview.http_triggers > 0 || preview.email_triggers > 0}
	<Alert type="warning" title="Webhook and email trigger URLs will change">
		<p class="text-xs">
			{#if preview.http_triggers > 0}{pl(preview.http_triggers, 'HTTP trigger')} will have new webhook
				URLs.
			{/if}
			{#if preview.email_triggers > 0}{pl(preview.email_triggers, 'email trigger')} will have new addresses.
			{/if}
			Update any external integrations that reference these endpoints.
		</p>
	</Alert>
{/if}

<OffboardReassignControls
	{username}
	bind:targetKind
	bind:selectedUser
	bind:selectedFolder
	bind:selectedOperator
	{users}
	{folders}
	showTargetSelector={ownedCount > 0 || onBehalfCount > 0}
	{size}
/>
