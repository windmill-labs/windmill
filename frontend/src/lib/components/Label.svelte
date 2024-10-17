<script lang="ts">
	export let label: string | undefined = undefined
	export let primary = false
</script>

<div class={$$props.class}>
	<div class="flex flex-row justify-between items-center w-full">
		<div class="flex flex-row items-center gap-1">
			<span class="{primary ? 'text-primary' : 'text-secondary'} text-sm leading-6">{label}</span>
			<slot name="header" />
		</div>
		<slot name="error" />
		<slot name="action" />
	</div>
	<slot />
</div>
