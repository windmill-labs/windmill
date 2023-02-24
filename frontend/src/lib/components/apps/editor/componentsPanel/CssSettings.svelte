<script lang="ts">
	import { onMount, getContext } from 'svelte'
	import { LayoutDashboardIcon, MousePointer2, CurlyBraces } from 'lucide-svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { emptyString } from '$lib/utils'
	import { Tab, TabContent, Tabs } from '../../../common'
	import type { AppEditorContext } from '../../types'
	import ListItem from './ListItem.svelte'
	import { isOpenStore } from './store'
	import CssProperty from './CssProperty.svelte'
	import { components, type AppComponent } from '../component'
	import { slide } from 'svelte/transition'

	const STATIC_ELEMENTS = [ 'app' ] as const
	const TITLE_PREFIX = 'Css.' as const

	type CustomCSSType = typeof STATIC_ELEMENTS[number] | AppComponent['type'];

	interface CustomCSSEntry {
		type: CustomCSSType;
		name: string;
		icon: any;
		ids: string[];
	}

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	let showUnused = false
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

	const entries: CustomCSSEntry[] = [
		{
			type: 'app',
			name: 'App',
			icon: LayoutDashboardIcon,
			ids: ['viewer', 'grid', 'component']
		},
		...Object.values(components).map(({ name, icon, data: { type, customCss }}) => ({
			type,
			name,
			icon,
			ids: Object.keys(customCss ?? {}) ?? []
		}))
	]
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

	$: staticComponents = entries.filter(({type}) => STATIC_ELEMENTS.includes(type as any))
	$: usedComponents = Object.keys(
		$app.grid.reduce((acc, {data}) => {
			acc[data.type] = true
			return acc
		}, {})
	).map(type => entries.find(entry => entry.type === type))
	.filter(Boolean) as CustomCSSEntry[]
	$: unusedComponents = entries.filter(({type}) => {
		return !STATIC_ELEMENTS.includes(type as any) && !usedComponents.find(entry => entry.type === type)
	})

	onMount(() => {
		isOpenStore.addItems(
			[{ name: 'App' }, ...Object.values(components)].map(component => {
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
			{#each [...staticComponents, ...usedComponents] as { type, name, icon, ids }}
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
									{#if $app?.css?.[type][id]}
										<CssProperty
											name={id}
											bind:value={$app.css[type][id]}
											on:focus={() => (isCustom[type] = true)}
										/>
									{/if}
								</div>
							{/each}
						</div>
					</ListItem>
				{/if}
			{/each}
			<div class="px-1 my-4">
				<button
					on:click|preventDefault|stopPropagation={() => {
						showUnused = !showUnused
					}}
					class="w-full text-xs text-gray-500 text-center font-medium hover:underline p-2"
				>
					{showUnused ? 'Hide' : 'Show'} unused components
				</button>
			</div>
			{#if showUnused}
				<div transition:slide={{ duration: 300 }}>
					{#each unusedComponents as { type, name, icon, ids }}
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
											{#if $app?.css?.[type][id]}
												<CssProperty
													name={id}
													bind:value={$app.css[type][id]}
													on:focus={() => (isCustom[type] = true)}
												/>
											{/if}
										</div>
									{/each}
								</div>
							</ListItem>
						{/if}
					{/each}
				</div>
			{/if}
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
