<script lang="ts">
	import { type AlertAction, type AlertType, classes, icons } from './model'
	import Button from '../button/Button.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { ChevronDown, ChevronUp } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		type?: AlertType
		title: string
		notRounded?: boolean
		tooltip?: string
		documentationLink?: string | undefined
		size?: 'xs' | 'sm'
		collapsible?: boolean
		bgClass?: string | undefined
		bgStyle?: string | undefined
		iconClass?: string | undefined
		/** Drop the type's icon; the text starts at the alert's edge. */
		hideIcon?: boolean
		iconStyle?: string | undefined
		titleClass?: string | undefined
		titleStyle?: string | undefined
		descriptionClass?: string | undefined
		descriptionStyle?: string | undefined
		class?: string | undefined
		isCollapsed?: boolean
		/** Buttons under the body, colored to match the alert's type. They stay visible while a
		 * collapsible alert is collapsed. */
		actions?: AlertAction[]
		children?: import('svelte').Snippet
	}

	let {
		type = 'info',
		title,
		notRounded = false,
		tooltip = '',
		documentationLink = undefined,
		size = 'sm',
		collapsible = false,
		bgClass = undefined,
		bgStyle = undefined,
		iconClass = undefined,
		hideIcon = false,
		iconStyle = undefined,
		titleClass = undefined,
		titleStyle = undefined,
		descriptionClass = undefined,
		descriptionStyle = undefined,
		class: classNames = undefined,
		isCollapsed = $bindable(true),
		actions = [],
		children
	}: Props = $props()

	function toggleCollapse() {
		if (collapsible) {
			isCollapsed = !isCollapsed
		}
	}

	const SvelteComponent = $derived(icons[type])

	// A blank title would still occupy a text line and push the body down, leaving an alert
	// that is visibly top-heavy. Body-only alerts skip the row, and the gap under it, entirely.
	const hasTitleRow = $derived(!!title || collapsible || tooltip != '' || !!documentationLink)
</script>

<div
	class={twMerge(
		notRounded ? '' : 'rounded-md',
		size === 'sm' ? 'px-3 py-2' : 'px-2.5 py-1.5',
		classes[type].bgClass,
		bgClass,
		classNames
	)}
	style={bgStyle}
>
	<div class="flex flex-row items-center">
		<!-- shrink-0: the badge's min-content width is the 16px icon, so without it a
		     narrow container squeezes the circle into an oval. -->
		{#if !hideIcon}
			<div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full">
				<SvelteComponent
					class={twMerge(classes[type].iconClass, iconClass)}
					style={iconStyle}
					size={16}
				/>
			</div>
		{/if}
		<div class={twMerge(!hideIcon && 'ml-1', 'w-full')}>
			{#if hasTitleRow}
				<div class={twMerge('w-full flex flex-row items-center justify-between')}>
					<span
						class={twMerge('text-xs font-semibold', classes[type].titleClass, titleClass)}
						style={titleStyle}
					>
						{title}
						{#if tooltip != '' || documentationLink}
							<Tooltip {documentationLink}>{tooltip}</Tooltip>
						{/if}
					</span>
					{#if collapsible}
						<button class="cursor-pointer" onclick={toggleCollapse}>
							{#if isCollapsed}
								<ChevronDown size={16} />
							{:else}
								<ChevronUp size={16} />
							{/if}
						</button>
					{/if}
				</div>
			{/if}

			{#if children && (!collapsible || !isCollapsed)}
				<div
					transition:slide|local={{ duration: 200 }}
					class={twMerge(
						'text-xs',
						hasTitleRow ? 'mt-1' : '',
						classes[type].descriptionClass,
						descriptionClass
					)}
					style={descriptionStyle}
				>
					{@render children?.()}
				</div>
			{/if}

			{#if actions.length > 0}
				<div class={twMerge('flex flex-wrap gap-2', hasTitleRow || children ? 'mt-2' : '')}>
					{#each actions as action}
						<Button
							unifiedSize={size === 'sm' ? 'sm' : 'xs'}
							variant={action.variant ?? 'default'}
							tone={type}
							startIcon={action.startIcon}
							disabled={action.disabled}
							loading={action.loading}
							onClick={action.onClick}
						>
							{action.label}
						</Button>
					{/each}
				</div>
			{/if}
		</div>
	</div>
</div>
