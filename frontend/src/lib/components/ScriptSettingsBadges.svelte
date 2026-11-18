<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import {
		getActiveScriptSettingsBadges,
		type ScriptAdvancedSettingsFields
	} from './scriptSettings'

	interface Props {
		settings: ScriptAdvancedSettingsFields | undefined
		// When provided, badges become clickable and call this with the badge key
		// (e.g. to open the settings drawer focused on that section).
		onclick?: (key: string) => void
		small?: boolean
		/** On the row that holds the badges — so a script with none of these settings leaves no
		 * gap where they would have been. */
		class?: string
	}

	let { settings, onclick, small = true, class: className = '' }: Props = $props()

	let badges = $derived(getActiveScriptSettingsBadges(settings))
</script>

{#if badges.length > 0}
	<div class={twMerge('flex flex-row flex-wrap gap-1 items-center', className)}>
		{#each badges as badge (badge.key)}
			<!-- Icon-only chips to save space in crowded top bars; the label and current
				value are shown in the hover popover. -->
			<Popover notClickable placement="bottom">
				<Badge
					color="blue"
					{small}
					icon={{ icon: badge.icon, position: 'left' }}
					clickable={Boolean(onclick)}
					onclick={onclick ? () => onclick?.(badge.key) : undefined}
					aria-label={`${badge.label}: ${badge.detail}`}
				/>
				{#snippet text()}
					<span class="font-semibold">{badge.label}</span> — {badge.detail}
				{/snippet}
			</Popover>
		{/each}
	</div>
{/if}
