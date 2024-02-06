<script lang="ts">
	import { Paintbrush } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import { formatOnSave } from '$lib/stores'
	import Popover from './Popover.svelte'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import PaintbrushOff from './icons/PaintbrushOff.svelte'

	const SETTING_NAME = 'formatOnSave'
	function loadSetting() {
		$formatOnSave = (getLocalSetting(SETTING_NAME) ?? 'true') == 'true'
	}

	function storeSetting() {
		$formatOnSave = !$formatOnSave
		storeLocalSetting(SETTING_NAME, $formatOnSave.toString())
	}

	loadSetting()
</script>

<Popover>
	<svelte:fragment slot="text"
		>{$formatOnSave ? 'Disable' : 'Enable'} auto-formatting</svelte:fragment
	>
	<Button
		size="xs"
		color="light"
		startIcon={{
			icon: !$formatOnSave ? PaintbrushOff : Paintbrush
		}}
		btnClasses="text-tertiary"
		on:click={() => {
			storeSetting()
		}}
	/>
</Popover>
