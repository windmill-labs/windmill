<script lang="ts">
	export let title: string

	/**
	 * TODO:
	 *  - text font and color

	 */
</script>

<div
	class="flex items-center justify-between flex-wrap py-2 px-4 border-b bg-gray-50 shadow-sm h-12"
>
	<span class="text-sm font-bold">{title}</span>
	<slot />
</div>
