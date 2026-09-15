<script lang="ts">
	import { KeyRound } from 'lucide-svelte'
	import MenuButton from './MenuButton.svelte'
	import { accountSetup } from './accountSetup.svelte'

	/**
	 * Shown while the account has no credentials of its own: one that came in through a
	 * single-use sign-in link has no way back once this session ends, so it stays until
	 * that is fixed and has no dismiss. The host decides whether it is shown; the row opens
	 * the same modal the Settings entry does.
	 *
	 * A menu row like the ones under it, tinted rather than filled: the accent selection
	 * surface with accent text says "this one, act on it" in the rail's own vocabulary,
	 * where a filled block would outshout every real action on the page.
	 */
	interface Props {
		isCollapsed: boolean
	}
	let { isCollapsed }: Props = $props()

	// Same words as the Settings entry; both lines fit the narrowest expanded rail.
	const label = 'Finish account setup'
	const reason = 'No sign-in method yet'
</script>

<div class="relative">
	<MenuButton
		{label}
		sublabel={isCollapsed ? undefined : reason}
		icon={KeyRound}
		iconClasses="text-accent"
		labelClass="text-accent font-semibold"
		buttonClass="border border-border-selected/50 bg-surface-accent-selected hover:bg-surface-accent-selected hover:border-border-selected"
		{isCollapsed}
		stopPropagationOnClick
		on:click={() => (accountSetup.open = true)}
	/>
	{#if isCollapsed}
		<!-- No words in the collapsed rail, so the pulse carries "waiting on you", placed
		     where the rail puts its other badges. -->
		<span class="pointer-events-none absolute right-1 top-1 flex h-2 w-2">
			<span
				class="absolute inline-flex h-full w-full animate-ping rounded-full bg-surface-accent-primary opacity-75"
			></span>
			<span class="relative inline-flex h-2 w-2 rounded-full bg-surface-accent-primary"></span>
		</span>
	{/if}
</div>
