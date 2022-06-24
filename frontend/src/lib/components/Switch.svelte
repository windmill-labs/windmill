<script>
	export let checked = false
	export let label = ''
	export let horizontal = true // Determines if the switch and label are on a row or col
	export let textFormat = 'text-base' // Tailwind CSS classes to format the text
</script>

<div class="flex {horizontal ? 'flex-row justify-between' : 'flex-col'}  {$$props.class}">
	<div class="my-auto {textFormat} mr-2">{label}</div>
	<label class="switch my-auto">
		<input type="checkbox" bind:checked />
		<span class="slider" />
	</label>
</div>

<style>
	.switch {
		position: relative;
		display: inline-block;
		width: 30px;
		height: 17px;
	}

	.switch input {
		opacity: 0;
		width: 0;
		height: 0;
	}

	.slider {
		position: absolute;
		cursor: pointer;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: #ccc;
		-webkit-transition: 0.4s;
		transition: 0.4s;
		border-radius: 17px;
	}

	.slider:before {
		position: absolute;
		content: '';
		height: 13px;
		width: 13px;
		left: 2px;
		bottom: 2px;
		background-color: white;
		-webkit-transition: 0.4s;
		transition: 0.4s;
		border-radius: 50%;
	}

	input:checked + .slider {
		background-color: #2196f3;
	}

	input:checked + .slider {
		box-shadow: 0 0 1px #2196f3;
	}

	input:checked + .slider:before {
		-webkit-transform: translateX(13px);
		-ms-transform: translateX(13px);
		transform: translateX(13px);
	}
</style>
