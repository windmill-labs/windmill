<script lang="ts">
	import uFuzzy from '@leeoniya/ufuzzy'
	import { untrack } from 'svelte'

	interface Props {
		filter?: string
		items: any[] | undefined
		f: (item: any) => string
		filteredItems: (any & { marked: string })[] | undefined
		opts?: uFuzzy.Options
	}

	let { filter = '', items, f, filteredItems = $bindable(), opts = {} }: Props = $props()

	let uf = new uFuzzy(opts)

	function filterItems() {
		let trimmed = filter.trim()
		if (items == undefined || trimmed.length == 0) {
			filteredItems = items
			return
		}
		// pre-filter
		let idxs = uf.filter(plaintextItems, trimmed) ?? []

		let info = uf.info(idxs, plaintextItems, trimmed)
		let order = uf.sort(info, plaintextItems, trimmed)

		let result: any[] = []

		for (let i = 0; i < order.length; i++) {
			let infoIdx = order[i]
			result.push({
				...items[info.idx[infoIdx]],
				marked: uFuzzy.highlight(plaintextItems[info.idx[infoIdx]], info.ranges[infoIdx])
			})
		}
		filteredItems = result
	}
	let plaintextItems = $derived(items?.map((item) => f(item)) ?? [])

	$effect.pre(() => {
		plaintextItems && filter != undefined && setTimeout(() => untrack(() => filterItems()), 0)
	})
</script>
