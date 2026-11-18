<script lang="ts">
	import { Folder, Tag } from 'lucide-svelte'
	import LabelList from '../badge/LabelList.svelte'

	let { labels, inherited }: { labels?: string[] | null; inherited: string[] | undefined } =
		$props()
</script>

<!-- A row has little room beside its title: one of each kind shows, the rest are counted. -->
<LabelList {labels} icon={Tag} titlePrefix="Label" max={1} color="subtle" />
<LabelList
	labels={inherited}
	icon={Folder}
	titlePrefix="Label inherited from folder"
	max={1}
	color="subtle"
/>
