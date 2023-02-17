<script lang="ts">
	import { Script } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import type { BadgeColor } from '../badge/model'
	import Badge from '../badge/Badge.svelte'
	import { LanguageIcon } from '../languageIcons'
	import Popover from '$lib/components/Popover.svelte'

	export let language: Script.language

	const languageLabel = {
		[Script.language.PYTHON3]: 'Python',
		[Script.language.DENO]: 'TypeScript',
		[Script.language.GO]: 'Go',
		[Script.language.BASH]: 'Bash'
	}[language]

	const baseClassByLanguage = {
		[Script.language.PYTHON3]: 'border border-[#ffe082] !bg-white',
		[Script.language.DENO]: 'border border-blue-200 !bg-white',
		[Script.language.GO]: 'border border-[#80cbc4] !bg-white',
		[Script.language.BASH]: 'border border-gray-400 !bg-white'
	}[language]

	const colorByLanguage = {
		[Script.language.PYTHON3]: 'gray',
		[Script.language.DENO]: 'gray',
		[Script.language.GO]: 'gray',
		[Script.language.BASH]: 'gray'
	}[language] as BadgeColor
</script>

<Badge
	color={colorByLanguage}
	baseClass={classNames(baseClassByLanguage, 'flex gap-1 items-center')}
>
	<Popover notClickable>
		<LanguageIcon lang={language} width={12} height={12} />
		<span slot="text">
			{languageLabel}
		</span>
	</Popover>

	<!-- <span class="hidden xl:inline">
	</span> -->
</Badge>
