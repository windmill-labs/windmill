<script lang="ts">
	import type { Snippet } from 'svelte'
	import Button from '../button/Button.svelte'

	interface Props {
		icon: any
		title: string
		description?: string
		/**
		 * Accent create button. A page that renders one here should hide the page-header
		 * button for the same action while it shows, so the two accent CTAs don't sit on
		 * screen together — and must derive that from the very conditions gating this
		 * `action`, or a state where the card has no button leaves no way to create at all.
		 */
		action?: {
			label: string
			icon?: any
			onClick: () => void
			aiId?: string
			aiDescription?: string
		}
		children?: Snippet
	}

	let { icon: Icon, title, description, action, children }: Props = $props()
</script>

<div
	class="flex flex-col items-center justify-center gap-4 rounded-lg border border-dashed border-border-light px-6 py-16 text-center"
>
	<div class="text-secondary">
		<Icon size={32} />
	</div>
	<div class="flex flex-col gap-1 max-w-md">
		<div class="text-sm font-semibold text-emphasis">{title}</div>
		{#if description}
			<div class="text-xs text-secondary">{description}</div>
		{/if}
	</div>
	{#if action}
		<Button
			unifiedSize="md"
			variant="accent"
			startIcon={action.icon ? { icon: action.icon } : undefined}
			onClick={action.onClick}
			aiId={action.aiId}
			aiDescription={action.aiDescription}
		>
			{action.label}
		</Button>
	{/if}
	{@render children?.()}
</div>
