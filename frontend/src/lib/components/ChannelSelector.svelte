<script lang="ts">
	import Select from './select/Select.svelte'

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
	}

	let {
		disabled = false,
		placeholder = 'Select channel',
		selectedChannel = $bindable(undefined),
		containerClass = 'w-64',
		minWidth = '160px',
		channels = []
	}: Props = $props()
</script>

<div class={containerClass}>
	<div class="flex items-center gap-2">
		<div class="flex-grow" style="min-width: {minWidth};">
			<Select
				containerStyle={'min-width: ' + minWidth}
				items={channels.map((channel) => ({
					label: channel.channel_name ?? 'Unknown Channel',
					value: channel.channel_id ?? ''
				}))}
				{placeholder}
				bind:value={
					() => selectedChannel?.channel_id,
					(value) => (selectedChannel = channels.find((channel) => channel.channel_id === value))
				}
				clearable
				disabled={disabled || channels.length === 0}
			/>
		</div>
	</div>

	{#if channels.length === 0 && !disabled}
		<div class="text-xs text-tertiary mt-1">No channels available</div>
	{/if}
</div>
