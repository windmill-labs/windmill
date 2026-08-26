<script lang="ts">
	import type { Snippet } from 'svelte'
	import { Folder, Tag } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { labelBadgeColor } from './labelColors'
	import { labelColorCache, labelColorOf } from './labelStore'

	interface Props {
		label: string
		/** The workspace the label belongs to — not necessarily the one being navigated. */
		workspace: string | undefined
		/** Marks a label the item gets from its folder. Renders the folder icon. */
		inherited?: boolean
		tagIcon?: boolean
		size?: 'verySmall' | 'small' | 'large'
		clickable?: boolean
		selected?: boolean
		title?: string
		class?: string
		onclick?: (event: MouseEvent) => void
		/** Trailing content inside the chip, e.g. a remove button. */
		children?: Snippet
	}

	let {
		label,
		workspace,
		inherited = false,
		tagIcon = false,
		size = 'small',
		clickable = false,
		selected = false,
		title,
		class: clazz = '',
		onclick,
		children
	}: Props = $props()

	let color = $derived(labelBadgeColor(labelColorOf($labelColorCache, workspace, label)))
</script>

<Badge
	{color}
	small={size === 'small'}
	verySmall={size === 'verySmall'}
	large={size === 'large'}
	{selected}
	{clickable}
	{onclick}
	class={twMerge('px-1', clazz)}
	title={title ?? (inherited ? `Label inherited from folder: ${label}` : `Label: ${label}`)}
>
	{#if inherited}
		<Folder size={10} class="mr-0.5 shrink-0" />
	{:else if tagIcon}
		<Tag size={10} class="inline -mt-px shrink-0" />
	{/if}
	{label}
	{@render children?.()}
</Badge>
