<script lang="ts">
	import { getContext } from 'svelte'
	import { LayoutDashboardIcon, ArrowUpSquare, ExternalLink, TextCursorInput } from 'lucide-svelte'
	import { Badge, Button, ClearableInput, Tab, TabContent, Tabs } from '../../../common'
	import type { AppViewerContext } from '../../types'
	import ListItem from './ListItem.svelte'
	import { ccomponents, components } from '../component'
	import { customisationByComponent } from './cssUtils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { createEventDispatcher } from 'svelte'

	const STATIC_ELEMENTS = ['app'] as const
	const TITLE_PREFIX = 'Css.' as const

	type CustomCSSType = (typeof STATIC_ELEMENTS)[number] | keyof typeof components

	const dispatch = createEventDispatcher()

	interface CustomCSSEntry {
		type?: CustomCSSType
		name: string
		icon: any
		ids?: { id: string; forceStyle: boolean; forceClass: boolean }[]
		description?: string
		order?: number
	}

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	const descriptions = {
		buttoncomponent:
			'The button component also has additional color specific classes to allow customizing classes by color. wm-button-wrapper-blue, wm-button-container-blue, ...'
	}
	const entries: CustomCSSEntry[] = [
		{
			name: 'Dark Mode',
			icon: LayoutDashboardIcon,
			description:
				'When in dark mode, the entire document has the .dark class applied to it. You can apply selective styling by using the .dark class: e.g. .dark .my-element { color: white; }',
			order: 3
		},
		{
			type: 'app',
			name: 'App',
			icon: LayoutDashboardIcon,
			ids: ['viewer', 'grid', 'component'].map((id) => ({
				id,
				forceStyle: true,
				forceClass: true
			})),
			order: 2
		},
		{
			type: 'quillcomponent',
			name: 'Rich Text Editor',
			icon: TextCursorInput,
			ids: ['q'].map((id) => ({ id, forceStyle: true, forceClass: true }))
		},
		...Object.entries(ccomponents)
			.filter(([key]) => key !== 'quillcomponent')
			.map(([type, { name, icon, customCss }]) => ({
				type: type as keyof typeof components,
				name,
				icon,
				ids: Object.entries(customCss).map(([id, v]) => ({
					id,
					forceStyle: v?.style != undefined,
					forceClass: v?.['class'] != undefined
				})),
				description: descriptions[type as keyof typeof descriptions]
			}))
	]

	entries.sort((a, b) => (b.order ?? 0) - (a.order ?? 0) + a.name.localeCompare(b.name))

	let search = ''
</script>

<div class="p-2">
	<ClearableInput bind:value={search} placeholder="Search..." />
</div>
<div class="h-[calc(100%-50px)] overflow-auto relative">
	{#each search != '' ? entries.filter((x) => x.name
					.toLowerCase()
					.includes(search.toLowerCase())) : entries as { type, name, icon, ids, description } (name + type)}
		{#if description || (ids && ids.length > 0)}
			<ListItem
				title={name}
				prefix={TITLE_PREFIX}
				on:open={(e) => {
					if ($app.css != undefined) {
						if (type && e.detail && $app.css[type] == undefined) {
							$app.css[type] = Object.fromEntries((ids ?? []).map(({ id }) => [id, {}]))
						}
					}
				}}
			>
				<div slot="title" class="flex items-center">
					<svelte:component this={icon} size={18} />
					<span class="ml-1">
						{name}
					</span>
				</div>
				{#if description}
					<div class="py-2 text-xs text-gray-500">{description}</div>
				{/if}
				{#if type}
					<div class="py-2">
						{#each customisationByComponent.filter( (c) => c.components.includes(type) ) as customisation (customisation.components.join('-'))}
							{#if customisation.link}
								<a
									href={customisation.link}
									target="_blank"
									class="text-frost-500 dark:text-frost-300 font-semibold text-xs"
								>
									<div class="flex flex-row gap-2">
										See documentation
										<ExternalLink size="16" />
									</div>
								</a>
							{/if}

							<Tabs selected="selectors">
								{#if customisation.selectors.length > 0}
									<Tab value="selectors" size="xs">
										Selectors ({customisation.selectors.length})
									</Tab>
								{/if}
								{#if customisation.variables.length > 0}
									<Tab value="variables" size="xs">
										<div class="flex flex-row gap-2 justify-center-center items-center">
											Variables ({customisation.variables.length})
										</div>
									</Tab>
								{/if}
								<div slot="content" class="h-full">
									<TabContent value="selectors" class="h-full mt-2 ">
										<DataTable size="sm">
											<Head>
												<tr>
													<Cell head first>Selector</Cell>
													<Cell head>Comment</Cell>
													<Cell head last />
												</tr>
											</Head>
											{#each customisation.selectors as { selector, comment }}
												<Row>
													<Cell first>
														<Badge color="gray">{selector}</Badge>
													</Cell>
													<Cell>
														{#if comment}
															<div class="max-w-24 whitespace-pre-wrap">{comment}</div>
														{/if}
													</Cell>
													<Cell>
														<Button
															size="xs2"
															color="light"
															on:click={() => {
																dispatch('insertSelector', `${selector} {}`)
															}}
														>
															<ArrowUpSquare size={16} />
														</Button>
													</Cell>
												</Row>
											{/each}
										</DataTable>
									</TabContent>
									<TabContent value="variables" class="h-full mt-2">
										<DataTable>
											<Head>
												<tr>
													<Cell head first>Variable</Cell>
													<Cell head>Default value</Cell>
													<Cell head>Comment</Cell>
													<Cell head last />
												</tr>
											</Head>
											{#each customisation.variables as { variable, value, comment }}
												<Row>
													<Cell first>
														<Badge color="gray">{variable}</Badge>
													</Cell>
													<Cell>
														<Badge color="gray">{value}</Badge>
													</Cell>
													<Cell>
														{#if comment}
															<div class="w-80 whitespace-pre-wrap">{comment}</div>
														{/if}
													</Cell>
													<Cell sticky>
														<Button
															size="xs2"
															color="light"
															on:click={() => {
																dispatch(
																	'insertSelector',
																	`${customisation.root} { ${variable}: ${value};}`
																)
															}}
															wrapperClasses="px-2 py-3.5 bg-surface"
														>
															<ArrowUpSquare size={16} />
														</Button>
													</Cell>
												</Row>
											{/each}
										</DataTable>
									</TabContent>
								</div>
							</Tabs>
						{/each}
					</div>
				{/if}
			</ListItem>
		{/if}
	{/each}
</div>
