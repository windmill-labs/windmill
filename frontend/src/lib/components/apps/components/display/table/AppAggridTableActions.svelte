<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../../../types'
	import type { TableAction } from '$lib/components/apps/editor/component'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'

	import AppButton from '../../buttons/AppButton.svelte'
	import AppCheckbox from '../../inputs/AppCheckbox.svelte'
	import AppSelect from '../../inputs/AppSelect.svelte'

	import { twMerge } from 'tailwind-merge'
	import { Plug2 } from 'lucide-svelte'
	import ComponentOutputViewer from '$lib/components/apps/editor/contextPanel/ComponentOutputViewer.svelte'
	import { connectOutput } from '$lib/components/apps/editor/appUtils'
	import RowWrapper from '../../layout/RowWrapper.svelte'
	import type { ICellRendererParams } from 'ag-grid-community'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import AppModal from '../../layout/AppModal.svelte'

	interface Props {
		p?: ICellRendererParams<any>
		id: string
		render: boolean
		actions?: TableAction[]
		rowIndex: number
		row: { original: Record<string, any> }
		onSet: (id: string, value: any, rowIndex: number) => void
		onRemove: (id: string, rowIndex: number) => void
		wrapActions?: boolean | undefined
		selectRow: (params: ICellRendererParams<any>) => void
		setModalRow: (row?: ICellRendererParams<any>) => void
	}

	let {
		p,
		id,
		render,
		actions = [],
		rowIndex,
		row,
		onSet,
		onRemove,
		wrapActions = undefined,
		selectRow,
		setModalRow
	}: Props = $props()

	const dispatch = createEventDispatcher()
	const { selectedComponent, hoverStore, mode, connectingInput, componentControl, app } =
		getContext<AppViewerContext>('AppViewerContext')

	$componentControl[id] = {
		...$componentControl[id],
		onDelete: () => {
			// Remove associated subgrid
			actions.forEach((action) => {
				if (action?.type === 'modalcomponent') delete $app.subgrids?.[`${action.id}-0`]
			})
		}
	}
</script>

<RowWrapper
	value={row}
	index={rowIndex}
	onSet={(id, value) => onSet(id, value, rowIndex)}
	onRemove={(id) => onRemove(id, rowIndex)}
