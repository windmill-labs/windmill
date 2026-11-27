<script lang="ts">
	import mark from './ace-data-cloud.png'

	let { width = 24, height = 24 }: { width?: string | number; height?: string | number } = $props()
	const cssSize = (value: string | number) => (typeof value === 'number' ? value + 'px' : value)
</script>

<!-- Publisher mark from AceDataCloud/ModelProviderDify, _assets/icon.png. -->
<img src={mark} alt="" style:width={cssSize(width)} style:height={cssSize(height)} />
