<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { faBed, faCodeBranch, faSave, faStop } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { PhoneIncoming, Repeat } from 'svelte-lucide'

	export let module: FlowModule

	const dispatch = createEventDispatcher()

	let width = 0

	$: moduleRetry = module.retry?.constant || module.retry?.exponential
</script>

<div class="flex flex-row space-x-2" bind:clientWidth={width}>
	{#if module.value.type === 'script' || module.value.type === 'rawscript'}
		<button
			class={classNames('badge', module.stop_after_if ? 'badge-on' : 'badge-off')}
			on:click={() => dispatch('toggleStopAfterIf')}
		>
			<Icon data={faStop} scale={0.8} />
		</button>
		<button
			class={classNames('badge', moduleRetry ? 'badge-on' : 'badge-off', 'center-center')}
			on:click={() => dispatch('toggleRetry')}
		>
			<Repeat size="14px" />
		</button>
		<button
			class={classNames('badge', Boolean(module.sleep) ? 'badge-on' : 'badge-off')}
			on:click={() => dispatch('toggleSleep')}
		>
			<Icon data={faBed} scale={0.8} />
		</button>
		<button
			class={classNames('badge', Boolean(module.suspend) ? 'badge-on' : 'badge-off')}
			on:click={() => dispatch('toggleSuspend')}
		>
			<PhoneIncoming size="14px" />
		</button>
	{/if}
	{#if module.value.type === 'script'}
		<div class="w-2" />
		<Button
			size="xs"
			color="light"
			variant="border"
			on:click={() => dispatch('fork')}
			startIcon={{ icon: faCodeBranch }}
			iconOnly={false}
		>
			Fork
		</Button>
	{/if}

	{#if module.value.type === 'rawscript'}
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: faSave }}
			on:click={() => dispatch('createScriptFromInlineScript')}
			iconOnly={false}
		>
			Save to workspace
		</Button>
	{/if}
</div>

<style>
	.badge {
		@apply whitespace-nowrap text-sm font-medium border px-2.5 py-0.5 rounded cursor-pointer flex items-center;
	}

	.badge-on {
		@apply bg-blue-100 text-blue-800 hover:bg-blue-200;
	}

	.badge-off {
		@apply bg-gray-100 text-gray-800 hover:bg-gray-200;
	}
</style>
