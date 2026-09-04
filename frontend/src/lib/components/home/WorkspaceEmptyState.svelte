<script lang="ts">
	import { onMount } from 'svelte'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { preloadHubProjects, type HubProjectPick } from '$lib/hubProject'
	import { workspaceStore } from '$lib/stores'
	import CreateActionsMenu from './CreateActionsMenu.svelte'
	import HubTemplatePicker from './HubTemplatePicker.svelte'

	interface Props {
		/** A project was chosen here. The list owns the import dialog, and opens it on this. */
		onPick: (project: HubProjectPick) => void
	}

	let { onPick }: Props = $props()

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
		logFeatureUsage('home', 'empty_state_view')
	})

	// Warmed as soon as the empty state renders rather than on the first click: the
	// catalogue is one request for the whole hub, and paying for it here is what makes
	// the picker open on content.
	$effect(() => {
		if ($workspaceStore) preloadHubProjects($workspaceStore)
	})
</script>

<div
	class="rounded-md border-[1.5px] border-dashed border-border-normal/60 bg-surface"
	role="status"
	aria-label="Your workspace is empty"
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
		Your scripts, flows and apps will show up here.
		<!-- Opens downward into the page rather than upward into the hero: the caption sits high
		     when the AI composer is hidden, so the room is below it. `fitViewport` caps the box on
		     a short viewport, which is why the height below is definite and the list inside fills
		     it — a squeezed box with a fixed-height list inside overflows its own frame. -->
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
					>create a new one</button
				>.
			{/snippet}
		</CreateActionsMenu>
	</div>
</div>
