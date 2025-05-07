<script lang="ts">
	import Select from '$lib/components/apps/svelte-select/lib/Select.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import { createEventDispatcher, onMount } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'

	interface ChannelItem {
		channel_id: string
		channel_name: string
	}

	export let disabled = false
	export let placeholder = 'Select channel'
	export let selectedChannel: ChannelItem | undefined = undefined
	export let containerClass = 'w-64'
	export let minWidth = '160px'
	export let channels: ChannelItem[] = []

	let darkMode: boolean = false

	const dispatch = createEventDispatcher<{
		change: ChannelItem
	}>()

	function onThemeChange() {
		darkMode = document.documentElement.classList.contains('dark')
	}

	onMount(() => {
		onThemeChange()
	})

	function handleChannelSelect(event) {
		selectedChannel = event.detail
		if (selectedChannel) {
			dispatch('change', selectedChannel)
		}
	}

	function filterChannels(filterText: string | unknown) {
		if (!channels) return channels

		const searchText = typeof filterText === 'string' ? filterText : ''

		const filtered = searchText
			? channels.filter((channel) =>
					channel.channel_name.toLowerCase().includes(searchText.toLowerCase())
				)
			: channels

		return filtered
	}
</script>

<div class={containerClass}>
	<div class="flex items-center gap-2">
		<div class="flex-grow" style="min-width: {minWidth};">
			<Select
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				containerStyles={'border-color: lightgray; min-width: ' +
					minWidth +
					';' +
					(darkMode
						? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
						: SELECT_INPUT_DEFAULT_STYLE.containerStyles)}
				itemId="channel_id"
				label="channel_name"
				items={channels || []}
				on:change={handleChannelSelect}
				{placeholder}
				searchable={true}
				disabled={disabled || channels.length === 0}
				on:input={async (e) => {
					const filterText = e.detail
					return filterChannels(filterText)
				}}
			/>
		</div>
	</div>

	{#if channels.length === 0 && !disabled}
		<div class="text-xs text-tertiary mt-1"> No channels available </div>
	{/if}
</div>

<DarkModeObserver on:change={onThemeChange} />
