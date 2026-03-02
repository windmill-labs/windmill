<script lang="ts">
	export let color: 'light' | 'dark' | undefined = 'light'
	export let border: boolean = false

	$: colorClasses = {
		light: 'text-primary bg-surface hover:bg-surface-hover text-primary',
		dark: 'text-primary hover:bg-surface-hover-dark text-primary'
	}[color ?? 'light']
</script>

<div class="rounded-full p-1 {colorClasses} {border ? 'border border-tertiary' : ''}">
	<slot />
</div>
