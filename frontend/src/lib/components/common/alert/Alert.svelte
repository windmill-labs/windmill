<script lang="ts">
	import { classNames } from '$lib/utils'
	import {
		faCheckCircle,
		faInfoCircle,
		faWarning,
		type IconDefinition
	} from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import type { AlertType } from './model'

	export let type: AlertType = 'info'
	export let title: string

	export let size: 'xs' | 'sm' = 'sm'

	const icons: Record<AlertType, IconDefinition> = {
		info: faInfoCircle,
		warning: faWarning,
		error: faWarning,
		success: faCheckCircle
	}

	const classes: Record<AlertType, Record<string, string>> = {
		info: {
			bgClass: 'bg-blue-50 border-blue-200 border',
			iconClass: 'text-blue-500',
			titleClass: 'text-blue-800',
			descriptionClass: 'text-blue-700'
		},
		warning: {
			bgClass: 'bg-yellow-50 border-yellow-200 border',
			iconClass: 'text-yellow-500',
			titleClass: 'text-yellow-800',
			descriptionClass: 'text-yellow-700'
		},
		error: {
			bgClass: 'bg-red-50 border-red-200 border',
			iconClass: 'text-red-500',
			titleClass: 'text-red-800',
			descriptionClass: 'text-red-700'
		},
		success: {
			bgClass: 'bg-green-50 border-green-200 border',
			iconClass: 'text-green-500',
			titleClass: 'text-green-800',
			descriptionClass: 'text-green-700'
		}
	}
</script>

<div
	class={classNames(
		'rounded-md',
		size === 'sm' ? 'p-4' : 'p-2 ',
		classes[type].bgClass,
		$$props.class
	)}
>
	<div class="flex">
		<div class="flex h-8 w-8 items-center justify-center rounded-full">
			<Icon data={icons[type]} class={classes[type].iconClass} />
		</div>

		<div class="ml-2 w-full">
			<span
				class={classNames(
					size === 'sm' ? 'text-sm' : 'text-xs ',
					'font-medium',
					classes[type].titleClass
				)}
			>
				{title}
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
