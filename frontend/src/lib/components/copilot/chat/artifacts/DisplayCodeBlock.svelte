<script lang="ts">
	import MarkdownCodeBlock from '$lib/components/MarkdownCodeBlock.svelte'
</script>

<!-- A `pre` renderer for untrusted artifact markdown: ```wm-display fences render (the fence
     admits only inert kinds), mermaid stays source. -->
<MarkdownCodeBlock renderDisplay />
