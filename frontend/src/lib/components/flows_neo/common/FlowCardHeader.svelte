<script lang="ts">
	export let title: string

	/**
	 * TODO:
	 *  - text font and color

	 */
</script>

<div class="flex items-center justify-between flex-wrap py-2 px-4 border-b">
	<span class="text-sm">{title}</span>
	<slot />
</div>
