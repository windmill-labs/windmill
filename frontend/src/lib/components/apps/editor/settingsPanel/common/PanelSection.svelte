<script lang="ts">
	export let title: string
</script>

<div class="p-4 flex flex-col gap-2 items-start">
	<div class="flex justify-between items-center w-full">
		<div class="text-xs font-bold">{title}</div>
		<slot name="action" />
	</div>
	<slot />
</div>
