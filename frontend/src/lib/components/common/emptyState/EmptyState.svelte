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
			/**
			 * `default` unless the surface has no other live call to action. Accent is for the case
			 * where this button is the only thing to press — a form whose submit is disabled until
			 * this is done, say — so it is not competing with one.
			 */
			variant?: 'default' | 'accent'
			/** Same write lock the surface's other controls take. An empty state is still a live
			 *  control: without this it stays clickable while a request that has already read the
			 *  empty list is in flight, and whatever it adds is discarded when that request lands. */
			disabled?: boolean
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
		<!-- `default` by default: the page header usually carries the accent CTA for this same
			 action, and one accent per view is the rule. A caller whose other CTA is disabled until
			 this is done opts into `accent`. -->
		<Button
			unifiedSize="md"
			variant={action.variant ?? 'default'}
			disabled={action.disabled}
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
