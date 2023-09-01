<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { ccomponents, type AppComponent } from '../component'

	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	import { twMerge } from 'tailwind-merge'
	import { fade } from 'svelte/transition'
	import css from 'svelte-highlight/languages/css'
	import { Highlight } from 'svelte-highlight'
	import { MoveRight } from 'lucide-svelte'
	import { customisationByComponent } from '../componentsPanel/cssUtils'
	export let component: AppComponent | undefined

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}
	let migrationModalOpen: boolean = false

	export function open() {
		migrationModalOpen = true
	}

	function generateCodeFromMigrations(migrations: Map<string, string[]>) {
		let code = ''
		for (const [key, value] of migrations) {
			code += `${key} {\n\t${value.join(';\n\t')}\n}\n\n`
		}

		return code
	}

	$: generatedCode = generateCodeFromMigrations(migrations)

	$: console.log(generatedCode)

	$: migrations = new Map<string, string[]>()

	function setOrUpdateMigration(key: string, value: string) {
		const selector = customisationByComponent
			.find((c) => c.components.includes(component?.type ?? ''))
			?.selectors.find((s) => {
				return s.customCssKey === key
			})?.selector

		if (!selector) {
			return
		}

		if (migrations.has(selector)) {
			const arr = migrations.get(selector)

			if (arr) {
				arr.push(value)

				migrations.set(selector, arr)
			}
		} else {
			migrations.set(selector, [value])
		}

		migrations = migrations
	}

	function appendMigrationsToCss(migrations: Map<string, string[]>) {
		let cssString = $app.cssString

		if (!cssString) {
			cssString = ''
		}

		for (const [key, value] of migrations) {
			if (cssString.includes(key)) {
				// append value to existing value
				const regex = new RegExp(`\\${key}\\s*{\\s*([\\s\\S]*?)\\s*}`, 'g')

				const match = regex.exec(cssString)

				if (match) {
					const existingValue = match[1]

					cssString = cssString.replace(
						regex,
						`.${key} {\n\t${existingValue}\n\t${value.join('\n\t')}\n}`
					)
				}
			} else {
				// append key and value
				cssString += `${key} {\n\t${value.join('\n\t')}\n}\n\n`
			}
		}

		$app.cssString = cssString
	}
</script>

{#if migrationModalOpen}
	<div
		transition:fadeFast|local
		class={'absolute top-0 bottom-0 left-0 right-0 z-[5000]'}
		role="dialog"
	>
		<div
			class={twMerge(
				'fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
				migrationModalOpen ? 'ease-out duration-300 opacity-100' : 'ease-in duration-200 opacity-0'
			)}
		/>

		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class={twMerge(
						'relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-5xl sm:p-6 ',
						migrationModalOpen
							? 'ease-out duration-300 opacity-100 translate-y-0 sm:scale-100'
							: 'ease-in duration-200 opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95'
					)}
				>
					<div class="leading-6 font-semibold text-sm"> Migrate to CSS editor </div>
					<div class="">
						<div class="">
							<div class="leading-6 text-xs font-semibold"
								>ID <Badge color="indigo" size="xs">
									{component?.id}
								</Badge>
							</div>

							{#if component?.type && $app.css}
								{#each Object.keys(component.customCss ?? {}) as x}
									{#if component.customCss[x].style != undefined && component.customCss[x].style !== ''}
										<div class="grid grid-cols-2 gap-2">
											<div>
												<div class="flex flex-row justify-between items-center py-0.5">
													<div class="leading-6 text-xs font-semibold">
														<Badge>
															{x}
														</Badge>
													</div>
													<Button
														color="light"
														size="xs"
														on:click={() => {
															setOrUpdateMigration(x, component.customCss[x].style)

															component.customCss[x].style = ''
														}}
													>
														<div class="flex flex-row gap-2 items-center">
															Migrate
															<MoveRight size={16} />
														</div>
													</Button>
												</div>
												<div class="border p-2 rounded-md">
													<Highlight code={component.customCss[x].style} language={css} />
												</div>
											</div>
											<div class="">
												<div class="leading-6 text-xs font-semibold my-1">Preview</div>
												<div class="border rounded-md p-2">
													<Highlight
														code={`.wm-button {\n\t${component.customCss[x].style}\n}`}
														language={css}
													/>
												</div>
											</div>
										</div>
									{/if}
								{/each}
							{/if}

							<div class="leading-6 text-xs font-semibold">
								Global: {component?.type ? ccomponents[component.type]?.name : ''}
							</div>

							{#if component?.type && $app.css}
								{#each Object.keys($app.css[component?.type] ?? {}) as x}
									{#if $app.css[component?.type][x].style != undefined && $app.css[component?.type][x].style !== ''}
										<div class="grid grid-cols-2 gap-2">
											<div>
												<div class="flex flex-row justify-between items-center py-0.5">
													<div class="leading-6 text-xs font-semibold">
														<Badge>
															{x}
														</Badge>
													</div>
													<Button
														color="light"
														size="xs"
														on:click={() => {
															setOrUpdateMigration(x, $app.css[component?.type][x].style)
															$app.css[component?.type][x].style = ''
														}}
													>
														<div class="flex flex-row gap-2 items-center">
															Migrate
															<MoveRight size={16} />
														</div>
													</Button>
												</div>
												<div class="border p-2 rounded-md">
													<Highlight code={$app.css[component?.type][x].style} language={css} />
												</div>
											</div>
											<div class="">
												<div class="leading-6 text-xs font-semibold my-1">Preview</div>
												<div class="border rounded-md p-2">
													<Highlight
														code={`.wm-button {\n\t${$app.css[component?.type][x].style}\n}`}
														language={css}
													/>
												</div>
											</div>
										</div>
									{/if}
								{/each}
							{/if}
						</div>
					</div>
					<div>
						<div class="leading-6 text-xs font-semibold my-1">Current migrations</div>
						{#if migrations.size > 0}
							<div class="border rounded-md p-2">
								<Highlight code={generatedCode} language={css} />
							</div>
						{:else}
							<div class="text-gray-500 text-xs">No migrations</div>
						{/if}
					</div>
					<div class="mt-2 w-64 float-right">
						<Button
							size="xs"
							color="dark"
							on:click={() => {
								appendMigrationsToCss(migrations)
								migrationModalOpen = false
							}}
						>
							<div class="flex flex-row gap-2 items-center">
								Apply migration
								<MoveRight size={16} />
							</div>
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
