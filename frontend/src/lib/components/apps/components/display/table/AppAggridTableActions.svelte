<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../../../types'
	import type { TableAction } from '$lib/components/apps/editor/component'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'

	import AppButton from '../../buttons/AppButton.svelte'
	import { AppCheckbox, AppSelect } from '../..'
	import { twMerge } from 'tailwind-merge'
	import { Popup } from '$lib/components/common'
	import { Plug2 } from 'lucide-svelte'
	import ComponentOutputViewer from '$lib/components/apps/editor/contextPanel/ComponentOutputViewer.svelte'
	import { connectOutput } from '$lib/components/apps/editor/appUtils'
	import RowWrapper from '../../layout/RowWrapper.svelte'

	export let id: string
	export let render: boolean
	export let actions: TableAction[] = []
	export let rowIndex: number
	export let row: { original: Record<string, any> }
	export let onSet: (id: string, value: any) => void
	export let onRemove: (id: string) => void
	export let wrapActions: boolean | undefined = undefined

	const dispatch = createEventDispatcher()
	const { selectedComponent, hoverStore, mode, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')
</script>

<RowWrapper value={row} index={rowIndex} {onSet} {onRemove}>
	<div
		class={twMerge(
			'flex flex-row justify-center items-center gap-4 h-full px-4 py-1',
			!wrapActions ? 'flex-wrap' : ''
		)}
	>
		{#each actions as action, actionIndex}
			<!-- svelte-ignore a11y-mouse-events-have-key-events -->
			<!-- svelte-ignore missing-declaration -->
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div
				on:mouseover|stopPropagation={() => {
					if (action.id !== $hoverStore) {
						$hoverStore = action.id
					}
				}}
				on:mouseout|stopPropagation={() => {
					if ($hoverStore !== undefined) {
						$hoverStore = undefined
					}
				}}
				on:pointerdown|stopPropagation={() => {
					$selectedComponent = [action.id]
				}}
				class={twMerge(
					($selectedComponent?.includes(action.id) || $hoverStore === action.id) &&
						$mode !== 'preview'
						? 'outline outline-indigo-500 outline-1 outline-offset-1 relative z-50'
						: 'relative'
				)}
			>
				{#if $mode !== 'preview'}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<!-- svelte-ignore a11y-no-static-element-interactions -->
					<div
						title={`Id: ${action.id}`}
						class={twMerge(
							'px-2 text-2xs font-bold absolute shadow  -top-2 -left-4 border z-50 rounded-sm w-8 !h-5 flex justify-center items-center',
							'bg-indigo-500/90 border-indigo-600 text-white',
							$selectedComponent?.includes(action.id) || $hoverStore === action.id
								? 'opacity-100'
								: 'opacity-0'
						)}
						on:click|stopPropagation={() => {
							$selectedComponent = [action.id]
						}}
					>
						{action.id}
					</div>

					{#if $connectingInput.opened}
						<div class="absolute z-50 left-8 -top-[10px]">
							<Popup
								floatingConfig={{
									strategy: 'absolute',
									placement: 'bottom-start'
								}}
							>
								<svelte:fragment slot="button">
									<button
										class="bg-red-500/70 border border-red-600 px-1 py-0.5"
										title="Outputs"
										aria-label="Open output"><Plug2 size={12} /></button
									>
								</svelte:fragment>
								<ComponentOutputViewer
									suffix="table"
									on:select={({ detail }) =>
										connectOutput(connectingInput, 'buttoncomponent', action.id, detail)}
									componentId={action.id}
								/>
							</Popup>
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
							{controls}
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
							}}
							verticalAlignment="center"
							{controls}
						/>
					{:else if action.type == 'selectcomponent'}
						<div class="w-40">
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
						}}
						noWFull
						id={action.id}
						customCss={action.customCss}
						configuration={action.configuration}
						recomputeIds={action.recomputeIds}
						extraQueryParams={{
							row
						}}
						componentInput={action.componentInput}
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
						}}
					/>
				{:else if action.type == 'selectcomponent'}
					<div class="w-40">
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
							}}
						/>
					</div>
				{/if}
			</div>
		{/each}
	</div>
</RowWrapper>
