<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	export let color: string = 'black'
</script>

<BarsStaggered size={14} style={`color: ${color};`} />
