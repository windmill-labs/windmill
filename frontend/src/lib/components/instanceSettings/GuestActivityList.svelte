<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import { Alert } from '$lib/components/common'
	import type { GuestActivity, GuestUsage } from '$lib/gen'

	interface Props {
		usage: GuestUsage
		guests: GuestActivity[]
		hasMore: boolean
		loading: boolean
		onLoadMore: () => void
	}

	let { usage, guests, hasMore, loading, onLoadMore }: Props = $props()
	const loadMoreSize = 50
	let pastAllowance = $derived(usage.guest_count > usage.free_allowance)
</script>

<SettingsPageHeader
	title="Guests"
	description="People your identity provider authenticated who opened an app set to Guests without a Windmill account. One email is one guest, however many workspaces it opened."
/>

<div class="mb-4">
	<Alert type={pastAllowance ? 'warning' : 'info'} size="xs" title="{usage.guest_count} of {usage.free_allowance} free guests used in the last {usage.window_days} days">
		{#if usage.metered}
			Beyond the allowance, every four guests count as one seat{usage.guest_seats > 0
				? `: ${usage.billable_guests} guests past it take ${usage.guest_seats} ${usage.guest_seats === 1 ? 'seat' : 'seats'} now`
				: ''}.
		{:else}
			Beyond the allowance, new guests are refused until the count drops below it; an
			Enterprise license meters them instead.
		{/if}
	</Alert>
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
			<Cell head>Workspaces</Cell>
			<Cell head>First seen</Cell>
			<Cell head last>Last seen</Cell>
		</tr>
	</Head>
	<tbody>
		{#each guests as guest, i (guest.email)}
			<tr class={i % 2 === 0 ? 'bg-surface-tertiary' : 'bg-surface'}>
				<Cell first><span class="font-mono text-xs">{guest.email}</span></Cell>
				<Cell>{guest.workspaces.join(', ')}</Cell>
				<Cell><span class="whitespace-nowrap">{guest.first_seen}</span></Cell>
				<Cell last><span class="whitespace-nowrap">{guest.last_seen}</span></Cell>
			</tr>
		{/each}
	</tbody>
</DataTable>
