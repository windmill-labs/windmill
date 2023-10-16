<script lang="ts">
	import { FlowStatusModule } from '$lib/gen'
	import { Skeleton } from './common'
	import type { GraphModuleState } from './graph'

	export let flowModuleStates: Record<string, GraphModuleState> = {}

	let items = flowModuleStates ? computeItems(flowModuleStates) : undefined

	$: computeItems(flowModuleStates)

	let min: undefined | number = undefined
	let max: undefined | number = undefined
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
			let started_at = v.started_at && nmin ? ((v.started_at - nmin) / total) * 100 : undefined
			let len = 0
			if (v.duration_ms) {
				len = v.duration_ms
			} else if (v.started_at) {
				len = Date.now() - v.started_at
			} else {
				len = 0
			}
			if (total) {
				len *= 100 / total
			} else {
				len = 0
			}
			return { name: k, started_at, len }
		})
		min = nmin
		max = nmax
		return nentries
	}
</script>

{#if items}
	<div class="border rounded-md divide-y">
		<div class="px-2 py-2 grid grid-cols-12 w-full"
			><div />
			<div class="col-span-11 pt-1 px-2 flex justify-between"><div>{min}</div><div>{max}</div></div>
		</div>
		{#each items as item}
			<div class="px-2 py-2 grid grid-cols-12 w-full"
				><div>{item.name}</div>
				<div class="col-span-11 pt-1 px-2 flex"
					><div style="width: {item.started_at}%" class="h-4" /><div
						style="width: {item.len}%"
						class="h-4 bg-blue-600 border center-center text-white text-xs"
						>{Math.ceil(item.len) ?? 0}%</div
					></div
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
