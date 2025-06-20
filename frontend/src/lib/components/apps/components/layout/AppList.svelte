<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ListWrapper from './ListWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { Button } from '$lib/components/common'
	import { Loader2, ChevronLeft, ChevronRight } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'listcomponent'> | undefined
		render: boolean
		initializing: boolean | undefined
	}

	let {
		id,
		componentInput,
		configuration,
		customCss = undefined,
		render,
		initializing = $bindable()
	}: Props = $props()

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput, allIdsInPath, mode } =
		getContext<AppViewerContext>('AppViewerContext')
	let page = $state(0)

	let everRender = $state(render)
	$effect.pre(() => {
		render && !everRender && (everRender = true)
	})

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		inputs: {},
		page: 0
	})

	let resolvedConfig = $state(
		initConfig(components['listcomponent'].initialData.configuration, configuration)
	)

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	let css = $state(initCss($app.css?.listcomponent, customCss))
	let result: any[] | undefined = $state(undefined)

	let isCard = $derived(resolvedConfig.width?.selected == 'card')

	let inputs = $state({})
	let loading: boolean = $state(false)
	let isPreviousLoading = $state(false)
	let isNextLoading = $state(false)

	$effect.pre(() => {
		if (!loading) {
			isPreviousLoading = false
			isNextLoading = false
		}
	})

	function getPagination(
		configuration: {
			auto: { pageSize: number | undefined }
			manual: { pageCount: number | undefined }
		},
		mode: 'auto' | 'manual' = 'auto',
		initialData: Array<any> | undefined = [],
		page: number = 0
	) {
		const l = initialData ? initialData.length : 0
		if (mode === 'auto') {
			const pageSize: number = configuration.auto.pageSize ?? 0
			const shouldDisplayPagination = (pageSize ?? 0) < l
			const total = Math.ceil(l / (pageSize ?? 0))

			return {
				shouldDisplayPagination,
				indexOffset: page * pageSize,
				maxIndex: (page + 1) * pageSize - 1,
				disableNext: pageSize > 0 && (page + 1) * pageSize >= l,
				total: total
			}
		} else {
			const pageCount = configuration.manual.pageCount ?? 0
			const total = pageCount

			return {
				shouldDisplayPagination: true,
				indexOffset: 0,
				maxIndex: l,
				disableNext: page + 1 >= pageCount,
				total: total
			}
		}
	}

	let pagination = $derived(
		getPagination(
			resolvedConfig.pagination?.configuration,
			resolvedConfig.pagination?.selected,
			result,
			page
		)
	)
</script>

{#each Object.keys(components['listcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.listcomponent}
	/>
{/each}

<InitializeComponent {id} />

<RunnableWrapper
	hasChildrens
	{render}
	{outputs}
	autoRefresh
	{componentInput}
	{id}
	bind:initializing
	bind:result
	bind:loading
>
	{#if everRender}
		<div
			class={twMerge('w-full h-full', css?.container?.class, 'wm-list')}
			style={css?.container?.style}
		>
			<div
				class="w-full h-full shrink flex {$allIdsInPath.includes(id) && $mode == 'dnd'
					? 'overflow-visible'
					: 'overflow-auto'} {isCard
					? 'gap-2 flex-wrap'
					: resolvedConfig?.displayBorders
						? 'divide-y flex-col'
						: 'flex-col'}"
			>
				{#if $app.subgrids?.[`${id}-0`] && Array.isArray(result) && result.length > 0}
					{#each result ?? [] as value, index (index)}
						{@const inRange = index <= pagination.maxIndex && index >= pagination.indexOffset}
						<div
							style={inRange
								? `${
										isCard
											? `min-width: ${resolvedConfig.width?.configuration?.card?.minWidthPx}px; `
											: ''
									} max-height: ${resolvedConfig.heightPx}px;`
								: ''}
							class={inRange
								? `${
										$allIdsInPath.includes(id)
											? 'overflow-visible'
											: resolvedConfig.heightPx
												? 'overflow-auto'
												: ''
									} ${!isCard ? 'w-full' : resolvedConfig?.displayBorders ? 'border' : ''}`
								: 'h-0 float overflow-hidden invisible absolute'}
						>
							<ListWrapper
								onSet={(id, value) => {
									if (!inputs[id]) {
										inputs[id] = { [index]: value }
									} else {
										inputs[id] = { ...inputs[id], [index]: value }
									}
									outputs?.inputs.set(inputs, true)
								}}
								onRemove={(id) => {
									if (inputs?.[id] == undefined) {
										return
									}
									if (index == 0) {
										delete inputs[id]
										inputs = { ...inputs }
									} else {
										delete inputs[id][index]
										inputs[id] = { ...inputs[id] }
									}
									outputs?.inputs.set(inputs, true)
								}}
								{value}
								{index}
							>
								<SubGridEditor
									visible={render && inRange}
									{id}
									subGridId={`${id}-0`}
									containerHeight={resolvedConfig.heightPx}
									on:focus={() => {
										if (!$connectingInput.opened) {
											$selectedComponent = [id]
										}
										onFocus()
									}}
								/>
							</ListWrapper>
						</div>
					{/each}
				{:else}
					<ListWrapper disabled value={undefined} index={0}>
						<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
					</ListWrapper>
					{#if !Array.isArray(result)}
						<div class="text-center text-tertiary">Input data is not an array</div>
					{/if}
				{/if}
			</div>
			{#if pagination.shouldDisplayPagination}
				<div
					class="bg-surface-secondary z-20 h-8 flex flex-row gap-1 p-1 items-center wm-list-pagination absolute bottom-0 w-full"
				>
					<Button
						size="xs2"
						variant="border"
						color="light"
						btnClasses="flex flex-row gap-1 items-center wm-list-pagination-buttons"
						on:click={() => {
							isPreviousLoading = true
							page = page - 1
							outputs?.page.set(page, true)
						}}
						disabled={page === 0}
					>
						{#if isPreviousLoading && loading}
							<Loader2 size={14} class="animate-spin" />
						{:else}
							<ChevronLeft size={14} />
						{/if}
						Previous
					</Button>
					<Button
						size="xs2"
						variant="border"
						color="light"
						btnClasses="flex flex-row gap-1 items-center wm-list-pagination-buttons"
						on:click={() => {
							isNextLoading = true
							page = page + 1
							outputs?.page.set(page, true)
						}}
						disabled={pagination.disableNext && pagination.total > 0}
					>
						Next

						{#if isNextLoading && loading}
							<Loader2 size={14} class="animate-spin" />
						{:else}
							<ChevronRight size={14} />
						{/if}
					</Button>
					<div class="text-xs"
						>{page + 1} {pagination.total > 0 ? `of ${pagination.total}` : ''}</div
					>
				</div>
			{/if}
		</div>
	{:else if $app.subgrids}
		<ListWrapper disabled value={undefined} index={0}>
			<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
		</ListWrapper>
	{/if}
</RunnableWrapper>
