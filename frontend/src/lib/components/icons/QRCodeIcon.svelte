<script lang="ts">
	import { QrCode } from 'svelte-lucide'

	export let height = '24px'
	export let width = '24px'
</script>

<QrCode size={height} />
