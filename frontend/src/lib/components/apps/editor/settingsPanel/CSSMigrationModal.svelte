<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, ComponentCssProperty } from '../../types'
	import { ccomponents, type AppComponent } from '../component'

	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	import { twMerge } from 'tailwind-merge'
	import { fade } from 'svelte/transition'
	import css from 'svelte-highlight/languages/css'
	import { Highlight } from 'svelte-highlight'
	import { MoveRight } from 'lucide-svelte'
	import { customisationByComponent } from '../componentsPanel/cssUtils'
	import { sendUserToast } from '$lib/toast'
	import CloseButton from '$lib/components/common/CloseButton.svelte'
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
	$: migrations = new Map<string, string[]>()

	function getSelector(key: string) {
		return customisationByComponent
			.find((c) => c.components.includes(component?.type ?? ''))
			?.selectors.find((s) => {
				return s.customCssKey === key
			})?.selector
	}

	function setOrUpdateMigration(key: string, value: string) {
		const selector = getSelector(key)
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
		const theme = $app.theme

		if (theme?.type === 'path') {
			sendUserToast(
				'Cannot migrate to CSS editor when using a theme by path. Fork a global theme to edit CSS.',
				true
			)
			return
		} else if (theme?.type === 'inlined') {
			let cssString = theme.css

			if (!cssString) {
				cssString = ''
			}

			for (const [key, value] of migrations) {
				if (cssString.includes(`${key} {` || `${key}{`)) {
					// append value to existing value
					const regex = new RegExp(`\\${key}\\s*{\\s*([\\s\\S]*?)\\s*}`, 'g')

					const match = regex.exec(cssString)

					if (match) {
						const existingValue = match[1]

						cssString = cssString.replace(
							regex,
							`${key} {\n\t${existingValue}\n\t${value.join('\n\t')}\n}`
						)
					}
				} else {
					const firstBreakline = cssString === '' ? '' : '\n\n'

					// append key and value
					cssString += `${firstBreakline}${key} {\n\t${value.join('\n\t')}\n}\n\n`
				}
			}

			theme.css = cssString

			$app.theme = theme
		}
	}

	function hasStyles(customCss: Record<string, ComponentCssProperty> | undefined) {
		if (!customCss) {
			return false
		}

		return Object.keys(customCss ?? {})
			.map((key) => customCss[key])
			.some((c) => c.style !== '')
	}

	let type: string | undefined = component?.type
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
					<div class="leading-6 font-semibold text-sm w-full flex justify-between">
						<div>Migrate to CSS editor</div><CloseButton
							on:close={() => (migrationModalOpen = false)}
						/>
					</div>

					<div class="">
						<div class="">
							{#if hasStyles(component?.customCss)}
								<div class="leading-6 text-xs font-semibold">
									ID <Badge color="indigo" size="xs">
										{component?.id}
									</Badge>
								</div>
							{/if}
							{#if component?.type && $app.css}
								{#each Object.keys(component.customCss ?? {}) as cssKey}
									{#if component.customCss?.[cssKey].style != undefined && component.customCss[cssKey].style !== ''}
										<div class="grid grid-cols-2 gap-2">
											<div>
												<div class="flex flex-row justify-between items-center py-0.5">
													<div class="leading-6 text-xs font-semibold">
														<Badge>
															{cssKey}
														</Badge>
													</div>
													<Button
														color="dark"
														size="xs"
														on:click={() => {
															if (component?.customCss?.[cssKey]?.style != undefined) {
																setOrUpdateMigration(
																	cssKey,
																	component.customCss[cssKey].style ?? ''
																)
																component.customCss[cssKey].style = ''
															}
														}}
														endIcon={{ icon: MoveRight }}
													>
														Migrate
													</Button>
												</div>
												<div class="border p-2 rounded-md">
													<Highlight code={component.customCss[cssKey].style} language={css} />
												</div>
											</div>
											<div class="">
												<div class="leading-6 text-xs font-semibold my-1">Preview</div>
												<div class="border rounded-md p-2">
													<Highlight
														code={`${getSelector(cssKey)} {\n\t${
															component.customCss[cssKey].style
														}\n}`}
														language={css}
													/>
												</div>
											</div>
										</div>
									{/if}
								{/each}
							{/if}

							{#if hasStyles(component?.type ? $app.css?.[component?.type] : undefined)}
								<div class="leading-6 text-xs font-semibold">
									Global: {component?.type ? ccomponents[component.type]?.name : ''}
								</div>
							{/if}

							{#if component?.type && $app.css}
								{#each Object.keys($app.css[component?.type] ?? {}) as cssKey}
									{#if type && $app.css?.[type]?.[cssKey].style != undefined && $app.css[type]?.[cssKey].style !== ''}
										<div class="grid grid-cols-2 gap-2">
											<div>
												<div class="flex flex-row justify-between items-center py-0.5">
													<div class="leading-6 text-xs font-semibold">
														<Badge>
															{cssKey}
														</Badge>
													</div>
													<Button
														color="dark"
														size="xs"
														on:click={() => {
															if (type && $app.css?.[type]) {
																setOrUpdateMigration(cssKey, $app.css[type][cssKey].style)
																$app.css[type][cssKey].style = ''
															}
														}}
														endIcon={{ icon: MoveRight }}
													>
														Migrate
													</Button>
												</div>
												<div class="border p-2 rounded-md">
													<Highlight code={$app.css[type][cssKey].style} language={css} />
												</div>
											</div>
											<div class="">
												<div class="leading-6 text-xs font-semibold my-1">Preview</div>
												<div class="border rounded-md p-2">
													<Highlight
														code={`${getSelector(cssKey)} {\n\t${$app.css[type][cssKey].style}\n}`}
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
					<div class="mt-2 flex flex-row justify-end items-center gap-2">
						<div class="text-xs">
							If the class is already present in the CSS editor, the migration will append the new
							values to the existing ones.
						</div>
						<Button
							size="xs"
							color="dark"
							on:click={() => {
								appendMigrationsToCss(migrations)
								migrationModalOpen = false
							}}
							disabled={migrations.size === 0}
							endIcon={{ icon: MoveRight }}
						>
							Apply migration
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
