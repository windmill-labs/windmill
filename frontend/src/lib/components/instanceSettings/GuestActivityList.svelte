<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Alert } from '$lib/components/common'
	import { SettingService, type GuestActivity, type GuestUsage } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		usage: GuestUsage
		guests: GuestActivity[]
		hasMore: boolean
		loading: boolean
		onLoadMore: () => void
		/** The instance switch was written; the caller re-reads usage and says whether
		 * that read succeeded, so the toggle can show what is actually stored. */
		onInstanceSwitch: () => Promise<boolean>
	}

	let { usage, guests, hasMore, loading, onLoadMore, onInstanceSwitch }: Props = $props()
	const loadMoreSize = 50
	// One write at a time, and the toggle always ends on what is stored: the reloaded
	// value when the reload succeeds, else the write's outcome.
	let switchPending = $state(false)
	let switchOn = $state(usage.instance_enabled)
	$effect(() => {
		switchOn = usage.instance_enabled
	})

	async function setInstanceSwitch(enabled: boolean) {
		switchPending = true
		let written = false
		try {
			await SettingService.setGlobal({
				key: 'guest_access_disabled',
				requestBody: { value: !enabled }
			})
			written = true
			sendUserToast(
				enabled
					? 'Guests can sign in again where a workspace allows them'
					: 'Guests can no longer sign in anywhere on this instance'
			)
		} catch (e) {
			sendUserToast(`Could not change the instance guest switch: ${e}`, true)
		}
		const reloaded = await onInstanceSwitch()
		if (!reloaded) {
			switchOn = written ? enabled : usage.instance_enabled
		}
		switchPending = false
	}
	// A capped instance refuses the next stranger as soon as the allowance is used up.
	let pastAllowance = $derived(
		usage.metered
			? usage.guest_count > usage.free_allowance
			: usage.guest_count >= usage.free_allowance
	)
</script>

<SettingsPageHeader
	title="Guests"
	description="People your identity provider authenticated who opened an app set to Guests without a Windmill account. One email is one guest, however many workspaces it opened."
/>

<div class="flex flex-row gap-2 items-center mb-4">
	{#key usage}
		<Toggle
			bind:checked={switchOn}
			disabled={switchPending}
			on:change={(e) => setInstanceSwitch(e.detail)}
			options={{
				right: 'Allow guests on this instance',
				rightTooltip:
					'Off, no guest can sign in anywhere, whatever a workspace or an app says, and sessions already issued stop on their next request.'
			}}
		/>
	{/key}
</div>

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
