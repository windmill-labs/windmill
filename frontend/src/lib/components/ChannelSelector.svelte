<script lang="ts">
	import Select from './select/Select.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { RefreshCcw } from 'lucide-svelte'
	import { Button } from './common'

	interface ChannelItem {
		channel_id?: string
		channel_name?: string
	}

	interface Props {
		disabled?: boolean
		placeholder?: string
		selectedChannel?: ChannelItem | undefined
		containerClass?: string
		minWidth?: string
		channels?: ChannelItem[]
		teamId?: string
		showRefreshButton?: boolean
		onError?: (error: Error) => void
		onSelectedChannelChange?: (channel: ChannelItem | undefined) => void
	}

	let {
		disabled = false,
		placeholder = 'Select channel',
		selectedChannel = $bindable(undefined),
		containerClass = 'w-64',
		minWidth = '160px',
		channels = undefined,
		teamId,
		showRefreshButton = true,
		onError,
		onSelectedChannelChange
	}: Props = $props()

	let isFetching = $state(false)
	let loadedChannels = $state<ChannelItem[]>([])
	let loadedForTeamId = $state<string | undefined>(undefined)

	let selectedChannelId = $state<string | undefined>(selectedChannel?.channel_id)

	const searchMode = $derived(!channels && !!teamId)

	let displayChannels = $derived.by(() => {
		const baseChannels = channels || loadedChannels
		if (
			selectedChannel &&
			!baseChannels.find((c) => c.channel_id === selectedChannel?.channel_id)
		) {
			return [selectedChannel, ...baseChannels]
		}
		return baseChannels
	})

	$effect(() => {
		const newChannel = selectedChannelId
			? displayChannels.find((c) => c.channel_id === selectedChannelId)
			: undefined

		if (newChannel?.channel_id !== selectedChannel?.channel_id) {
			selectedChannel = newChannel
		}
	})

	$effect(() => {
		if (selectedChannel?.channel_id !== selectedChannelId) {
			selectedChannelId = selectedChannel?.channel_id
		}
	})

	let previousChannelId = $state<string | undefined>(undefined)

	$effect(() => {
		if (selectedChannel?.channel_id !== previousChannelId) {
			previousChannelId = selectedChannel?.channel_id
			onSelectedChannelChange?.(selectedChannel)
		}
	})

	// Fetch channels when teamId is set or changes
	$effect(() => {
		if (searchMode && teamId && teamId !== loadedForTeamId) {
			loadedForTeamId = teamId
			fetchChannels()
		}
	})

	async function fetchChannels() {
		if (!teamId) return

		isFetching = true
		try {
			const response = await WorkspaceService.listAvailableTeamsChannels({
				workspace: $workspaceStore!,
				teamId: teamId
			})

			loadedChannels =
				response.channels?.map((c) => ({
					channel_id: c.channel_id || '',
					channel_name: c.channel_name || ''
				})) || []
		} catch (error) {
			onError?.(error as Error)
			console.error('Error fetching channels:', error)
			loadedChannels = []
		} finally {
			isFetching = false
		}
	}

	async function refreshChannels() {
		if (searchMode) {
			await fetchChannels()
		}
	}
</script>

<div class={containerClass}>
	<div class="flex flex-col gap-1">
		<div class="flex items-center gap-1">
			<div class="flex-grow" style="min-width: {minWidth};">
				{#if searchMode}
					<Select
						containerStyle={'min-width: ' + minWidth}
						items={displayChannels
							.filter((channel) => channel.channel_id && channel.channel_name)
							.map((channel) => ({
								label: channel.channel_name ?? 'Unknown Channel',
								value: channel.channel_id ?? ''
							}))}
						placeholder={isFetching ? 'Loading...' : teamId ? placeholder : 'Select a team first'}
						clearable
						disabled={disabled || !teamId}
						loading={isFetching}
						bind:value={selectedChannelId}
					/>
				{:else}
					<Select
						containerStyle={'min-width: ' + minWidth}
						items={displayChannels
							.filter((channel) => channel.channel_id && channel.channel_name)
							.map((channel) => ({
								label: channel.channel_name ?? 'Unknown Channel',
								value: channel.channel_id ?? ''
							}))}
						{placeholder}
						clearable
						disabled={disabled || displayChannels.length === 0}
						bind:value={selectedChannelId}
					/>
				{/if}
			</div>

			{#if showRefreshButton && searchMode}
				<Button
					onclick={refreshChannels}
					disabled={isFetching || disabled || !teamId}
					title="Refresh channels"
					startIcon={{ icon: RefreshCcw, props: { class: isFetching ? 'animate-spin' : '' } }}
					unifiedSize="sm"
					variant="subtle"
					iconOnly
				/>
			{/if}
		</div>

		{#if searchMode && loadedChannels.length > 0 && !isFetching}
			<span class="text-2xs text-tertiary pl-1">
				{loadedChannels.length} channel{loadedChannels.length === 1 ? '' : 's'}
			</span>
		{/if}
	</div>
</div>
