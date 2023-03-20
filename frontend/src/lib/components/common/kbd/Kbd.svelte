<script lang="ts">
	export let kbdClass = ''
</script>

<span
	class="{$$props.class} rounded border bg-white/70 px-1.5 !text-xs text-gray-600 shadow-sm font-light transition-all group-hover:border-primary-500 group-hover:text-primary-500"
>
	<kbd class={kbdClass}>
		<slot />
	</kbd>
</span>
