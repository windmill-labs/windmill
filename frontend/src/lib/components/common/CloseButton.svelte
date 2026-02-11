<script lang="ts">
	import Button from './button/Button.svelte'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		noBg?: boolean
		small?: boolean
		Icon?: any | undefined
		class?: string
		id?: string | undefined
		onClick?: (e: Event) => void
	}

	let { noBg = false, small = false, Icon, class: className, id, onClick }: Props = $props()
</script>

<Button
	{onClick}
	onPointerdown={(e) => e.stopPropagation()}
	{id}
	startIcon={{ icon: Icon ?? X }}
	iconOnly
	unifiedSize="sm"
	color="light"
	wrapperClasses="shrink-0"
	btnClasses={twMerge(
		'hover:bg-surface-hover rounded-full p-0 !min-h-0',
		noBg ? '' : 'bg-surface-secondary',
		small ? 'w-6 h-6' : 'w-8 h-8',
		className ?? ''
	)}
/>
