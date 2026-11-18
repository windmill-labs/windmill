<script lang="ts">
	import { get } from 'svelte/store'
	import { SettingService, SettingsService } from '$lib/gen'
	import { enterpriseLicense } from '$lib/stores'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { applyAccentColor, parseAccentColor } from '$lib/accentColor'
	import { instanceUi } from '$lib/instanceUi'
	import { instanceSettingsSaved } from './instanceSettings'

	// The only fetcher of `/settings/instance_ui`: the banner renders from the store this
	// fills. Loaded once per full page load (and after a settings save), not polled: a
	// change reaches other sessions on their next reload.

	// Painted on mount, before the license and settings requests return, so a reload does
	// not flash the default blue. Only written on a licensed instance, and cleared once the
	// license check says there is none.
	const cached = useLocalStorageValue<string>('instance_accent_color', '', 'string')

	let color = $state(parseAccentColor(cached.val))

	// Successive saves can refetch concurrently, and responses are not ordered: without
	// this a slow earlier fetch lands last and puts a retracted announcement back on screen.
	let latestLoad = 0

	async function load() {
		const generation = ++latestLoad
		try {
			// `enterpriseLicense` stays unset both while loading and on CE, so ask directly.
			// Not through `setLicense()`: it swallows a failed request, which would read as CE
			// and retract the banner and accent of a licensed instance.
			const license = get(enterpriseLicense) || (await SettingsService.getLicenseId())
			if (license && !get(enterpriseLicense)) enterpriseLicense.set(license)
			const next = license ? await SettingService.getInstanceUi() : undefined
			if (generation !== latestLoad) return
			instanceUi.set(next)
			color = parseAccentColor(next?.accent_color)
			cached.val = color ?? ''
		} catch (e) {
			// Keep whatever is on screen: a transient failure must not silently retract
			// an announcement that is still in force.
			console.warn('Could not fetch the instance banner and accent color', e)
		}
	}

	$effect(() => {
		// Re-runs on save so a superadmin sees their change without reloading.
		$instanceSettingsSaved
		load()
	})

	$effect(() => {
		applyAccentColor(color)
		return () => applyAccentColor(undefined)
	})
</script>
