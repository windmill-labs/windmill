<script lang="ts">
	import { FlowStatusModule } from '$lib/gen'
	import { tweened, type Tweened } from 'svelte/motion'
	import { Skeleton } from './common'
	import type { GraphModuleState } from './graph'
	import TimelineBar from './TimelineBar.svelte'
	import { displayDate } from '$lib/utils'

	export let flowModuleStates: Record<string, GraphModuleState> = {}

	let min: undefined | number = undefined
	let max: undefined | number = undefined

	let items:
		| Record<
				string,
				{
					started_at_p: Tweened<number>
					len_p: Tweened<number>
					duration_ms?: number
					started_at?: number
				}
		  >
		| undefined = undefined

	$: computeItems(flowModuleStates)

	function computeItems(flowModuleStates: Record<string, GraphModuleState>): any {
		let nmin: undefined | number = undefined
		let nmax: undefined | number = undefined

		let isStillRunning = false
		Object.values(flowModuleStates).forEach((v) => {
			if (v.started_at) {
				if (!nmin) {
					nmin = v.started_at
				} else {
					nmin = Math.min(nmin, v.started_at)
				}
			}
			if (v.type == FlowStatusModule.type.IN_PROGRESS) {
				isStillRunning = true
			}
			if (!isStillRunning) {
				if (v.started_at && v.duration_ms) {
					let lmax = v.started_at + v.duration_ms
					if (!nmax) {
						nmax = lmax
					} else {
						nmax = Math.max(nmax, lmax)
					}
				}
			}
		})

		let total = (isStillRunning || !nmax ? Date.now() : nmax) - (nmin ?? Date.now())

		const nentries = Object.entries(flowModuleStates).map(([k, v]) => {
			let started_at_n = v.started_at && nmin ? ((v.started_at - nmin) / total) * 100 : undefined
			let len_n = 0
			if (v.duration_ms) {
				len_n = v.duration_ms
			} else if (v.started_at) {
				len_n = Date.now() - v.started_at
			} else {
				len_n = 0
			}
			if (total) {
				len_n *= 100 / total
			} else {
				len_n = 0
			}
			let p = items?.[k]
			console.log(p)
			let started_at_p =
				p?.started_at_p ??
				tweened(started_at_n ?? 0, {
					duration: 1000
				})
			let len_p =
				p?.len_p ??
				tweened(len_n, {
					duration: 1000
				})
			started_at_p?.set(started_at_n ?? 0)
			len_p.set(len_n)
			return [k, { started_at_p, len_p, started_at: v.started_at, duration_ms: v.duration_ms }]
		})
		min = nmin
		max = nmax
		items = Object.fromEntries(nentries)
	}
</script>

{#if items}
	<div class="border rounded-md divide-y">
		<div class="px-2 py-2 grid grid-cols-12 w-full"
			><div />
			<div class="col-span-11 pt-1 px-2 flex text-xs text-secondary justify-between"
				><div>| {min ? displayDate(new Date(min)) : ''}</div><div
					>{max ? displayDate(new Date(max)) + ' |' : 'running'}</div
				></div
			>
		</div>
		{#each Object.entries(items) as [k, v] (k)}
			<div class="px-2 py-2 grid grid-cols-12 w-full"
				><div>{k}</div>
				<div class="col-span-11 pt-1 px-2 flex"
					><TimelineBar
						len_p={v.len_p}
						started_at_p={v.started_at_p}
						duration_ms={v.duration_ms}
						started_at={v.started_at}
					/></div
				></div
			>
		{/each}
	</div>
{:else}
	<div class="mt-4" />
	<Skeleton layout={[[2], 1]} />
	{#each new Array(6) as _}
		<Skeleton layout={[[4], 0.5]} />
	{/each}
{/if}
