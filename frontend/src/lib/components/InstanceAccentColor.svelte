<script lang="ts">
	import { get } from 'svelte/store'
	import { SettingService } from '$lib/gen'
	import { enterpriseLicense } from '$lib/stores'
	import { setLicense } from '$lib/enterpriseUtils'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { ACCENT_COLOR_SETTING, applyAccentColor, parseAccentColor } from '$lib/accentColor'
	import { instanceSettingsSaved } from './instanceSettings'

	// Painted on mount, before the license and setting requests return, so a reload
	// does not flash the default blue. Only written on a licensed instance, and cleared
	// once the license check says there is none.
	const cached = useLocalStorageValue<string>('instance_accent_color', '', 'string')

	let color = $state(parseAccentColor(cached.val))

	let latestLoad = 0
	async function load() {
		const generation = ++latestLoad
		try {
			// `enterpriseLicense` stays unset both while loading and on CE, so resolve it here.
			await setLicense()
			const next = get(enterpriseLicense)
				? parseAccentColor(await SettingService.getGlobal({ key: ACCENT_COLOR_SETTING }))
				: undefined
			if (generation !== latestLoad) return
			color = next
			cached.val = next ?? ''
		} catch (e) {
			console.warn('Could not fetch the instance accent color', e)
		}
	}

	$effect(() => {
		$instanceSettingsSaved
		load()
	})

	$effect(() => {
		applyAccentColor(color)
		return () => applyAccentColor(undefined)
	})
</script>
