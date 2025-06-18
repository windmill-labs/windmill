<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { parseISO, format as formatDateFns } from 'date-fns'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { twMerge } from 'tailwind-merge'
	import { enUS, fr, de, pt, ja } from 'date-fns/locale'
	import Select from '$lib/components/select/Select.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'dateselectcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['dateselectcomponent'].initialData.configuration,
		configuration
	)

	let value: string | undefined = undefined

	$componentControl[id] = {
		setValue(nvalue: string) {
			value = nvalue
		}
	}

	let outputs = initOutput($worldStore, id, {
		day: undefined as number | undefined,
		month: undefined as number | undefined,
		year: undefined as number | undefined
	})

	$: !value && handleDefault(resolvedConfig.defaultValue)

	function getLocale(locale: string = 'en-US') {
		const localeMapping: { [key: string]: Locale } = {
			'en-US': enUS,
			'en-GB': enUS,
			'en-IE': enUS,
			'de-DE': de,
			'fr-FR': fr,
			'br-FR': fr,
			'ja-JP': ja,
			'pt-TL': pt,
			'fr-CA': fr,
			'en-CA': enUS
		}
		return localeMapping[resolvedConfig?.locale as string] || enUS
	}

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue

		if (value) {
			const date = parseISO(value)
			const locale = getLocale(resolvedConfig.locale)
			selectedDay = String(date.getDate())
			selectedMonth = formatDateFns(date, 'MMMM', { locale })
			selectedYear = String(date.getFullYear())

			if (resolvedConfig?.enableDay) {
				outputs.day.set(Number(selectedDay))
			}

			if (resolvedConfig?.enableMonth) {
				outputs.month.set(date.getMonth() + 1)
			}

			if (resolvedConfig?.enableYear) {
				outputs.year.set(Number(selectedYear))
			}
		}
	}

	let css = initCss($app.css?.dateinputcomponent, customCss)

	let selectedDay: string | undefined = undefined
	let selectedMonth: string | undefined = undefined
	let selectedYear: string | undefined = undefined

	$: monthItems = computeMonthItems(resolvedConfig?.locale)

	function updateOutputs(enableDay?: boolean, enableMonth?: boolean, enableYear?: boolean) {
		if (enableDay) {
			outputs.day.set(Number(selectedDay))
		} else {
			outputs.day.set(undefined)
		}

		if (enableMonth) {
			const monthIndex = monthItems.findIndex((item) => item.label === selectedMonth)

			outputs.month.set(monthIndex + 1)
		} else {
			outputs.month.set(undefined)
		}

		if (enableYear) {
			outputs.year.set(Number(selectedYear))
		} else {
			outputs.year.set(undefined)
		}
	}

	$: updateOutputs(resolvedConfig.enableDay, resolvedConfig.enableMonth, resolvedConfig.enableYear)

	function computeMonthItems(locale: string = 'en-US') {
		return [
			{
				label: formatDateFns(new Date(2000, 0, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'January'
			},
			{
				label: formatDateFns(new Date(2000, 1, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'February'
			},
			{
				label: formatDateFns(new Date(2000, 2, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'March'
			},
			{
				label: formatDateFns(new Date(2000, 3, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'April'
			},
			{
				label: formatDateFns(new Date(2000, 4, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'May'
			},
			{
				label: formatDateFns(new Date(2000, 5, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'June'
			},
			{
				label: formatDateFns(new Date(2000, 6, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'July'
			},
			{
				label: formatDateFns(new Date(2000, 7, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'August'
			},
			{
				label: formatDateFns(new Date(2000, 8, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'September'
			},
			{
				label: formatDateFns(new Date(2000, 9, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'October'
			},
			{
				label: formatDateFns(new Date(2000, 10, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'November'
			},
			{
				label: formatDateFns(new Date(2000, 11, 1), 'MMMM', { locale: getLocale(locale) }),
				value: 'December'
			}
		]
	}

	function computeDayPerMonth(selectedMonth: string | undefined, selectedYear: string | undefined) {
		if (!selectedMonth || !selectedYear) {
			return 31
		}

		const monthIndex =
			[
				'January',
				'February',
				'March',
				'April',
				'May',
				'June',
				'July',
				'August',
				'September',
				'October',
				'November',
				'December'
			].indexOf(selectedMonth) + 1

		const daysInMonth = new Date(Number(selectedYear), monthIndex, 0).getDate()

		return daysInMonth
	}
</script>

{#each Object.keys(components['dateselectcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.dateselectcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	<div
		class={twMerge(
			'w-full',
			resolvedConfig?.orientation === 'horizontal'
				? 'flex flex-row gap-2  '
				: 'flex  gap-2 flex-col',
			css?.container?.class
		)}
		style={css?.container?.style}
	>
		{#if resolvedConfig?.enableDay}
			<div
				class={twMerge('grow', resolvedConfig?.orientation === 'horizontal' ? 'w-1/4' : 'w-full')}
			>
				<Select
					bind:value={
						() => selectedDay,
						(v) => {
							selectedDay = v ?? ''
							outputs.day.set(v ? Number(v) : undefined)
						}
					}
					items={Array.from(
						{ length: computeDayPerMonth(selectedMonth, selectedYear) },
						(_, i) => ({ value: String(i + 1) })
					)}
					class={twMerge('text-clip min-w-0', css?.input?.class, 'wm-date-select')}
					containerStyle={css?.input?.style}
					placeholder="Pick a day"
					clearable
				/>
			</div>
		{/if}
		{#if resolvedConfig?.enableMonth}
			<div
				class={twMerge('grow', resolvedConfig?.orientation === 'horizontal' ? 'w-1/2' : 'w-full')}
			>
				<Select
					bind:value={
						() => selectedMonth ?? '',
						(v) => {
							selectedMonth = v
							outputs.month.set(monthItems.findIndex((item) => item.value === selectedMonth) + 1)
						}
					}
					items={monthItems}
					placeholder="Pick a month"
					class={twMerge('text-clip min-w-0', css?.input?.class, 'wm-date-select')}
					containerStyle={css?.input?.style}
					clearable
				/>
			</div>
		{/if}
		{#if resolvedConfig?.enableYear}
			<div
				class={twMerge('grow', resolvedConfig?.orientation === 'horizontal' ? 'w-1/4' : 'w-full')}
			>
				<Select
					bind:value={
						() => selectedYear,
						(v) => {
							selectedYear = v ?? ''
							outputs.year.set(selectedYear ? Number(selectedYear) : undefined)
						}
					}
					items={safeSelectItems(Array.from({ length: 201 }, (_, i) => `${1900 + i}`))}
					placeholder="Pick a year"
					class={twMerge('text-clip min-w-0', css?.input?.class, 'wm-date-select')}
					containerStyle={css?.input?.style}
					clearable
				/>
			</div>
		{/if}
	</div>
</AlignWrapper>
