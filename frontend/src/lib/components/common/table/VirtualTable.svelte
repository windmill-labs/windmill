<script>
	const CLASSNAME_TABLE = 'tablesort' // keep same for compatibility with https://github.com/mattiash/svelte-tablesort
	const CLASSNAME_SORTABLE = 'sortable'
	const CLASSNAME_ASC = 'ascending'
	const CLASSNAME_DESC = 'descending'

	import { compareNumbers, compareStrings, sortFunction } from 'generator-sort'
	import { onMount, tick } from 'svelte'

	// props
	export let items
	export let requireBorderCollapse = false
	let className = ''
	export { className as class }

	// MARK: virtual stuff
	export let height = '100%' // the height of the viewport/table
	export let itemHeight = undefined // the height of each row

	// read-only, but visible to consumers via bind:start resp. bind:end
	export let start = 0 // the index of the first visible item
	export let end = 0 // the index of the last visible item

	// local state
	let averageHeight
	let bottom = 0
	let contents
	let headHeight = 0
	let footHeight = 0
	let heightMap = []
	let mounted
	let rows
	let thead
	let top = 0
	let viewport
	let viewportHeight = 0
	let visible

	// whenever `items` changes, invalidate the current heightmap
	$: if (mounted) refreshHeightMap(sortedItems, viewportHeight, itemHeight)

	async function refreshHeightMap(items, viewportHeight, itemHeight) {
		const { scrollTop } = viewport
		await tick() // wait until the DOM is up to date
		let contentHeight = top - (scrollTop - headHeight)
		let i = start
		while (contentHeight < viewportHeight - headHeight && i < items.length) {
			let row = rows[i - start]
			if (!row) {
				end = i + 1
				await tick() // render the newly visible row
				row = rows[i - start]
			}
			const row_height = (heightMap[i] = itemHeight || row.getBoundingClientRect().height)
			contentHeight += row_height
			i += 1
		}
		end = i
		const remaining = items.length - end
		averageHeight = (top + contentHeight) / end
		bottom = remaining * averageHeight + footHeight
		heightMap.length = items.length
		await scrollToIndex(0, { behavior: 'auto' })
	}

	function getComputedPxAmount(elem, pseudoElem, property) {
		const compStyle = getComputedStyle(elem, pseudoElem)
		return parseInt(compStyle[property])
	}

	async function handleScroll() {
		rows = contents.children
		const isStartOverflow = sortedItems.length < start
		const rowBottomBorder = getComputedPxAmount(rows[1], null, 'border-bottom-width')
		const rowTopBorder = getComputedPxAmount(rows[1], null, 'border-top-width')
		const headBorderTop = getComputedPxAmount(thead, null, 'border-top-width')
		const headBorderBottom = getComputedPxAmount(thead, null, 'border-bottom-width')
		const actualBorderCollapsedWidth = requireBorderCollapse
			? Math.max(rowBottomBorder, rowTopBorder)
			: 0

		if (isStartOverflow) {
			await scrollToIndex(sortedItems.length - 1, { behavior: 'auto' })
		}

		const { scrollTop } = viewport
		let new_start = 0
		// acquire height map for currently visible rows
		for (let v = 0; v < rows.length; v += 1) {
			heightMap[start + v] = itemHeight || rows[v].getBoundingClientRect().height
		}
		let i = 0
		// start from top: thead, with its borders, plus the first border to afterwards neglect
		let y = headHeight + rowTopBorder / 2
		let row_heights = []
		// loop items to find new start
		while (i < sortedItems.length) {
			const row_height = heightMap[i] || averageHeight
			row_heights[i] = row_height
			// we only want to jump if the full (incl. border) row is away
			if (y + row_height + actualBorderCollapsedWidth > scrollTop) {
				// this is the last index still inside the viewport
				new_start = i
				top =
					y -
					(requireBorderCollapse
						? (headBorderBottom + headBorderTop) / 2
						: headHeight + rowTopBorder / 2) //+ rowBottomBorder - rowTopBorder
				break
			}
			y += row_height
			i += 1
		}

		// console.log(
		//     'a',
		//     i,
		//     y,
		//     top,
		//     bottom,
		//     scrollTop,
		//     headHeight,
		//     averageHeight,
		//     actualBorderCollapsedWidth,
		//     row_heights,
		//     heightMap
		// )
		new_start = Math.max(0, new_start)
		// loop items to find end
		while (i < sortedItems.length) {
			const row_height = heightMap[i] || averageHeight
			y += row_height
			i += 1
			if (y > scrollTop + viewportHeight) {
				break
			}
		}
		start = new_start
		end = i
		const remaining = sortedItems.length - end
		if (end === 0) {
			end = 10
		}
		averageHeight = y / end
		let remaining_height = remaining * averageHeight // 0
		// compute height map for remaining items
		while (i < sortedItems.length) {
			i += 1
			heightMap[i] = averageHeight
			// remaining_height += heightMap[i] / remaining
		}
		// find the
		bottom = remaining_height
		if (!isFinite(bottom)) {
			bottom = 200000
		}
	}

	export async function scrollToIndex(index, opts) {
		const { scrollTop } = viewport
		const itemsDelta = index - start
		const _itemHeight = itemHeight || averageHeight
		const distance = itemsDelta * _itemHeight
		opts = {
			left: 0,
			top: scrollTop + distance,
			behavior: 'smooth',
			...opts
		}
		viewport.scrollTo(opts)
	}

	// MARK: table sort stuff
	let sortOrder = [[]]

	$: sortedItems = sorted([...items], sortOrder)

	$: visible = sortedItems.slice(start, end).map((data, i) => {
		return { index: i + start, data }
	})

	const sorted = function (arr, sortOrder) {
		arr.sort(
			sortFunction(function* (a, b) {
				for (let [fieldName, r] of sortOrder) {
					const reverse = r === 0 ? 1 : -1
					if (typeof a[fieldName] === 'number') {
						yield reverse * compareNumbers(a[fieldName], b[fieldName])
					} else {
						yield reverse * compareStrings(a[fieldName], b[fieldName])
					}
				}
			})
		)

		return arr
	}

	function updateSortOrder(th, push) {
		const fieldName = th.dataset.sort
		if (push) {
			if (sortOrder[sortOrder.length - 1][0] === fieldName) {
				sortOrder[sortOrder.length - 1] = [fieldName, (sortOrder[sortOrder.length - 1][1] + 1) % 2]
			} else {
				sortOrder = [...sortOrder, [fieldName, 0]]
			}
		} else {
			if (sortOrder.length === 1 && sortOrder[0][0] === fieldName) {
				sortOrder[0] = [fieldName, (sortOrder[0][1] + 1) % 2]
			} else {
				resetClasses()
				sortOrder = [[fieldName, 0]]
			}
		}
		th.className =
			CLASSNAME_SORTABLE +
			' ' +
			(sortOrder[sortOrder.length - 1][1] ? CLASSNAME_DESC : CLASSNAME_ASC)
	}

	function resetClasses() {
		const th = thead.getElementsByTagName('th')
		for (let i = 0; i < th.length; i++) {
			if (th[i].dataset.sort) {
				th[i].className = CLASSNAME_SORTABLE
			}
		}
	}

	// MARK: initial triggers
	onMount(() => {
		// triggger initial refresh for virtual
		rows = contents.children
		mounted = true
		refreshHeightMap(items, viewportHeight, itemHeight)

		// prepare sorting
		const th = thead.getElementsByTagName('th')
		for (let i = 0; i < th.length; i++) {
			if (th[i].dataset.sort) {
				th[i].className = CLASSNAME_SORTABLE
				th[i].onclick = (event) => updateSortOrder(th[i], event.shiftKey)
			}
			if (th[i].dataset.sortInitial === 'descending') {
				th[i].className = CLASSNAME_SORTABLE + ' ' + CLASSNAME_DESC
				sortOrder = [...sortOrder, [th[i].dataset.sort, 1]]
			} else if (th[i].dataset.sortInitial != undefined) {
				th[i].className = CLASSNAME_SORTABLE + ' ' + CLASSNAME_ASC
				sortOrder = [...sortOrder, [th[i].dataset.sort, 0]]
			}
		}
	})
