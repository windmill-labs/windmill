<!--
@component
A one-line note across the top of a pane: why the pane behaves as it does, with at most one
action that changes it. Lighter than an `Alert`, which is a box for a message of its own rather
than a remark about the content under it.
-->
<script lang="ts">
	import { Info } from 'lucide-svelte'
	import Button from '../button/Button.svelte'

	interface Props {
		icon?: typeof Info
		action?: { label: string; onClick: () => void }
		children: import('svelte').Snippet
	}

	let { icon: Icon = Info, action = undefined, children }: Props = $props()
</script>

<div class="shrink-0 flex items-center gap-2 px-4 py-1.5 border-b text-2xs text-secondary">
	<Icon size={16} class="shrink-0" />
	<span class="flex-1">{@render children()}</span>
	{#if action}
		<Button unifiedSize="2xs" variant="subtle" onClick={action.onClick}>{action.label}</Button>
	{/if}
</div>
