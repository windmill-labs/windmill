<script lang="ts">
	export let label: string | undefined = undefined
</script>

<div>
	<div class="flex flex-row justify-between items-center w-full">
		<div class="flex flex-row items-center gap-1">
			<span class="text-secondary text-sm leading-6">{label}</span>
			<slot name="header" />
		</div>
		<slot name="error" />
		<slot name="action" />
	</div>
	<slot />
</div>
