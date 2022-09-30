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

	const icons: Record<AlertType, IconDefinition> = {
		info: faInfoCircle,
		warning: faWarning,
		error: faWarning,
		success: faCheckCircle
	}

	const classes: Record<AlertType, Record<string, string>> = {
		info: {
			bgClass: 'bg-blue-50 border border-blue-500',
			iconClass: 'text-blue-500',
			titleClass: 'text-blue-800',
			descriptionClass: 'text-blue-700',
			iconBorderClass: 'border-blue-100'
		},
		warning: {
			bgClass: 'bg-yellow-50 border border-yellow-500',
			iconClass: 'text-yellow-500',
			titleClass: 'text-yellow-800',
			descriptionClass: 'text-yellow-700',
			iconBorderClass: 'border-yellow-100'
		},
		error: {
			bgClass: 'bg-red-50 border border-red-500',
			iconClass: 'text-red-500',
			titleClass: 'text-red-800',
			descriptionClass: 'text-red-700',
			iconBorderClass: 'border-red-100'
		},
		success: {
			bgClass: 'bg-green-50 border border-green-500',
			iconClass: 'text-green-500',
			titleClass: 'text-green-800',
			descriptionClass: 'text-green-700',
			iconBorderClass: 'border-green-100'
		}
	}
</script>

<div class={classNames('rounded-md p-4 w-full', classes[type].bgClass)}>
	<div class="flex">
		<div
			class={classNames(
				'flex h-8 w-8 items-center justify-center rounded-full bg-white border',
				classes[type].iconBorderClass
			)}
		>
			<Icon data={icons[type]} class={classes[type].iconClass} />
		</div>

		<div class="ml-2 w-full">
			<span class={classNames('text-sm font-medium', classes[type].titleClass)}>{title}</span>
			<div class={classNames('mt-2 text-sm', classes[type].descriptionClass)}>
				<slot />
			</div>
		</div>
	</div>
</div>
