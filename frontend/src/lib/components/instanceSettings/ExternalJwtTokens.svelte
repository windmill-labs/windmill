<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import type { ExternalJwtToken } from '$lib/gen'
	import { displayDate } from '$lib/utils'
	import { Check, X } from 'lucide-svelte'

	interface Props {
		tokens: ExternalJwtToken[]
		hasMore: boolean
		loading: boolean
		activeOnly: boolean
		onLoadMore: () => void
		onActiveOnlyChange: (v: boolean) => void
	}

	let { tokens, hasMore, loading, activeOnly, onLoadMore, onActiveOnlyChange }: Props = $props()
	const loadMoreSize = 50
</script>

<SettingsPageHeader
	title="External JWTs"
	description="External JWT tokens that have authenticated against this instance, deduplicated by their claims."
/>

<div class="flex flex-row gap-2 items-center mb-2">
	<Toggle
		checked={activeOnly}
		on:change={(e) => onActiveOnlyChange(e.detail)}
		options={{
			left: 'Recently active only',
			leftTooltip: 'Show only tokens used in the last 30 days'
		}}
	/>
</div>

<DataTable
	shouldLoadMore={hasMore}
	loadMore={loadMoreSize}
	{loading}
	on:loadMore={() => onLoadMore()}
>
	<Head>
		<tr>
			<Cell head first>Email</Cell>
			<Cell head>Username</Cell>
			<Cell head>Admin</Cell>
			<Cell head>Operator</Cell>
			<Cell head>Workspace</Cell>
			<Cell head>Label</Cell>
			<Cell head>Scopes</Cell>
			<Cell head last>Last Used</Cell>
		</tr>
	</Head>
	<tbody>
		{#each tokens as token, i (token.jwt_hash)}
			<tr class={i % 2 === 0 ? 'bg-surface-tertiary' : 'bg-surface'}>
				{#if token.email === ''}
					<Cell first colspan={7}>
						<span class="text-tertiary italic">Legacy entry — details unavailable</span>
					</Cell>
					<Cell last><span class="whitespace-nowrap">{displayDate(token.last_used_at)}</span></Cell>
				{:else}
					<Cell first><span class="font-mono text-xs">{token.email}</span></Cell>
					<Cell>{token.username}</Cell>
					<Cell>
						{#if token.is_admin}
							<Check size={14} class="text-green-600" />
						{:else}
							<X size={14} class="text-tertiary" />
						{/if}
					</Cell>
					<Cell>
						{#if token.is_operator}
							<Check size={14} class="text-green-600" />
						{:else}
							<X size={14} class="text-tertiary" />
						{/if}
					</Cell>
					<Cell>{token.workspace_id ?? '-'}</Cell>
					<Cell>{token.label ?? '-'}</Cell>
					<Cell>
						{#if token.scopes && token.scopes.length > 0}
							{token.scopes.join(', ')}
						{:else}
							-
						{/if}
					</Cell>
					<Cell last><span class="whitespace-nowrap">{displayDate(token.last_used_at)}</span></Cell>
				{/if}
			</tr>
		{/each}
	</tbody>
</DataTable>
