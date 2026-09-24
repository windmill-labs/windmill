<script lang="ts">
	import { SettingService } from '$lib/gen'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { ACCENT_COLOR_SETTING, applyAccentColor, parseAccentColor } from '$lib/accentColor'
	import { instanceSettingsSaved } from './instanceSettings'

	// Last color seen, so a reload paints the right accent before the fetch returns
	// instead of flashing the default blue.
	const cached = useLocalStorageValue<string>('instance_accent_color', '', 'string')

	let color = $state(parseAccentColor(cached.val))

	let latestLoad = 0
	async function load() {
		const generation = ++latestLoad
		try {
			const next = parseAccentColor(await SettingService.getGlobal({ key: ACCENT_COLOR_SETTING }))
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
