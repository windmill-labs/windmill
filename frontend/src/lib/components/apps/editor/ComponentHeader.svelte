<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AppViewerContext } from '../types'
	import { Anchor, Bug, Expand, Move } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { Alert, Button } from '$lib/components/common'
	import { components, type AppComponent } from './component'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let pointerdown: boolean = false
	export let hover: boolean = false

	const dispatch = createEventDispatcher()

	const { errorByComponent, openDebugRun } = getContext<AppViewerContext>('AppViewerContext')

	$: error = Object.values($errorByComponent).find((e) => e.componentId === component.id)

	function openDebugRuns() {
		if ($openDebugRun) {
			$openDebugRun(component.id)
		}
	}
</script>

{#if pointerdown || selected || hover}
	<span
		title={`Id: ${component.id}`}
		class={classNames(
			'px-2 text-2xs font-semibold w-fit absolute shadow top-[-18px] left-0 border rounded-t-sm',
			selected
				? 'bg-indigo-500 border-indigo-600 text-white'
				: 'bg-blue-500 border-blue-600 text-white'
		)}
	>
		{components[component.type].name}
		{component.id}
	</span>
{/if}

{#if selected}
	<div class="top-[-19px] right-0 flex flex-row absolute ">
		<button
			title="Expand"
			class={classNames(
				'px-1 text-2xs py-0.5 font-bold w-fit border-t border-l cursor-pointer rounded-tl-sm',
				'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800'
			)}
			on:click={() => dispatch('expand')}
			on:pointerdown|stopPropagation
		>
			<Expand aria-label="Expand position" size={14} />
		</button>
		<button
			title="Lock Position"
			class={classNames(
				'px-1 text-2xs py-0.5 font-bold w-fit border-t border-l cursor-pointer',
				'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800'
			)}
			on:click={() => dispatch('lock')}
			on:pointerdown|stopPropagation
		>
			{#if locked}
				<Anchor aria-label="Unlock position" size={14} class="text-orange-500" />
			{:else}
				<Anchor aria-label="Lock position" size={14} />
			{/if}
		</button>
		<span
			title="Move"
			on:mousedown|stopPropagation|capture
			class={classNames(
				'px-1 text-2xs py-0.5 font-bold w-fit border-t border-x cursor-move rounded-tr-sm',
				'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800'
			)}
		>
			<Move size={14} />
		</span>
	</div>
{/if}

{#if error}
	{@const json = JSON.parse(JSON.stringify(error.error))}
	<span
		title="Error"
		class={classNames(
			'text-red-500 px-1 text-2xs py-0.5 font-bold w-fit absolute border border-red-500 -bottom-1  shadow left-1/2 transform -translate-x-1/2 z-50 cursor-pointer',
			'bg-red-100/80'
		)}
	>
		<Popover notClickable placement="bottom" popupClass="!bg-white border w-96">
			<Bug size={14} />
			<span slot="text">
				<div class="bg-white">
					<Alert type="error" title={`${json?.name}: ${json?.message}`}>
						<div class="flex flex-col gap-2">
							<div>
								<pre class=" whitespace-pre-wrap text-gray-900 bg-white border w-full p-4 text-xs"
									>{json?.stack ?? ''}
									</pre>
							</div>
							<Button color="red" variant="border" on:click={openDebugRuns}>Open Debug Runs</Button>
						</div>
					</Alert>
				</div>
			</span>
		</Popover>
	</span>
{/if}
