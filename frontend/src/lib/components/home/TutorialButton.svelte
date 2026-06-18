<script lang="ts">
	import { CheckCircle2, Circle, RefreshCw, CheckCheck } from 'lucide-svelte'
	import type { ComponentType } from 'svelte'

	interface Props {
		icon: ComponentType
		title: string
		description: string
		onclick: () => void
		isCompleted?: boolean
		disabled?: boolean
		comingSoon?: boolean
		onReset?: () => void
		onComplete?: () => void
	}

	let {
		icon: Icon,
		title,
		description,
		onclick,
		isCompleted = false,
		disabled = false,
		comingSoon = false,
		onReset,
		onComplete
	}: Props = $props()

	let isHovered = $state(false)

	// Determine which action button to show
	const actionButton = $derived(() => {
		if (isCompleted && isHovered && onReset) {
			return {
				icon: RefreshCw,
				label: 'Reset',
				onClick: onReset
			}
		}
		if (!isCompleted && isHovered && onComplete) {
			return {
				icon: CheckCheck,
				label: 'Mark as completed',
				onClick: onComplete
			}
		}
		return null
	})

	function handleAction(e: MouseEvent | KeyboardEvent) {
		const button = actionButton()
		if (!button) return
		e.stopPropagation()
		e.preventDefault()
		button.onClick()
	}
</script>

<button
	onclick={disabled || comingSoon ? undefined : onclick}
	disabled={disabled || comingSoon}
	class="group relative flex items-center gap-4 w-full px-4 py-3 first-of-type:!border-t-0 first-of-type:rounded-t-md last-of-type:rounded-b-md [*:not(:last-child)]:border-b border-b border-light transition-colors text-left last:border-b-0 {disabled || comingSoon
		? 'opacity-50 cursor-not-allowed'
		: 'hover:bg-surface-hover'}"
>
	<!-- Icon -->
	<Icon size={20} class="flex-shrink-0 text-accent-primary transition-colors" />

	<!-- Content -->
	<div class="flex-1 min-w-0">
		<div class="text-emphasis flex-wrap text-left text-xs font-semibold {!disabled && !comingSoon
			? 'group-hover:text-accent-primary'
			: ''} transition-colors">
			{title}
			{#if comingSoon}
				<span class="ml-2 text-3xs text-secondary">(Coming soon)</span>
			{/if}
		</div>
		<div class="text-hint text-3xs truncate text-left font-normal">
			{description}
		</div>
	</div>

	<!-- Status -->
	<div
		role="status"
		class="flex items-center gap-1.5 flex-shrink-0"
		onmouseenter={() => (isHovered = true)}
		onmouseleave={() => (isHovered = false)}
	>
		{#if actionButton()}
			{@const button = actionButton()!}
			{@const ActionIcon = button.icon}
			<div
				role="button"
				tabindex="0"
				onclick={handleAction}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						handleAction(e)
					}
				}}
				class="flex items-center gap-1.5 px-2 py-1 text-xs font-normal text-secondary hover:text-primary hover:bg-surface-hover rounded transition-colors cursor-pointer"
			>
				<ActionIcon size={14} class="flex-shrink-0" />
				{button.label}
			</div>
		{:else}
			<span
				class="text-xs font-normal {isCompleted
					? 'text-green-500'
					: 'text-blue-300'}"
			>
				{isCompleted ? 'Completed' : 'Not started'}
			</span>
			{#if isCompleted}
				<CheckCircle2 size={14} class="text-green-500 flex-shrink-0" />
			{:else}
				<Circle size={14} class="text-blue-300 flex-shrink-0" />
			{/if}
		{/if}
	</div>
</button>

