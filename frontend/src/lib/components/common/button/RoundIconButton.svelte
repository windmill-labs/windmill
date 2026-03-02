<script lang="ts">
	interface Props {
		color?: 'light' | 'dark' | undefined;
		border?: boolean;
		children?: import('svelte').Snippet;
	}

	let { color = 'light', border = false, children }: Props = $props();

	let colorClasses = $derived({
		light: 'text-primary bg-surface hover:bg-surface-hover text-primary',
		dark: 'text-primary hover:bg-surface-hover-dark text-primary'
	}[color ?? 'light'])
</script>

<div class="rounded-full p-1 {colorClasses} {border ? 'border border-tertiary' : ''}">
	{@render children?.()}
</div>
