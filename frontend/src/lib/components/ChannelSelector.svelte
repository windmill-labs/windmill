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
	}

	let {
		disabled = false,
		placeholder = 'Select channel',
		selectedChannel = $bindable(undefined),
		containerClass = 'w-64',
		minWidth = '160px',
		channels = undefined,
		teamId,
		onError
	}: Props = $props()

	let isFetching = $state(false)
	let hasSearched = $state(false)
	let searchResults = $state<ChannelItem[]>([])

	// Only enable search mode if no channels are provided AND teamId is provided
	const searchMode = !channels && !!teamId

	// Determine which channels to show: provided channels or search results
	// In search mode, include the selected channel if it exists
	let displayChannels = $derived(() => {
		const baseChannels = channels || searchResults;
		if (searchMode && selectedChannel && !baseChannels.find(c => c.channel_id === selectedChannel?.channel_id)) {
			return [selectedChannel, ...baseChannels];
		}
		return baseChannels;
	})

	// Create separate filter text for search mode
	let searchFilterText = $state('')

	// Debounced search function
	const debouncedSearch = debounce(async (query: string) => {
		await searchChannels(query)
	}, 500)

	// Watch for search filter text changes (only in search mode)
	$effect(() => {
		if (searchMode) {
			if (searchFilterText.length >= 1) {
				debouncedSearch.debounced(searchFilterText)
			} else if (searchFilterText.length === 0) {
				searchResults = []
				hasSearched = false
			}
		}
	})

	async function searchChannels(query: string) {
		if (!query || !teamId) return

		isFetching = true
		hasSearched = true
		try {
			const response = await WorkspaceService.listAvailableTeamsChannels({
				workspace: $workspaceStore!,
				teamId: teamId,
				search: query
			})

			searchResults = response || []
			isFetching = false
			console.log('Channels found:', searchResults.length, 'for query:', query)
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
					items={searchFilterText.length >= 1 || (searchFilterText.length === 0 && selectedChannel) ? displayChannels().filter(channel => channel.channel_id && channel.channel_name).map((channel) => ({
						label: channel.channel_name ?? 'Unknown Channel',
						value: channel.channel_id ?? ''
					})) : []}
					placeholder={isFetching ? "Searching..." : (teamId ? "Search channels..." : "Select a team first")}
					clearable
					disabled={disabled || isFetching || !teamId}
					bind:filterText={searchFilterText}
					bind:value={
						() => selectedChannel?.channel_id,
						(value) => {
							selectedChannel = value ? displayChannels().find((channel) => channel.channel_id === value) : undefined
						}
					}
				/>
			{:else}
				<Select
					containerStyle={'min-width: ' + minWidth}
					items={displayChannels().filter(channel => channel.channel_id && channel.channel_name).map((channel) => ({
						label: channel.channel_name ?? 'Unknown Channel',
						value: channel.channel_id ?? ''
					}))}
					{placeholder}
					clearable
					disabled={disabled || displayChannels.length === 0}
					bind:value={
						() => selectedChannel?.channel_id,
						(value) => {
							selectedChannel = value ? displayChannels().find((channel) => channel.channel_id === value) : undefined
						}
					}
				/>
			{/if}
		</div>
	</div>

</div>
