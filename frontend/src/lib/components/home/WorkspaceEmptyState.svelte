<script lang="ts">
	import { onMount } from 'svelte'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import type { HubProjectPick } from '$lib/hubProject'
	import { disableHubStore } from '$lib/stores'
	import CreateActionsMenu from './CreateActionsMenu.svelte'
	import HubTemplatePicker from './HubTemplatePicker.svelte'

	interface Props {
		/** A project was chosen here. The list owns the import dialog, and opens it on this. */
		onPick: (project: HubProjectPick) => void
		/**
		 * The workspace holds items, all of them archived. It is not empty and must not be
		 * told it is: the caption names what is there and offers the way to it instead.
		 */
		archivedOnly?: boolean
		/** Switches the list to the archived view. */
		onShowArchived: () => void
		/**
		 * Whether to offer the template import and the create menu. Neither checks permissions
		 * itself, so an operator gets the state described without the two actions it may not
		 * take — the archived link stays, since reading archived items is not a write.
		 */
		canCreate?: boolean
	}

	let { onPick, archivedOnly = false, onShowArchived, canCreate = true }: Props = $props()

	// Row opacities: the list fading out of existence. Static on purpose — motion is what
	// makes a skeleton mean "loading", and this state means "empty".
	// `border-border-*`, not `border-*`: the Tailwind colour keys are themselves named
	// `border-light` / `border-normal`, so `border-light` matches no utility and silently
	// falls back to the global default border colour in app.css. Bars and dashes both sit on
	// `border-light`; only the container outline steps up, so nothing outweighs its frame.
	const rowOpacities = [1, 0.7, 0.4]

	// The inline "create a new one" link is the anchor for the very same New menu the
	// toolbar button opens, so the menu pops next to the words that promised it.
	let newLinkEl: HTMLButtonElement | undefined = $state(undefined)

	onMount(() => {
		// Only the empty case: the counter answers how many workspaces sit empty and what
		// their owners do next, and a workspace whose items are all archived is neither.
		if (!archivedOnly) logFeatureUsage('home', 'empty_state_view')
	})

	// The catalogue is fetched when the picker opens, never on render: `disable_hub` says an
	// instance makes no hub requests at all, and it loads asynchronously, so anything fired
	// from here goes out before the setting that forbids it is known.
</script>

<div
	class="rounded-md border-[1.5px] border-dashed border-border-normal/60 bg-surface"
	role="status"
	aria-label={archivedOnly ? 'Everything here is archived' : 'Your workspace is empty'}
>
	{#each rowOpacities as opacity, i (i)}
		<div
			aria-hidden="true"
			class="flex items-center gap-[14px] px-4 py-[13px] {i > 0
				? 'border-t border-dashed border-border-light'
				: ''}"
			style="opacity: {opacity}"
		>
			<div class="size-4 shrink-0 rounded bg-border-light"></div>
			<div>
				<div class="h-[9px] w-[140px] rounded-full bg-border-light"></div>
				<div class="mt-[5px] h-[7px] w-[70px] rounded-full bg-border-light/60"></div>
			</div>
		</div>
	{/each}

	<!-- A <div>, not a <p>: CreateActionsMenu wraps its trigger in an element, which a
	     paragraph may not contain. -->
	<div
		class="border-t border-dashed border-border-light px-4 pb-[22px] pt-[18px] text-center text-[13.5px] leading-relaxed text-hint"
	>
		{#if archivedOnly}
			<!-- Its own line: the state and the invitation are two sentences, and splicing them
			     into one leaves a link doing the work of a conjunction. -->
			<span class="block">
				Everything in this workspace is archived.
				<button
					class="border-b border-transparent text-accent hover:border-accent"
					onclick={onShowArchived}>Show archived items</button
				>.
			</span>
		{:else}
			Your scripts, flows and apps will show up here.
		{/if}
		{#if canCreate}
			<!-- The hub half goes when the instance has the hub turned off, and the remaining link
		     opens the sentence instead of continuing it. -->
			{#if !$disableHubStore}
				<!-- Opens downward into the page rather than upward into the hero: the caption sits
			     high when the AI composer is hidden, so the room is below it. `fitViewport` caps
			     the box on a short viewport, which is why the height below is definite and the
			     list inside fills it — a squeezed box with a fixed-height list inside overflows
			     its own frame. -->
				<Popover
					floatingConfig={{
						placement: 'bottom',
						strategy: 'absolute',
						gutter: 8,
						overflowPadding: 16,
						flip: { fallbackPlacements: ['top', 'bottom-start', 'top-start'] },
						fitViewport: true,
						overlap: false
					}}
					contentClasses="p-0 flex"
					contentStyle="height: min(72vh, 520px);"
					class="border-b border-transparent text-accent hover:border-accent"
					triggerAttrs={{ 'aria-label': 'Start from a template' }}
					on:openChange={(e) =>
						e.detail && logFeatureUsage('home', 'template_picker_open', { key: 'empty_state' })}
				>
					{#snippet trigger()}Start from a template{/snippet}
					{#snippet content({ close })}
						<HubTemplatePicker
							onPick={(project) => {
								close()
								onPick(project)
							}}
						/>
					{/snippet}
				</Popover>
				or
			{/if}
			<CreateActionsMenu source="empty_state" triggerElement={newLinkEl}>
				{#snippet trigger()}
					<!-- A bare <button> for a link inside a sentence, signed off by design: <Button>
				     carries its own padding and background and cannot sit inline in running text.
				     Inline links take `text-accent`, never a raw Tailwind blue.
				     The full stop rides inside the snippet: across a component boundary Svelte
				     keeps the markup whitespace, which would leave a gap before it. -->
					<button
						bind:this={newLinkEl}
						class="border-b border-transparent text-accent hover:border-accent"
						>{$disableHubStore ? 'Create a new one' : 'create a new one'}</button
					>.
				{/snippet}
			</CreateActionsMenu>
		{/if}
	</div>
</div>
