<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { Download } from 'lucide-svelte'
	import type { OffboardPreview } from '$lib/gen/types.gen'
	import OffboardItemsBox from './OffboardItemsBox.svelte'
	import OffboardReassignControls from './OffboardReassignControls.svelte'
	import { countPaths, pl, downloadCsv } from './offboarding-utils'

	type Props = {
		preview: OffboardPreview
		username: string
		targetKind: 'user' | 'folder'
		selectedUser: string | undefined
		selectedFolder: string | undefined
		selectedOperator: string | undefined
		users: Array<{ label: string; value: string }>
		folders: Array<{ label: string; value: string }>
		size?: 'sm' | 'md'
		csvFilename?: string
	}

	let {
		preview,
		username,
		targetKind = $bindable(),
		selectedUser = $bindable(),
		selectedFolder = $bindable(),
		selectedOperator = $bindable(),
		users,
		folders,
		size = 'md',
		csvFilename = `offboard-${username}.csv`
	}: Props = $props()

	let ownedCount = $derived(countPaths(preview.owned))
	let onBehalfCount = $derived(countPaths(preview.executing_on_behalf))
	let referencingCount = $derived(countPaths(preview.referencing))

	function downloadAffectedCsv() {
		if (!preview.referencing) return
		const rows: string[][] = [['type', 'path']]
		for (const [kind, list] of Object.entries(preview.referencing)) {
			if (Array.isArray(list)) {
				for (const p of list) rows.push([kind, p])
			}
		}
		downloadCsv(rows, csvFilename)
	}
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
	{#if (preview.tokens?.length ?? 0) > 0}
		<div class="bg-surface-secondary rounded-md p-3">
			<p class="text-xs font-medium text-primary mb-0.5">Tokens ({preview.tokens?.length ?? 0})</p>
			<p class="text-xs text-tertiary mb-1">These tokens will be deleted.</p>
			<div class="flex flex-col gap-0.5 text-xs text-secondary">
				{#each preview.tokens ?? [] as token}
					<span
						>{token.label || '(no label)'}{#if token.scopes?.length}: {token.scopes.join(
								', '
							)}{/if}</span
					>
				{/each}
			</div>
		</div>
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
	showTargetSelector={ownedCount > 0}
	{size}
/>
