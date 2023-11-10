<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AlertType } from './model'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { AlertCircle, AlertTriangle, CheckCircle2, Info } from 'lucide-svelte'

	export let type: AlertType = 'info'
	export let title: string
	export let notRounded = false
	export let tooltip: string = ''
	export let documentationLink: string | undefined = undefined

	export let size: 'xs' | 'sm' = 'sm'

	const icons: Record<AlertType, any> = {
		info: Info,
		warning: AlertCircle,
		error: AlertTriangle,
		success: CheckCircle2
	}

	const classes: Record<AlertType, Record<string, string>> = {
		info: {
			bgClass: 'bg-blue-50 border-blue-200 border dark:bg-blue-900/50 dark:border-blue-900',
			iconClass: 'text-blue-500 dark:text-blue-400',
			titleClass: 'text-blue-800 dark:text-blue-100',
			descriptionClass: 'text-blue-700 dark:text-blue-200'
		},
		warning: {
			bgClass: 'bg-yellow-50 border-yellow-200 border dark:bg-yellow-900/50 dark:border-yellow-900',
			iconClass: 'text-yellow-500 dark:text-yellow-400',
			titleClass: 'text-yellow-800 dark:text-yellow-100',
			descriptionClass: 'text-yellow-700 dark:text-yellow-200'
		},
		error: {
			bgClass: 'bg-red-50 border-red-200 border dark:bg-red-900/50 dark:border-red-900',
			iconClass: 'text-red-500 dark:text-red-400',
			titleClass: 'text-red-800 dark:text-red-100',
			descriptionClass: 'text-red-700 dark:text-red-200'
		},
		success: {
			bgClass: 'bg-green-50 border-green-200 border dark:bg-green-900/50 dark:border-green-900',
			iconClass: 'text-green-500 dark:text-green-400',
			titleClass: 'text-green-800 dark:text-green-100',
			descriptionClass: 'text-green-700 dark:text-green-200'
		}
	}
</script>

<div
	class={classNames(
		notRounded ? '' : 'rounded-md',
		size === 'sm' ? 'p-4' : 'p-2 ',
		classes[type].bgClass,
		$$props.class
	)}
>
	<div class="flex">
		<div class="flex h-8 w-8 items-center justify-center rounded-full">
			<svelte:component this={icons[type]} class={classes[type].iconClass} size={16} />
		</div>

		<div class="ml-2 w-full">
			<span
				class={classNames(
					size === 'sm' ? 'text-sm' : 'text-xs ',
					'font-medium',
					classes[type].titleClass
				)}
			>
				{title}&nbsp;{#if tooltip != '' || documentationLink}
					<Tooltip {documentationLink} scale={0.9}>{tooltip}</Tooltip>
				{/if}
			</span>
			<div
				class={classNames(
					size === 'sm' ? 'text-sm' : 'text-xs ',
					'mt-2',
					classes[type].descriptionClass
				)}
			>
				<slot />
			</div>
		</div>
	</div>
</div>
