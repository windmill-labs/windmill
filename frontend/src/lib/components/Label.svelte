<script lang="ts">
	export let label: string | undefined = undefined
</script>

<label>
	<div class="flex flex-row justify-between items-center">
		<div class="flex flex-row items-center gap-2">
			<span class="text-secondary text-sm leading-6">{label}</span>
			<slot name="header" />
		</div>
		<slot name="error" />
	</div>
	<slot />
</label>