>
	<div
		class={twMerge(
			'flex flex-row justify-center items-center gap-4 h-full px-4 py-1 w-full transition-opacity duration-50',
			wrapActions ? 'flex-wrap' : ''
		)}
	>
		{#each actions as action, actionIndex}
			<!-- svelte-ignore a11y_mouse_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				onmouseover={stopPropagation(() => {
					if (action.id !== $hoverStore) {
						$hoverStore = action.id
					}
				})}
				onmouseout={stopPropagation(() => {
					if ($hoverStore !== undefined) {
						$hoverStore = undefined
					}
				})}
				onpointerdown={stopPropagation((e) => {
					p && selectRow(p)

					if (!$connectingInput.opened) {
						$selectedComponent = [action.id]
					}
				})}
				class={twMerge(
					($selectedComponent?.includes(action.id) || $hoverStore === action.id) &&
						$mode !== 'preview'
						? 'outline outline-indigo-500 outline-1 outline-offset-1 relative '
						: 'relative',
					$hoverStore === action.id && $selectedComponent?.[0] !== action.id
						? 'outline-blue-500'
						: '',
					'w-full cursor-pointer'
				)}
			>
				{#if $mode !== 'preview'}
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						title={`Id: ${action.id}`}
						class={twMerge(
							'px-2 text-2xs font-bold absolute shadow  -top-2 -left-4 border z-50 rounded-sm w-8 !h-5 flex justify-center items-center',
							'bg-indigo-500/90 border-indigo-600 text-white',
							$hoverStore === action.id && $selectedComponent?.[0] !== action.id
								? 'bg-blue-500/90 border-blue-600'
								: '',
							$selectedComponent?.includes(action.id) || $hoverStore === action.id
								? 'opacity-100'
								: 'opacity-0'
						)}
						onclick={stopPropagation(() => {
							$selectedComponent = [action.id]
						})}
					>
						{action.id}
					</div>

					{#if $connectingInput.opened}
						<div class="absolute z-50 left-8 -top-[10px]">
							<Popover
								floatingConfig={{
									strategy: 'absolute',
									placement: 'bottom-start'
								}}
								closeOnOtherPopoverOpen
								contentClasses="p-4"
							>
								{#snippet trigger()}
									<button
										class="bg-red-500/70 border border-red-600 px-1 py-0.5"
										title="Outputs"
										aria-label="Open output"><Plug2 size={12} /></button
									>
								{/snippet}
								{#snippet content()}
									<ComponentOutputViewer
										suffix="table"
										on:select={({ detail }) => {
											const tableId = action.id.split('_')[0]

											connectOutput(
												connectingInput,
												action.type,
												tableId,
												`inputs.${action.id}[${rowIndex}].${detail}`
											)
										}}
										componentId={action.id}
									/>
								{/snippet}
							</Popover>
						</div>
					{/if}
				{/if}
				{#if rowIndex === 0}
					{@const controls = {
						left: () => {
							if (actionIndex === 0) {
								$selectedComponent = [id]
								return true
							} else if (actionIndex > 0) {
								$selectedComponent = [actions[actionIndex - 1].id]
								return true
							}
							return false
						},
						right: () => {
							if (actionIndex === actions.length - 1) {
								return id
							} else if (actionIndex < actions.length - 1) {
								$selectedComponent = [actions[actionIndex + 1].id]
								return true
							}
							return false
						}
					}}
					{#if action.type == 'buttoncomponent'}
						<AppButton
							noInitialize
							extraKey={'idx' + rowIndex}
							{render}
							noWFull
							preclickAction={async () => {
								dispatch('toggleRow')
								p && selectRow(p)
							}}
							id={action.id}
							customCss={action.customCss}
							configuration={action.configuration}
							recomputeIds={action.recomputeIds}
							extraQueryParams={{
								row
							}}
							componentInput={action.componentInput}
							verticalAlignment="center"
							replaceCallback={true}
							{controls}
						/>
					{:else if action.type == 'modalcomponent'}
						<AppModal
							{render}
							noWFull
							id={action.id}
							customCss={action.customCss}
							configuration={action.configuration}
							onOpenRecomputeIds={action.onOpenRecomputeIds}
							onCloseRecomputeIds={action.onCloseRecomputeIds}
							verticalAlignment="center"
							preclickAction={async () => {
								dispatch('toggleRow')
								p && (selectRow(p), setModalRow(p))
							}}
							onClose={() => setModalRow(undefined)}
						/>
					{:else if action.type == 'checkboxcomponent'}
						<AppCheckbox
							noInitialize
							extraKey={'idx' + rowIndex}
							{render}
							id={action.id}
							customCss={action.customCss}
							configuration={action.configuration}
							recomputeIds={action.recomputeIds}
							onToggle={action.onToggle}
							preclickAction={async () => {
								dispatch('toggleRow')
								p && selectRow(p)
							}}
							verticalAlignment="center"
							{controls}
						/>
					{:else if action.type == 'selectcomponent'}
						<div style="height: 30px; margin-top: -3px;">
							<AppSelect
								noDefault
								noInitialize
								extraKey={'idx' + rowIndex}
								{render}
								--font-size="10px"
								id={action.id}
								customCss={action.customCss}
								configuration={action.configuration}
								recomputeIds={action.recomputeIds}
								onSelect={action.onSelect}
								preclickAction={async () => {
									dispatch('toggleRow')
									p && selectRow(p)
								}}
								{controls}
							/>
						</div>
					{/if}
				{:else if action.type == 'buttoncomponent'}
					<AppButton
						noInitialize
						extraKey={'idx' + rowIndex}
						{render}
						preclickAction={async () => {
							dispatch('toggleRow')
							p && selectRow(p)
						}}
						noWFull
						id={action.id}
						customCss={action.customCss}
						configuration={action.configuration}
						recomputeIds={action.recomputeIds}
						extraQueryParams={{
							row
						}}
						replaceCallback={true}
						componentInput={action.componentInput}
					/>
				{:else if action.type == 'modalcomponent'}
					<AppModal
						{render}
						noWFull
						id={action.id}
						customCss={action.customCss}
						configuration={action.configuration}
						onOpenRecomputeIds={action.onOpenRecomputeIds}
						onCloseRecomputeIds={action.onCloseRecomputeIds}
						verticalAlignment="center"
						preclickAction={async () => {
							dispatch('toggleRow')
							p && (selectRow(p), setModalRow(p))
						}}
						onClose={() => setModalRow(undefined)}
					/>
				{:else if action.type == 'checkboxcomponent'}
					<AppCheckbox
						noInitialize
						extraKey={'idx' + rowIndex}
						{render}
						id={action.id}
						customCss={action.customCss}
						configuration={action.configuration}
						recomputeIds={action.recomputeIds}
						onToggle={action.onToggle}
						preclickAction={async () => {
							dispatch('toggleRow')
							p && selectRow(p)
						}}
					/>
				{:else if action.type == 'selectcomponent'}
					<div style="height: 30px; margin-top: -3px;">
						<AppSelect
							noDefault
							noInitialize
							extraKey={'idx' + rowIndex}
							{render}
							--font-size="10px"
							id={action.id}
							customCss={action.customCss}
							configuration={action.configuration}
							recomputeIds={action.recomputeIds}
							onSelect={action.onSelect}
							preclickAction={async () => {
								dispatch('toggleRow')
								p && selectRow(p)
							}}
						/>
					</div>
				{/if}
			</div>
		{/each}
	</div>
</RowWrapper>