</script>

<svelte-virtual-table-viewport>
	<table
		class:require-border-collapse={requireBorderCollapse}
		class="{CLASSNAME_TABLE}
      {className} table"
		bind:this={viewport}
		bind:offsetHeight={viewportHeight}
		on:scroll={handleScroll}
		style="height: {height}; --bw-svt-p-top: {top}px; --bw-svt-p-bottom: {bottom}px; --bw-svt-head-height: {headHeight}px; --bw-svt-foot-height: {footHeight}px; --bw-svt-avg-row-height: {averageHeight}px"
	>
		<tbody bind:this={contents} class="tbody">
			{#each visible as item}
				<slot name="tbody" item={item.data} index={item.index}>Missing Table Row</slot>
			{/each}
		</tbody>
	</table>
</svelte-virtual-table-viewport>

<style type="text/css">
	table {
		position: relative;
		overflow-y: auto;
		-webkit-overflow-scrolling: touch;
		max-height: 100vh;
		box-sizing: border-box;
		display: block;
		/* table-layout: fixed; */
	}
	table :is(thead, tfoot, tbody) {
		display: table;
		table-layout: fixed;
		width: 100%;
		box-sizing: border-box;
	}
	table.require-border-collapse thead {
		min-height: calc(var(--bw-svt-p-top));
	}
	table.require-border-collapse tfoot {
		min-height: calc(var(--bw-svt-p-bottom));
	}
	table.require-border-collapse {
		border-collapse: collapse;
	}
	table:not(.require-border-collapse) tbody {
		padding-top: var(--bw-svt-p-top);
		padding-bottom: var(--bw-svt-p-bottom);
	}
	tbody {
		position: relative;
		box-sizing: border-box;
		border: 0px solid currentColor;
	}

	/** sortable styles */
	thead :global(th.sortable) {
		cursor: pointer;
		user-select: none;
		-moz-user-select: none;
		-webkit-user-select: none;
		-ms-user-select: none;
	}
</style>
