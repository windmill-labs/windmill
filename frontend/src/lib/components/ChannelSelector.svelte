<script lang="ts">
	import Select from './select/Select.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { debounce } from '$lib/utils'

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
		onError,
		onSelectedChannelChange
	}: Props = $props()

	let isFetching = $state(false)
	let searchResults = $state<ChannelItem[]>([])

	let selectedChannelId = $state<string | undefined>(selectedChannel?.channel_id)

	const searchMode = !channels && !!teamId

	let displayChannels = $derived.by(() => {
		const baseChannels = channels || searchResults
		if (selectedChannel && !baseChannels.find(c => c.channel_id === selectedChannel?.channel_id)) {
			return [selectedChannel, ...baseChannels]
		}
		return baseChannels
	})

	$effect(() => {
		const newChannel = selectedChannelId
			? displayChannels.find(c => c.channel_id === selectedChannelId)
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

	let searchFilterText = $state('')

	const debouncedSearch = debounce(async (query: string) => {
		await searchChannels(query)
	}, 500)

	$effect(() => {
		if (searchMode) {
			if (searchFilterText.length >= 1) {
				debouncedSearch.debounced(searchFilterText)
			} else if (searchFilterText.length === 0) {
				searchResults = []
			}
		}
	})

	async function searchChannels(query: string) {
		if (!query || !teamId) return

		isFetching = true
		try {
			const response = await WorkspaceService.listAvailableTeamsChannels({
				workspace: $workspaceStore!,
				teamId: teamId,
				search: query
			})

			searchResults = response || []
			isFetching = false
			return searchResults
		} catch (error) {
			isFetching = false
			onError?.(error)
			console.error('Error searching channels:', error)
			searchResults = []
			return []
		}
	}

</script>

<div class={containerClass}>
	<div class="flex items-center gap-2">
		<div class="flex-grow" style="min-width: {minWidth};">
			{#if searchMode}
				<Select
					containerStyle={'min-width: ' + minWidth}
					items={displayChannels.filter(channel => channel.channel_id && channel.channel_name).map((channel) => ({
						label: channel.channel_name ?? 'Unknown Channel',
						value: channel.channel_id ?? ''
					}))}
					placeholder={isFetching ? "Searching..." : (teamId ? "Search channels..." : "Select a team first")}
					clearable
					disabled={disabled || isFetching || !teamId}
					bind:filterText={searchFilterText}
					bind:value={selectedChannelId}
				/>
			{:else}
				<Select
					containerStyle={'min-width: ' + minWidth}
					items={displayChannels.filter(channel => channel.channel_id && channel.channel_name).map((channel) => ({
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
	</div>

</div>
