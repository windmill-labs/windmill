<script lang="ts">
	export let kbdClass = ''
	export let small = false
	if (small) {
		kbdClass += ' text-5xs -my-0.5'
	}
</script>

<span
	class="{$$props.class} ml-0.5 rounded border bg-white/70 px-1.5 !text-xs text-gray-600 shadow-sm font-light transition-all group-hover:border-primary-500 group-hover:text-primary-500"
>
	<kbd class={kbdClass}>
		<slot />
	</kbd>
</span>
