<script lang="ts">
	import { ChevronRight, KeyRound } from 'lucide-svelte'
	import Popover from '../Popover.svelte'
	import { accountSetup } from './accountSetup.svelte'

	/**
	 * Shown while the account has no credentials of its own: one that came in through a
	 * single-use sign-in link has no way back once this session ends, so it stays until
	 * that is fixed and has no dismiss. The host decides whether it is shown; the row opens
	 * the same modal the Settings entry does.
	 *
	 * Shaped like the rows under it — the two-line menu row's height, padding, gutter and
	 * radius — and tinted, not filled: the accent selection surface with accent text says
	 * "this one, act on it" in the rail's own vocabulary, where a filled block would outshout
	 * every real action on the page.
	 */
	interface Props {
		isCollapsed: boolean
	}
	let { isCollapsed }: Props = $props()

	// Same words as the Settings entry; both lines are measured to fit the narrowest
	// expanded rail without truncating.
	const label = 'Finish account setup'
	const reason = 'No sign-in method yet'
</script>

<Popover appearTimeout={0} disappearTimeout={0} class="w-full" disablePopup={!isCollapsed}>
	<button
		class="group relative flex w-full items-center gap-2 rounded-md border border-border-selected/50 bg-surface-accent-selected px-2 text-left transition-colors hover:border-border-selected focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300 {isCollapsed
			? 'h-8 justify-center'
			: 'h-10'}"
		aria-label={label}
		title={isCollapsed ? undefined : label}
		onclick={() => (accountSetup.open = true)}
	>
		<KeyRound size={16} class="shrink-0 text-accent" />
		{#if !isCollapsed}
			<span class="flex min-w-0 grow flex-col">
				<span class="w-full truncate whitespace-pre text-xs font-semibold text-accent">{label}</span
				>
				<span class="w-full truncate whitespace-pre text-2xs font-normal text-secondary"
					>{reason}</span
				>
			</span>
			<ChevronRight
				size={14}
				class="shrink-0 text-accent opacity-60 transition group-hover:translate-x-0.5 group-hover:opacity-100"
			/>
		{:else}
			<!-- No words in the collapsed rail, so the pulse carries "waiting on you", placed
			     where the rail puts its other badges. -->
			<span class="absolute right-1 top-1 flex h-2 w-2">
				<span
					class="absolute inline-flex h-full w-full animate-ping rounded-full bg-surface-accent-primary opacity-75"
				></span>
				<span class="relative inline-flex h-2 w-2 rounded-full bg-surface-accent-primary"></span>
			</span>
		{/if}
	</button>
	{#snippet text()}
		{label}
	{/snippet}
</Popover>
