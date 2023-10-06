<script lang="ts">
	export let label: string | undefined = undefined
</script>

<label>
	<div class="flex flex-row justify-between items-center">
		<span class="text-secondary text-sm leading-6">{label}</span>
		<slot name="error" />
	</div>
	<slot />
</label>
