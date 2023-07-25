<script lang="ts">
	export let kbdClass = ''
	export let small = false
	if (small) {
		kbdClass += ' !text-[10px] -my-1 px-1'
	} else {
		kbdClass += ' !text-xs px-1.5'
	}
</script>

<span
	class="{$$props.class} {small
		? 'h-4  center-center'
		: ''} -py-0.5 ml-0.5 rounded border bg-surface-secondary text-primary shadow-sm font-light transition-all group-hover:border-primary-500 group-hover:text-primary-inverse"
>
	<kbd class={kbdClass}>
		<slot />
	</kbd>
</span>
