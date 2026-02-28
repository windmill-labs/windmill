<script lang="ts">
	/**
	 * @deprecated Use `$lib/components/meltComponents/Tooltip.svelte` instead.
	 * This legacy tooltip component will be removed in a future version.
	 */
	import type { PopoverPlacement } from './Popover.model'
	import Popover from './Popover.svelte'
	import { InfoIcon } from 'lucide-svelte'
	import { getContext, hasContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	interface Props {
		light?: boolean
		wrapperClass?: string
		placement?: PopoverPlacement | undefined
		documentationLink?: string | undefined
		small?: boolean
		markdownTooltip?: string | undefined
		customSize?: string
		class?: string
		Icon?: typeof InfoIcon
		children?: import('svelte').Snippet
	}

	let {
		light = false,
		wrapperClass = '',
		placement = undefined,
		documentationLink = undefined,
		small = false,
		markdownTooltip = undefined,
		customSize = '100%',
		class: classNames = '',
		Icon = InfoIcon,
		children
	}: Props = $props()

	const disableTooltips = hasContext('disableTooltips')
		? getContext('disableTooltips') === true
		: false

	let markdownModule: Promise<{ default: typeof import('svelte-exmarkdown').default; gfmPlugin: typeof import('svelte-exmarkdown/gfm').gfmPlugin }> | undefined = $state()
	$effect(() => {
		if (markdownTooltip && !markdownModule) {
			markdownModule = Promise.all([
				import('svelte-exmarkdown'),
				import('svelte-exmarkdown/gfm')
			]).then(([md, gfm]) => ({ default: md.default, gfmPlugin: gfm.gfmPlugin }))
		}
	})
</script>

{#if disableTooltips !== true}
	<Popover
		notClickable
		{placement}
		class={twMerge(wrapperClass)}
		style="transform: scale({parseFloat(customSize) / 100});"
		{documentationLink}
	>
		<div
			class="inline-flex w-3 mx-0.5 h-3 {light
				? 'text-primary-inverse'
				: 'text-primary'} {classNames} relative"
		>
			<Icon class="{small ? 'bottom-0' : '-bottom-0.5'} absolute" size={small ? 12 : 14} />
		</div>
		{#snippet text()}
			{#if markdownTooltip && markdownModule}
				{#await markdownModule then { default: Markdown, gfmPlugin }}
					<div class="prose-sm">
						<Markdown md={markdownTooltip} plugins={[gfmPlugin()]} />
					</div>
				{/await}
			{:else}
				{@render children?.()}
			{/if}
		{/snippet}
	</Popover>
{/if}
