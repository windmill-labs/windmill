<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { emptyString } from '$lib/utils'
	import { faAngleDown } from '@fortawesome/free-solid-svg-icons'
	import { LayoutDashboardIcon } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { slide } from 'svelte/transition'
	import type { AppEditorContext } from '../../types'
	import { components, type AppComponent } from '../Component.svelte'
	import { isOpenStoreCss } from './store'

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	let rawCode = ''
	let viewJsonSchema = false

	$: rawCode && parseJson()
	let jsonError = ''

	function parseJson() {
		try {
			$app.css = JSON.parse(rawCode ?? '')
			jsonError = ''
		} catch (e) {
			jsonError = e.message
		}
	}

	function switchTab(asJson: boolean) {
		viewJsonSchema = asJson
		if (asJson) {
			rawCode = JSON.stringify($app.css, null, 2)
		} else {
			parseJson()
		}
	}

	const entries: { type: 'app' | AppComponent['type']; name: string; icon: any; ids: string[] }[] =
		[
			{
				type: 'app' as 'app' | AppComponent['type'],
				name: 'App',
				icon: LayoutDashboardIcon,
				ids: ['viewer', 'grid', 'component']
			}
		].concat(
			Object.entries(components).map((c) => ({
				type: c[1].data.type as 'app' | AppComponent['type'],
				name: c[1].name,
				icon: c[1].icon,
				ids: c[1].cssIds ?? []
			}))
		)
	let isCustom: Record<string, boolean> = Object.fromEntries(
		Object.keys(entries).map((k) => [k, false])
	)
	if (Object.keys($isOpenStoreCss).length == 0) {
		$isOpenStoreCss = Object.fromEntries(Object.keys(entries).map((k) => [k, false]))
	}

	let newCss = $app.css ?? {}
	entries.forEach((e) => {
		if (!newCss[e.type]) {
			isCustom[e.type] = true
			newCss[e.type] = {}
		}
		e.ids.forEach((id) => {
			if (!newCss[e.type][id]) {
				newCss[e.type][id] = { style: '', class: '' }
			}
		})
		e.ids
			.map((id) => newCss[e.type][id].class != '' || newCss[e.type][id].style != '')
			.forEach((c) => {
				if (c) {
					isCustom[e.type] = true
				}
			})
	})
	//@ts-ignore
	$app.css = newCss
</script>

<div class="flex flex-row-reverse px-2">
	<Toggle
		on:change={(e) => switchTab(e.detail)}
		options={{
			right: 'As JSON'
		}}
	/>
</div>

{#if !viewJsonSchema}
	<div class="flex flex-col gap-2 p-1">
		{#each entries as { type, name, icon, ids }}
			{#if ids.length > 0}
				<div>
					<button
						on:click|preventDefault={() => ($isOpenStoreCss[type] = !$isOpenStoreCss[type])}
						class="w-full flex justify-between items-center px-1 py-1 
				rounded-sm duration-200 hover:bg-gray-100"
					>
						<h3 class="inline-flex gap-2  {isCustom[type] ? 'text-gray-800' : 'text-gray-500'}"
							>{name} <svelte:component this={icon} />
						</h3>
						<Icon
							data={faAngleDown}
							class="rotate-0 duration-300 {$isOpenStoreCss[type] ? '!rotate-180' : ''}"
						/>
					</button>
					{#if $isOpenStoreCss[type]}
						<div transition:slide|local={{ duration: 300 }} class="flex flex-col px-2 border">
							{#each ids as id}
								<div class="mb-2">
									<div class="mt-1 font-semibold">{id}</div>
									{#if $app?.css?.[type]?.[id]}
										<span class="text-xs">style</span>
										<input
											type="text"
											on:focus={() => (isCustom[type] = true)}
											bind:value={$app.css[type][id].style}
										/>
										<span class="text-xs">class</span>
										<input
											type="text"
											on:focus={() => (isCustom[type] = true)}
											bind:value={$app.css[type][id].class}
										/>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		{/each}
	</div>
{:else}
	{#if !emptyString(jsonError)}<span class="text-red-400 text-xs mb-1 flex flex-row-reverse"
			>{jsonError}</span
		>{:else}<div class="py-2" />{/if}
	<div class="h-full w-full border p-1 rounded">
		<SimpleEditor
			autoHeight
			class="editor"
			lang="json"
			bind:code={rawCode}
			fixedOverflowWidgets={false}
		/>
	</div>
{/if}
