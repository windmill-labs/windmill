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

	export let type: AlertType = 'info'
	export let title: string
	export let notRounded = false
	export let tooltip: string = ''
	export let documentationLink: string | undefined = undefined
	export let size: 'xs' | 'sm' = 'sm'
	export let collapsible: boolean = false

	export let bgClass: string | undefined = undefined
	export let bgStyle: string | undefined = undefined
	export let iconClass: string | undefined = undefined
	export let iconStyle: string | undefined = undefined
	export let titleClass: string | undefined = undefined
	export let titleStyle: string | undefined = undefined
	export let descriptionClass: string | undefined = undefined
	export let descriptionStyle: string | undefined = undefined

	export let isCollapsed = true

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
</script>

<div
	class={twMerge(
		notRounded ? '' : 'rounded-md',
		size === 'sm' ? 'p-4' : 'p-2',
		classes[type].bgClass,
		bgClass,
		$$props.class
	)}
	style={bgStyle}
>
	<div class="flex">
		<div class="flex h-8 w-8 items-center justify-center rounded-full">
			<svelte:component
				this={icons[type]}
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
						<Tooltip {documentationLink} scale={0.9}>{tooltip}</Tooltip>
					{/if}
				</span>
				{#if collapsible}
					<button class="cursor-pointer" on:click={toggleCollapse}>
						{#if isCollapsed}
							<ChevronDown size={16} />
						{:else}
							<ChevronUp size={16} />
						{/if}
					</button>
				{/if}
			</div>

			{#if $$slots.default && !isCollapsed}
				<div transition:slide|local={{ duration: 200 }} class="mt-2">
					<div
						class={twMerge(
							size === 'sm' ? 'text-sm' : 'text-xs',
							classes[type].descriptionClass,
							descriptionClass
						)}
						style={descriptionStyle}
					>
						<slot />
					</div>
				</div>
			{:else if $$slots.default && !collapsible}
				<div class="mb-2">
					<div
						class={twMerge(
							size === 'sm' ? 'text-sm' : 'text-xs',
							classes[type].descriptionClass,
							descriptionClass
						)}
						style={descriptionStyle}
					>
						<slot />
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>
