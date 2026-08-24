<!-- #1558BC / #1558BC per Atlassian Design logo library (atlassian.design/foundations/logos → trello_app.zip, Trello_icon.svg). Atlassian: never compose your own versions or deconstruct official assets. -->
<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg
	{width}
	{height}
	viewBox="-2.182 -2.182 52.364 52.364"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	preserveAspectRatio="xMidYMid"
	><path
		d="M0 12C0 5.37258 5.37258 0 12 0H36C42.6274 0 48 5.37258 48 12V36C48 42.6274 42.6274 48 36 48H12C5.37258 48 0 42.6274 0 36V12Z"
		fill="#1558BC"
	/><path
		d="M19.3456 36C20.2788 36 21.0362 35.263 21.0362 34.3519V13.6481C21.0362 12.7383 20.2801 12 19.3456 12H12.1906C11.2574 12 10.5 12.737 10.5 13.6481V34.3519C10.5 35.2617 11.2561 36 12.1906 36H19.3456Z"
		fill="white"
	/><path
		d="M35.8094 26.4952C36.7426 26.4952 37.5 25.7582 37.5 24.8471V13.6481C37.5 12.7383 36.744 12 35.8094 12H28.6544C27.7212 12 26.9638 12.737 26.9638 13.6481V24.8471C26.9638 25.7569 27.7199 26.4952 28.6544 26.4952H35.8094Z"
		fill="white"
	/></svg
>
