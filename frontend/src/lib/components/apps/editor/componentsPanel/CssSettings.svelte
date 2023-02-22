<script lang="ts">
	import { onMount, getContext } from 'svelte'
	import { LayoutDashboardIcon, MousePointer2, CurlyBraces } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { emptyString } from '$lib/utils'
	import { Tab, TabContent, Tabs } from '../../../common'
	import type { AppEditorContext } from '../../types'
	import { components, type AppComponent } from '../Component.svelte'
	import ListItem from './ListItem.svelte'
	import { isOpenStore } from './store'

	const TITLE_PREFIX = 'Css.' as const
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

	onMount(() => {
		isOpenStore.addItems(
			[{name: 'App'}, ...Object.values(components)].map(component => {
				return { [TITLE_PREFIX + component.name]: false }
			})
		)
	})
</script>

<Tabs
	selected="ui"
	on:selected={(e) => switchTab(e.detail === 'json')}
	class="relative"
>
	<Tab value="ui" size="xs" class="grow">
		<div class="m-1 center-center">
			<MousePointer2 size={16} />
			<span class="pl-1">UI</span>
		</div>
	</Tab>
	<Tab value="json" size="xs" class="grow">
		<div class="m-1 center-center">
			<CurlyBraces size={16} />
			<span class="pl-1">JSON</span>
		</div>
	</Tab>
	<div slot="content" class="h-full overflow-y-auto">
		<TabContent value="ui">
			{#each entries as { type, name, icon, ids }}
				{#if ids.length > 0}
					<ListItem title={name} prefix={TITLE_PREFIX}>
						<div slot="title" class="flex items-center">
							<svelte:component this={icon} size={18} />
							<span class="ml-1">
								{name}
							</span>
						</div>
						<div class="pb-2">
							{#each ids as id}
								<div class="mb-3">
									<div class="text-sm font-semibold text-gray-500 capitalize pt-2">
										{id}
									</div>
									{#if $app?.css?.[type]?.[id]}
										<div class="border-l border-gray-400/80 py-1 pl-3.5 mt-1 ml-0.5">
											<label class="block pb-2">
												<div class="text-xs font-medium pb-0.5">
													Style
												</div>
												<input
													type="text"
													on:focus={() => (isCustom[type] = true)}
													bind:value={$app.css[type][id].style}
												/>
											</label>
											<label class="block">
												<div class="text-xs font-medium pb-0.5">
													Tailwind classes
												</div>
												<input
													type="text"
													on:focus={() => (isCustom[type] = true)}
													bind:value={$app.css[type][id].class}
												/>
											</label>
										</div>
									{/if}
								</div>
							{/each}
						</div>
					</ListItem>
				{/if}
			{/each}
		</TabContent>
		<TabContent value="json">
			{#if !emptyString(jsonError)}
				<span class="text-red-400 text-xs mb-1 flex flex-row-reverse">
					{jsonError}
				</span>
			{:else}
				<div class="py-2" />
			{/if}
			<div class="h-full w-full py-1">
				<SimpleEditor
					autoHeight
					class="editor"
					lang="json"
					bind:code={rawCode}
					fixedOverflowWidgets={false}
				/>
			</div>
		</TabContent>
	</div>
</Tabs>
