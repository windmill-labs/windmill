<script lang="ts">
	import { ArrowRight, KeyRound } from 'lucide-svelte'
	import Popover from '../Popover.svelte'
	import { accountSetup } from './accountSetup.svelte'

	/**
	 * The one accent-filled element in the rail, shown while the account has no credentials
	 * of its own: an account that came in through a single-use sign-in link has no way back
	 * once this session ends, which is why this is louder than a menu entry and has no
	 * dismiss. The host decides whether it is shown; the whole block is the button, and it
	 * opens the same modal the Settings entry does.
	 */
	interface Props {
		isCollapsed: boolean
	}
	let { isCollapsed }: Props = $props()

	// Sized for the rail: the title must not truncate at the narrowest expanded width, and
	// the line under it is the reason in one breath. Same words as the Settings entry.
	const label = 'Finish account setup'
</script>

<Popover appearTimeout={0} disappearTimeout={0} class="w-full" disablePopup={!isCollapsed}>
	<button
		class="group relative flex w-full items-center rounded-md bg-surface-accent-primary text-left text-white transition-colors hover:bg-surface-accent-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300 active:bg-surface-accent-clicked {isCollapsed
			? 'h-8 justify-center'
			: 'gap-2.5 px-2.5 py-2'}"
		aria-label={label}
		onclick={() => (accountSetup.open = true)}
	>
		<KeyRound size={16} class="shrink-0" />
		{#if !isCollapsed}
			<span class="min-w-0 flex-1">
				<span class="block truncate text-xs font-semibold leading-tight">{label}</span>
				<span class="mt-0.5 block text-[11px] leading-snug text-white/80">
					Add a password or sign-in to get back in.
				</span>
			</span>
			<ArrowRight
				size={14}
				class="shrink-0 opacity-70 transition group-hover:translate-x-0.5 group-hover:opacity-100"
			/>
		{/if}
		<!-- The pulse says "waiting on you" the way the Settings entry already does; the
		     colour alone would not, and in the collapsed rail it is all the room there is. -->
		<span class="absolute flex h-2 w-2 {isCollapsed ? 'right-0.5 top-0.5' : 'right-1.5 top-1.5'}">
			<span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-white opacity-60"
			></span>
			<span class="relative inline-flex h-2 w-2 rounded-full bg-white"></span>
		</span>
	</button>
	{#snippet text()}
		{label}
	{/snippet}
</Popover>
