<script lang="ts">
	export let title: string
</script>

<div class="p-4 flex flex-col gap-2 items-start">
	<div class="text-sm font-bold">{title}</div>
	<slot />
</div>
