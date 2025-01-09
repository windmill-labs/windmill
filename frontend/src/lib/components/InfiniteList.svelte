<script lang="ts">
	import DataTable from './table/DataTable.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import { createEventDispatcher } from 'svelte'
	import { Row } from './table/index'
	import { twMerge } from 'tailwind-merge'
	const dispatch = createEventDispatcher()

	export let loadInputs: ((page: number, perPage: number) => Promise<any[]>) | undefined = undefined
	export let deleteItemFn: ((id: string) => Promise<any>) | undefined = undefined
	export let loading = false
	export let items: any[] | undefined = undefined
	export let selectedItemId: any | undefined = undefined

	let hasMore = false
	let page = 1
	let perPage = 10

	let hasAlreadyFailed = false

	export async function loadData(refresh = false) {
		if (!loadInputs) return
		loading = true
		hasMore = hasMore

		if (hasMore && !refresh) {
			page++
		}

		try {
			const newItems = await loadInputs(1, page * perPage)

			if (
				refresh &&
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
				isNew: !existingIds.has(item.id)
			}))

			setTimeout(() => {
				if (items) {
					items = items.map((item) => ({
						...item,
						isNew: false
					}))
				}
			}, 1000)

			page = Math.floor(items.length / perPage) + 1
			hasMore = items.length === perPage * (page - 1)
		} catch (e) {
			console.error(e)
			if (hasAlreadyFailed) return
			hasAlreadyFailed = true
			dispatch('error', e)
		} finally {
			loading = false
		}
	}

	export async function deleteItem(id: string) {
		if (!deleteItemFn) return
		try {
			items = items?.map((i) => (i.id === id ? { ...i, isDeleting: true } : i)) ?? []

			setTimeout(async () => {
				await deleteItemFn(id)
				if (selectedItemId === id) {
					selectedItemId = null
				}
				loadData(true)
			}, 300)
		} catch (err) {
			dispatch('error', err)
		}
	}

	$: {
		if (loadInputs) {
			loadData(true)
		}
	}

	$: console.log('dbg hasMore', hasMore)
</script>

<div class="h-full">
	{#if items === undefined && loading}
		<Skeleton layout={[[8]]} />
	{:else if items && items.length > 0}
		<DataTable
			size="xs"
			infiniteScroll
			{hasMore}
			tableFixed={true}
			on:loadMore={() => {
				console.log('dbg loadMore')
				loadData()
			}}
		>
			<slot name="columns" />

			<tbody class="w-full overflow-y-auto">
				{#each items ?? [] as item, index}
					<Row
						on:click={() => dispatch('select', item)}
						class={twMerge(
							selectedItemId === item.id ? 'bg-surface-selected' : 'hover:bg-surface-hover',
							'cursor-pointer rounded-md',
							item.isNew && index === 0 ? 'animate-slideIn' : 'group',
							item.isDeleting && 'animate-slideOut'
						)}
					>
						<slot {item} />
					</Row>
				{/each}
			</tbody>
		</DataTable>
	{:else}
		<slot name="empty" {items} />
	{/if}
</div>

<style>
	@keyframes slideIn {
		from {
			opacity: 0;
			transform: translateY(-100%);
			position: absolute;
			top: 0;
		}
		to {
			opacity: 1;
			transform: translateY(0);
			position: relative;
			top: auto;
		}
	}

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
			background-color: rgba(8, 240, 93, 0.2);
			box-shadow: 0 0 15px rgb(34 197 94 / 0.3);
		}
		100% {
			background-color: transparent;
			box-shadow: none;
		}
	}

	:global(.animate-slideIn) {
		animation: slideIn 0.3s ease-out forwards, greenHighlight 1s ease-out forwards;
		will-change: transform, opacity, background-color, box-shadow;
		position: relative;
	}

	:global(.animate-slideOut) {
		animation: slideOut 0.3s ease-out forwards;
		will-change: transform, opacity, max-height, margin, padding;
		position: relative;
		pointer-events: none;
	}
</style>
