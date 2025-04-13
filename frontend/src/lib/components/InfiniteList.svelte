<script lang="ts">
	import DataTable from './table/DataTable.svelte'
	import { createEventDispatcher } from 'svelte'
	import { Row } from './table/index'
	import { twMerge } from 'tailwind-merge'

	const dispatch = createEventDispatcher()

	export let loading = false
	export let items: any[] | undefined = undefined
	export let selectedItemId: any | undefined = undefined
	export let isEmpty: boolean = true
	export let length: number = 0
	export let rounded: boolean = true
	export let noBorder: boolean = false
	export let extraRowClasses: { bgSelected: string; bgHover: string; class: string } = {
		bgSelected: '',
		bgHover: '',
		class: ''
	}
	export let neverShowLoader = false

	const perPage = 20

	let hasMore = false
	let page = 1
	let hasAlreadyFailed = false
	let hovered: any | undefined = undefined
	let initLoad = false
	let loadInputs: ((page: number, perPage: number) => Promise<any[]>) | undefined = undefined
	let deleteItemFn: ((id: any) => Promise<any>) | undefined = undefined

	export function reset() {
		items = undefined
		loading = false
		initLoad = false
		length = 0
		hasMore = false
		page = 1
	}

	let loadingMore = false

	export async function loadData(loadOption: 'refresh' | 'forceRefresh' | 'loadMore' = 'loadMore') {
		// console.log('loadData', loadOption, length, items?.length)

		if (!loadInputs) return
		if (loadOption == 'loadMore') {
			if (loadingMore || loading) return
			loadingMore = true
		}
		loading = true
		hasMore = length === perPage * page

		if (hasMore && loadOption === 'loadMore') {
			page++
		}

		try {
			const newItems = await loadInputs(1, page * perPage)

			if (
				loadOption === 'refresh' &&
				items &&
				items?.length > 0 &&
				newItems.length === items?.length &&
				newItems.every((i, index) => i.id === items?.[index]?.id)
			) {
				return
			}

			const existingIds = new Set(items?.map((i) => i.id) || [])
			items = newItems.map((item) => ({
				...item,
				isNew: initLoad && !existingIds.has(item.id)
			}))

			setTimeout(() => {
				if (items) {
					items = items.map((item) => ({
						...item,
						isNew: false
					}))
				}
			}, 2000)

			page = Math.ceil(items.length / perPage)
			hasMore = items.length === perPage * page
			if (hasMore) {
				const potentialNewItems = await loadInputs(page + 1, perPage)
				hasMore = potentialNewItems.length > 0
			}
			initLoad = true
			isEmpty = items.length === 0
			length = items.length
		} catch (err) {
			console.error(err)
			if (hasAlreadyFailed) return
			hasAlreadyFailed = true
			dispatch('error', { type: 'load', error: err })
		} finally {
			loading = false
			loadingMore = false
		}
	}

	export async function deleteItem(id: string) {
		if (!deleteItemFn) return
		try {
			items = items?.map((i) => (i.id === id ? { ...i, isDeleting: true } : i)) ?? []

			setTimeout(async () => {
				deleteItemFn ? await deleteItemFn(id) : null
				if (selectedItemId === id) {
					selectedItemId = null
				}
				loadData('refresh')
			}, 100)
		} catch (err) {
			dispatch('error', { type: 'delete', error: err })
		}
	}

	export async function setLoader(loader: (page: number, perPage: number) => Promise<any[]>) {
		loadInputs = loader
		loadData('forceRefresh')
	}

	export async function setDeleteItemFn(fn: (id: any) => Promise<any>) {
		deleteItemFn = fn
	}
</script>

<DataTable
	size="xs"
	infiniteScroll
	{hasMore}
	tableFixed={true}
	on:loadMore={() => {
		loadData()
	}}
	{loading}
	{loadingMore}
	{rounded}
	{noBorder}
	{neverShowLoader}
>
	<slot name="columns" />

	<tbody class="h-full w-full">
		<Row
			on:click={() => dispatch('select', 'extraRow')}
			class={twMerge(
				extraRowClasses.class,
				selectedItemId === 'extraRow' ? extraRowClasses.bgSelected : extraRowClasses.bgHover,
				'cursor-pointer rounded-md'
			)}
			on:hover={(e) => (hovered = e.detail ? 'extraRow' : undefined)}
		>
			<slot name="extra-row" hover={hovered === 'extraRow'} />
		</Row>
		{#each items ?? [] as item, index}
			{@const hover = item.id === hovered}
			<Row
				on:click={() => dispatch('select', item)}
				class={twMerge(
					selectedItemId === item.id ? 'bg-surface-selected' : 'hover:bg-surface-hover',
					'cursor-pointer rounded-md',
					item.isNew && index === 0 ? 'animate-slideIn' : 'group'
				)}
				on:hover={(e) => (hovered = e.detail ? item.id : undefined)}
			>
				<slot {item} {hover} />
			</Row>
		{/each}
	</tbody>

	<svelte:fragment slot="emptyMessage">
		{#if (!items || items?.length === 0) && (!loading || neverShowLoader)}
			<slot name="empty" {items} />
		{/if}
	</svelte:fragment>
</DataTable>

<style>
	@keyframes slideOut {
		from {
			opacity: 1;
			transform: translateX(0);
			max-height: 100px;
		}
		to {
			opacity: 0;
			transform: translateX(-100%);
			max-height: 0;
			margin: 0;
			padding: 0;
		}
	}

	@keyframes greenHighlight {
		0% {
			background-color: rgba(70, 255, 138, 0.4);
			box-shadow: 0 0 15px rgb(34 197 94 / 0.3);
		}
		100% {
			background-color: transparent;
			box-shadow: none;
		}
	}

	:global(.animate-slideIn) {
		animation: greenHighlight 1s ease-out forwards;
		will-change: transform, opacity, background-color, box-shadow;
		position: relative;
	}
</style>
