<script lang="ts">
	export let value: string
	export let placeholder = ''
	export let minRows = 1
	export let maxRows = 20

	$: minHeight = `${1 + minRows * 1.2}em`
	$: maxHeight = maxRows ? `${1 + maxRows * 1.2}em` : `auto`
</script>

<div class="container">
	<pre aria-hidden="true" style="min-height: {minHeight}; max-height: {maxHeight}">
		{value + '\n'}
	</pre>
	<textarea type="text" class="col-span-10 resize" bind:value {placeholder} />
</div>

<style>
	.container {
		position: relative;
	}

	pre,
	textarea {
		font-family: inherit;
		padding: 0.5em;
		box-sizing: border-box;
		line-height: 1.2;
		overflow: hidden;
	}
	textarea {
		position: absolute;
		width: 100%;
		height: 100%;
		top: 0;
	}
</style>
