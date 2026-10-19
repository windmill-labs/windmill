<script lang="ts">
	import LabelList from './common/badge/LabelList.svelte'
	import type { BadgeColor } from './common/badge/model'
	import { Folder } from 'lucide-svelte'

	interface Props {
		labels: string[] | undefined
		max?: number
		color?: BadgeColor
	}

	let { labels, max, color }: Props = $props()
</script>

<LabelList {labels} icon={Folder} titlePrefix="Label inherited from folder" {max} {color} />
