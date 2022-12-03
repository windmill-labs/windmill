<script lang="ts">
	import uFuzzy from '@leeoniya/ufuzzy'

	export let filter: string = ''
	export let items: any[] | undefined
	export let f: (item: any) => string
	export let filteredItems: (any & { marked: string })[] | undefined
	export let opts: uFuzzy.Options = {}

	let uf = new uFuzzy(opts)
	$: plaintextItems = items?.map((item) => f(item)) ?? []

	$: plaintextItems && filter != undefined && setTimeout(() => filterItems(), 0)

	function filterItems() {
		if (items == undefined || filter.length == 0) {
			filteredItems = items
			return
		}
		// pre-filter
		let idxs = uf.filter(plaintextItems, filter)

		let info = uf.info(idxs, plaintextItems, filter)
		let order = uf.sort(info, plaintextItems, filter)

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
</script>
