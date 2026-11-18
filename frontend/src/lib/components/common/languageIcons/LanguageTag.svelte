<script lang="ts">
	import { languageLabel, languageShortLabel, type LabelledLanguage } from './languageLabels'

	let { lang }: { lang: LabelledLanguage } = $props()
</script>

<!-- Quiet on purpose: it sits beside a title, and the title is what the eye is reading. -->
<span
	class="shrink-0 text-3xs font-medium text-hint tracking-wide"
	title={languageLabel[lang] ?? lang}>{languageShortLabel[lang] ?? lang}</span
>
