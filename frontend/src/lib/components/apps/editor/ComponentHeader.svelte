<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AppViewerContext } from '../types'
	import { Anchor, Bug, Expand, Move } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { Alert, Button } from '$lib/components/common'
	import type { AppComponent } from './component'

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

<span
	title={`Id: ${component.id}`}
	class={classNames(
		'px-2 text-2xs font-bold w-fit absolute shadow  -top-1 -left-2 border z-50',
		selected
			? 'bg-indigo-500/90 border-blue-500 text-white'
			: 'bg-gray-200/90 border-gray-300  text-gray-500'
	)}
	style="padding-top: 1px; padding-bottom: 1px;"
>
	{component.id}
</span>

{#if pointerdown || selected || hover}
	<button
		title="Lock Position"
		class={classNames(
			'text-gray-800 px-1 text-2xs py-0.5 font-bold w-fit shadow border border-gray-300 absolute  -top-1  right-[2.5rem] z-50 cursor-pointer',
			' hover:bg-gray-300',
			selected ? 'bg-gray-200/80' : 'bg-gray-200/80'
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
	<button
		title="Expand"
		class={classNames(
			'text-gray-800 px-1 text-2xs py-0.5 font-bold w-fit shadow border border-gray-300 absolute  -top-1  right-[4.5rem] z-50 cursor-pointer',
			' hover:bg-gray-300',
			selected ? 'bg-gray-200/80' : 'bg-gray-200/80'
		)}
		on:click={() => dispatch('expand')}
		on:pointerdown|stopPropagation
	>
		<Expand aria-label="Expand position" size={14} />
	</button>
{/if}

{#if selected || hover}
	<span
		title="Move"
		on:mousedown|stopPropagation|capture
		class={classNames(
			'text-gray-600 px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute border border-gray-300 -top-1 shadow right-[0.5rem] z-50 cursor-move',
			'bg-gray-200/80'
		)}
	>
		<Move size={14} />
	</span>
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
