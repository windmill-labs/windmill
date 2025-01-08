<script lang="ts">
	export let color: 'light' | 'dark' | undefined = 'light'
	export let border: boolean = false

	$: colorClasses = {
		light: 'text-tertiary bg-surface hover:bg-surface-hover text-tertiary',
		dark: 'text-tertiary hover:bg-surface-hover-dark text-tertiary'
	}[color ?? 'light']
</script>

<div class="rounded-full p-1 {colorClasses} {border ? 'border border-tertiary' : ''}">
	<slot />
</div>
