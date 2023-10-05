<script lang="ts">
	export let label: string | undefined = undefined
</script>

<div>
	<div class="flex flex-row justify-between">
		<h2 class="text-base font-semibold mb-2">
			{label}
			<slot name="header" />
		</h2>
		<slot name="action" />
	</div>

	<slot />
</div>
