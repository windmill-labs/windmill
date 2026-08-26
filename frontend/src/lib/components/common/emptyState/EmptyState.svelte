<script lang="ts">
	import type { Snippet } from 'svelte'
	import Button from '../button/Button.svelte'

	interface Props {
		icon: any
		title: string
		description?: string
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
		<!-- `default`, not `accent`: the page header already carries the accent CTA for
			 this same action, and one accent per view is the rule. -->
		<Button
			unifiedSize="md"
			variant="default"
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
