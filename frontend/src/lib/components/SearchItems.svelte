<script lang="ts">
	import uFuzzy from '@leeoniya/ufuzzy'
	import { untrack } from 'svelte'
	import { escapeHtml } from '$lib/utils'

	interface Props {
		filter?: string
		items: any[] | undefined
		f: (item: any) => string
		filteredItems: (any & { marked: string })[] | undefined
		opts?: uFuzzy.Options
	}

	let { filter = '', items, f, filteredItems = $bindable(), opts = {} }: Props = $props()

	let uf = new uFuzzy(untrack(() => opts))

	// Consumers render `marked` with {@html}, and the searched text is workspace
	// content (paths, descriptions, resource types, ...). uFuzzy's default mark
	// concatenates the raw substrings, so escape every part and let only the
	// <mark> wrapper through as markup.
	const markEscaped = (part: string, matched: boolean) =>
		matched ? `<mark>${escapeHtml(part)}</mark>` : escapeHtml(part)

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
				marked: uFuzzy.highlight(
					plaintextItems[info.idx[infoIdx]],
					info.ranges[infoIdx],
					markEscaped
				)
			})
		}
		filteredItems = result
	}
	let plaintextItems = $derived(items?.map((item) => f(item)) ?? [])

	$effect.pre(() => {
		plaintextItems && filter != undefined && setTimeout(() => untrack(() => filterItems()), 0)
	})
</script>
