<script lang="ts">
	import { type AlertType, classes } from './model'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import {
		AlertCircle,
		AlertTriangle,
		CheckCircle2,
		Info,
		ChevronDown,
		ChevronUp
	} from 'lucide-svelte'
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
		iconStyle?: string | undefined
		titleClass?: string | undefined
		titleStyle?: string | undefined
		descriptionClass?: string | undefined
		descriptionStyle?: string | undefined
		class?: string | undefined
		isCollapsed?: boolean
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
		iconStyle = undefined,
		titleClass = undefined,
		titleStyle = undefined,
		descriptionClass = undefined,
		descriptionStyle = undefined,
		class: classNames = undefined,
		isCollapsed = $bindable(true),
		children
	}: Props = $props()

	const icons: Record<AlertType, any> = {
		info: Info,
		warning: AlertCircle,
		error: AlertTriangle,
		success: CheckCircle2
	}

	function toggleCollapse() {
		if (collapsible) {
			isCollapsed = !isCollapsed
		}
	}

	const SvelteComponent = $derived(icons[type])
</script>

<div
	class={twMerge(
		notRounded ? '' : 'rounded-md',
		size === 'sm' ? 'p-4' : 'p-2',
		classes[type].bgClass,
		bgClass,
		classNames
	)}
	style={bgStyle}
>
	<div class="flex">
		<div class="flex h-8 w-8 items-center justify-center rounded-full">
			<SvelteComponent
				class={twMerge(classes[type].iconClass, iconClass)}
				style={iconStyle}
				size={16}
			/>
		</div>
		<div class={twMerge('ml-1 w-full')}>
			<div class={twMerge('w-full flex flex-row items-center justify-between h-8')}>
				<span
					class={twMerge(
						size === 'sm' ? 'text-sm' : 'text-xs',
						'font-medium',
						classes[type].titleClass,
						titleClass
					)}
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

			{#if children && !isCollapsed}
				<div transition:slide|local={{ duration: 200 }} class="mt-2">
					<div
						class={twMerge(
							size === 'sm' ? 'text-sm' : 'text-xs',
							classes[type].descriptionClass,
							descriptionClass
						)}
						style={descriptionStyle}
					>
						{@render children?.()}
					</div>
				</div>
			{:else if children && !collapsible}
				<div class="mb-2">
					<div
						class={twMerge(
							size === 'sm' ? 'text-sm' : 'text-xs',
							classes[type].descriptionClass,
							descriptionClass
						)}
						style={descriptionStyle}
					>
						{@render children?.()}
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>
