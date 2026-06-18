<script lang="ts">
	import { QrCode } from 'lucide-svelte'

	interface Props {
		height?: number;
	}

	let { height = 24 }: Props = $props();
</script>

<QrCode size={height} />
