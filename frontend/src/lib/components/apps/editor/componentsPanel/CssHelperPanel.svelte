<script lang="ts">
	import { getContext } from 'svelte'
	import { LayoutDashboardIcon, ArrowUpSquare, ExternalLink } from 'lucide-svelte'
	import { Badge, Button, ClearableInput, Tab, TabContent, Tabs } from '../../../common'
	import type { AppViewerContext } from '../../types'
	import ListItem from './ListItem.svelte'
	import { ccomponents, components } from '../component'
	import { customisationByComponent } from './cssUtils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	const STATIC_ELEMENTS = ['app'] as const
	const TITLE_PREFIX = 'Css.' as const

	type CustomCSSType = (typeof STATIC_ELEMENTS)[number] | keyof typeof components

	interface CustomCSSEntry {
		type: CustomCSSType
		name: string
		icon: any
		ids: { id: string; forceStyle: boolean; forceClass: boolean }[]
	}

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	const entries: CustomCSSEntry[] = [
		{
			type: 'app',
			name: 'App',
			icon: LayoutDashboardIcon,
			ids: ['viewer', 'grid', 'component'].map((id) => ({ id, forceStyle: true, forceClass: true }))
		},
		...Object.entries(ccomponents).map(([type, { name, icon, customCss }]) => ({
			type: type as keyof typeof components,
			name,
			icon,
			ids: Object.entries(customCss).map(([id, v]) => ({
				id,
				forceStyle: v?.style != undefined,
				forceClass: v?.['class'] != undefined
			}))
		}))
	]
	entries.sort((a, b) => a.name.localeCompare(b.name))
	let search = ''
</script>

<div class="p-2">
	<ClearableInput bind:value={search} placeholder="Search..." />
</div>
<div class="h-[calc(100%-50px)] overflow-auto relative">
	{#each search != '' ? entries.filter((x) => x.name
					.toLowerCase()
					.includes(search.toLowerCase())) : entries as { type, name, icon, ids } (name + type)}
		{#if ids.length > 0}
			<ListItem
				title={name}
				prefix={TITLE_PREFIX}
				on:open={(e) => {
					if ($app.css != undefined) {
						if (e.detail && $app.css[type] == undefined) {
							$app.css[type] = Object.fromEntries(ids.map(({ id }) => [id, {}]))
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
				<div class="py-2">
					{#each customisationByComponent.filter( (c) => c.components.includes(type) ) as customisation (customisation.components.join('-'))}
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
										<Tooltip light>{customisation.variablesTooltip}</Tooltip>
									</div>
								</Tab>
							{/if}
							<div slot="content" class="h-full">
								<TabContent value="selectors" class="h-full mt-2">
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
													<Button size="xs2" color="light">
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
														<div class="max-w-24 whitespace-pre-wrap">{comment}</div>
													{/if}
												</Cell>
												<Cell>
													<Button size="xs2" color="light">
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
			</ListItem>
		{/if}
	{/each}
</div>
