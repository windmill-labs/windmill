<script lang="ts">
	import { parseAsset } from '$lib/components/assets/lib'
	import { type Asset, type RawScript } from '$lib/gen'
	import { inferAssets } from '$lib/infer'

	let { value, onChange }: { value: RawScript; onChange: (assets: Asset[]) => void } = $props()

	$effect(() => {
		inferAssets(value.language, value.content).then((assets) => {
			onChange(assets.map(parseAsset).filter((x) => !!x))
		})
	})
</script>
