<script lang="ts">
	import { faBarsStaggered } from '@fortawesome/free-solid-svg-icons'
	import { Icon } from 'svelte-awesome'

	export let color: string = 'black'
</script>

<Icon data={faBarsStaggered} scale={0.8} style={`color: ${color};`} />
