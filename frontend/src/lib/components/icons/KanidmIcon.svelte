<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 150 150" x="0px" y="0px" {width} {height}
	><defs
		><radialGradient id="a" gradientUnits="userSpaceOnUse" cx="420.208" cy="346.897" r="21.1"
			><stop offset="0" stop-color="#B1DEF4" /><stop
				offset="1"
				stop-color="#10A7CE"
			/></radialGradient
		><radialGradient id="b" gradientUnits="userSpaceOnUse" cx="385.794" cy="98.971" r="165.234"
			><stop offset="0" stop-color="#B1DEF4" /><stop
				offset="1"
				stop-color="#10A7CE"
			/></radialGradient
		><radialGradient id="c" gradientUnits="userSpaceOnUse" cx="154.649" cy="435.464" r="21.1"
			><stop offset="0" stop-color="#F7B6D3" /><stop
				offset="1"
				stop-color="#FF5D73"
			/></radialGradient
		><radialGradient id="d" gradientUnits="userSpaceOnUse" cx="120.117" cy="187.538" r="165.234"
			><stop offset="0" stop-color="#F7B6D3" /><stop
				offset="1"
				stop-color="#FF5D73"
			/></radialGradient
		><radialGradient id="e" gradientUnits="userSpaceOnUse" cx="660.417" cy="435.464" r="21.1"
			><stop offset="0" stop-color="#F7AEB4" /><stop
				offset="1"
				stop-color="#F60012"
			/></radialGradient
		><radialGradient id="f" gradientUnits="userSpaceOnUse" cx="625.885" cy="187.538" r="165.234"
			><stop offset="0" stop-color="#F7AEB4" /><stop
				offset="1"
				stop-color="#F60012"
			/></radialGradient
		><radialGradient id="g" gradientUnits="userSpaceOnUse" cx="283.389" cy="577.779" r="21.1"
			><stop offset="0" stop-color="#FBDEAD" /><stop
				offset="1"
				stop-color="#FF9800"
			/></radialGradient
		><radialGradient id="h" gradientUnits="userSpaceOnUse" cx="213.711" cy="337.353" r="165.234"
			><stop offset="0" stop-color="#FBDEAD" /><stop
				offset="1"
				stop-color="#FF9800"
			/></radialGradient
		><radialGradient id="B" gradientUnits="userSpaceOnUse" cx="421.086" cy="879.306" r="5.828"
			><stop offset="0" stop-color="#E61F19" /><stop offset=".988" stop-color="#9E282C" /><stop
				offset="1"
				stop-color="#9E282C"
			/></radialGradient
		><radialGradient
			id="k"
			gradientUnits="userSpaceOnUse"
			cx="146.832"
			cy="-316.119"
			r="53.614"
			gradientTransform="scale(1 -1)"
			><stop offset="0" stop-color="#FFF" /><stop
				offset="1"
				stop-color="#FFF"
				stop-opacity=".001"
			/></radialGradient
		><radialGradient
			id="l"
			gradientUnits="userSpaceOnUse"
			cx="55.175"
			cy="-176.464"
			r="53.614"
			gradientTransform="scale(1 -1)"
			><stop offset="0" stop-color="#FFF" /><stop
				offset="1"
				stop-color="#FFF"
				stop-opacity=".001"
			/></radialGradient
		><radialGradient
			id="m"
			gradientUnits="userSpaceOnUse"
			cx="565.531"
			cy="-165.113"
			r="53.614"
			gradientTransform="scale(1 -1)"
			><stop offset="0" stop-color="#FFF" /><stop
				offset="1"
				stop-color="#FFF"
				stop-opacity=".001"
			/></radialGradient
		><radialGradient
			id="n"
			gradientUnits="userSpaceOnUse"
			cx="333.47"
			cy="-63.138"
			r="53.614"
			gradientTransform="scale(1 -1)"
			><stop offset="0" stop-color="#FFF" /><stop
				offset="1"
				stop-color="#FFF"
				stop-opacity=".001"
			/></radialGradient
		><radialGradient id="i" gradientUnits="userSpaceOnUse" cx="536.581" cy="578.032" r="21.1"
			><stop offset="0" stop-color="#C0E7C7" /><stop
				offset="1"
				stop-color="#229614"
			/></radialGradient
		><radialGradient id="j" gradientUnits="userSpaceOnUse" cx="553.574" cy="328.29" r="165.234"
			><stop offset="0" stop-color="#C0E7C7" /><stop
				offset="1"
				stop-color="#229614"
			/></radialGradient
		><radialGradient
			id="o"
			gradientUnits="userSpaceOnUse"
			cx="488.812"
			cy="-304.626"
			r="53.614"
			gradientTransform="scale(1 -1)"
			><stop offset="0" stop-color="#FFF" /><stop
				offset="1"
				stop-color="#FFF"
				stop-opacity=".001"
			/></radialGradient
		><linearGradient
			id="p"
			gradientUnits="userSpaceOnUse"
			x1="395.946"
			y1="909.607"
			x2="398.861"
			y2="889.309"
			><stop offset="0" stop-color="#FF1F19" /><stop
				offset="1"
				stop-color="#B7282C"
			/></linearGradient
		><linearGradient
			id="q"
			gradientUnits="userSpaceOnUse"
			x1="364.412"
			y1="920.811"
			x2="381.882"
			y2="898.882"
			><stop offset="0" stop-color="#FF1F19" /><stop
				offset="1"
				stop-color="#B7282C"
			/></linearGradient
		><linearGradient
			id="r"
			gradientUnits="userSpaceOnUse"
			x1="400.103"
			y1="884.014"
			x2="414.327"
			y2="886.432"
			><stop offset="0" stop-color="#FF1F19" /><stop
				offset="1"
				stop-color="#B7282C"
			/></linearGradient
		><linearGradient
			id="t"
			gradientUnits="userSpaceOnUse"
			x1="401.529"
			y1="-883.029"
			x2="418.355"
			y2="-884.721"
			gradientTransform="scale(1 -1)"
			><stop offset="0" stop-color="#AF2525" /><stop
				offset="1"
				stop-color="#90282C"
			/></linearGradient
		><linearGradient
			id="u"
			gradientUnits="userSpaceOnUse"
			x1="406.53"
			y1="880.516"
			x2="409.403"
			y2="880.805"
			><stop offset="0" stop-color="#AF2525" /><stop
				offset="1"
				stop-color="#90282C"
			/></linearGradient
		><linearGradient
			id="v"
			gradientUnits="userSpaceOnUse"
			x1="452.2"
			y1="905.456"
			x2="444.604"
			y2="892.921"
			><stop offset="0" stop-color="#FF1F19" /><stop
				offset="1"
				stop-color="#B7282C"
			/></linearGradient
		><linearGradient
			id="w"
			gradientUnits="userSpaceOnUse"
			x1="462.254"
			y1="893.729"
			x2="456.459"
			y2="887.924"
			><stop offset="0" stop-color="#FF1F19" /><stop
				offset="1"
				stop-color="#B7282C"
			/></linearGradient
		><linearGradient
			id="x"
			gradientUnits="userSpaceOnUse"
			x1="446.135"
			y1="882.916"
			x2="429.775"
			y2="883.616"
			><stop offset="0" stop-color="#FF1F19" /><stop
				offset="1"
				stop-color="#B7282C"
			/></linearGradient
		><linearGradient
			id="z"
			gradientUnits="userSpaceOnUse"
			x1="443.248"
			y1="-882.928"
			x2="426.452"
			y2="-884.649"
			gradientTransform="scale(1 -1)"
			><stop offset="0" stop-color="#AF2525" /><stop
				offset="1"
				stop-color="#90282C"
			/></linearGradient
		><linearGradient
			id="A"
			gradientUnits="userSpaceOnUse"
			x1="438.234"
			y1="880.485"
			x2="435.361"
			y2="880.78"
			><stop offset="0" stop-color="#AF2525" /><stop
				offset="1"
				stop-color="#90282C"
			/></linearGradient
		><clipPath id="s"
			><path
				d="M408.16 886.73c2.63.16 5.1-.27 7.57-1.06 3.52-1.3 2.99-1.72 1.65-1.82l.21-2.29c-2.74.34-5.51-.08-8.21-.56 1.75.6 3.93 1.12 4.79 2.94h-.03s-3.11 3.22-12.86 1.6c2.22.78 4.56.96 6.88 1.19z"
			/></clipPath
		><clipPath id="y"
			><path
				d="M436.63 886.65c2.32-.16 4.66-.45 6.88-1.19-3.99.52-9.46 1.2-12.88-1.55 0 0-.01.01-.03 0 .38-1.36 2.78-2.34 4.78-2.94-5.2 1.07-8.18.51-8.18.51l.23 2.22.01.01c-1.35.13-1.9.59 1.62 1.88 2.44.98 4.99.96 7.57 1.06z"
			/></clipPath
		></defs
	><g style="display:inline"
		><path
			d="M69.333 108.453c-3.363 8.127 5.785 15.619 5.825 15.74-1.671-4.902-3.437-8.859-.19-12.284M145.238 107.918c3.363 8.128-5.785 15.62-5.825 15.74 1.671-4.901 3.437-8.858.19-12.284"
			style="fill:#803300;stroke:#000;stroke-width:1;stroke-linecap:butt;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
			transform="translate(-40.229 -7.439)"
		/><path
			d="M107.042 65.431c-1.308-.015-2.56 2.353-4.89 6.987-7.052-7.756-7.413-7.85-9.842 2.65l-.341.145c-8.882-5.325-9.341-5.366-8.667 5.041-.216.16-.431.322-.646.487-9.102-3.605-9.496-3.378-7.192 6.612l-.324.35c-9.277-2.506-9.536-2.13-6.077 7.659-.148.213-.295.43-.443.645-10.456-1.117-10.671-.913-5.355 8.829l-.079.142c-4.284 2.048-5.702 4.544-7.732 7.894-.157 6.645 6.955 12.372 17.18 18.88-3.513-6.512-9.007-9.813-10.06-17.889 1.543-2.905 4.168-3.909 6.422-5.216 24.62 14.368 50.616 14.535 76.551-.55 2.26 1.315 4.9 2.315 6.45 5.232-1.053 8.076-6.548 11.376-10.061 17.888 10.226-6.508 17.338-12.234 17.18-18.879-1.973-3.257-3.376-5.706-7.389-7.721 5.013-9.219 4.536-9.305-5.986-8.178l-.267-.346c3.635-10.261 3.407-10.53-6.392-7.871 2.48-10.706 2.149-10.75-7.779-6.804.748-11.285.48-11.262-8.888-5.643-2.499-10.818-2.793-10.786-9.94-2.924-2.69-4.934-4.104-7.404-5.433-7.42z"
			style="display:inline;fill:#f60;stroke:#000;stroke-width:1;stroke-linecap:butt;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
			transform="translate(-40.229 -7.439)"
		/><g style="display:inline"
			><path
				style="display:inline;fill:#d45500;stroke:#000;stroke-width:1;stroke-linecap:butt;stroke-linejoin:miter;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
				d="M98.809 121.552c-.683-.056 4.469 10.176 8.052.173.241-.674-3.692 2.91-8.052-.173z"
				transform="translate(-35.752 -16.771)"
			/><path
				style="fill:#000;stroke:#000;stroke-width:.264583px;stroke-linecap:butt;stroke-linejoin:miter;stroke-opacity:1"
				d="M93.213 111.563c-5.132 10.992 5.618 14.735 4.412 4.12-.32-3.288-2.328-7.175-4.412-4.12z"
				transform="translate(-35.752 -16.771)"
			/><path
				style="fill:#fff;stroke:#000;stroke-width:.264583px;stroke-linecap:butt;stroke-linejoin:miter;stroke-opacity:1"
				d="M95.394 112.414c-1.183 1.52-.081 3.836-.098 3.986 1.674 1.64 1.612-4.77.098-3.986z"
				transform="translate(-35.752 -16.771)"
			/><path
				d="M109.617 111.563c-5.132 10.992 5.618 14.735 4.412 4.12-.319-3.288-2.328-7.175-4.412-4.12z"
				style="fill:#000;stroke:#000;stroke-width:.264583px;stroke-linecap:butt;stroke-linejoin:miter;stroke-opacity:1"
				transform="translate(-35.752 -16.771)"
			/><path
				d="M111.799 112.414c-1.184 1.52-.082 3.836-.1 3.986 1.675 1.64 1.614-4.77.1-3.986z"
				style="display:inline;fill:#fff;stroke:#000;stroke-width:.264583px;stroke-linecap:butt;stroke-linejoin:miter;stroke-opacity:1"
				transform="translate(-35.752 -16.771)"
			/></g
		><path
			d="M82.13 105.045c-.51.004-.85.054-.795.089 0 0-9.134 11.936.31 18.658a9.747 8.277 37.794 0 0 1.437 9.424 9.747 8.277 37.794 0 0 10.565 3.967l-3.564-8.605 9.197.735a9.747 8.277 37.794 0 0-7.027-8.91 9.747 8.277 37.794 0 0-7.007.008c-1.863-1.364-4.63-4.925-.041-12.475 1.518-2.498-1.544-2.903-3.075-2.891z"
			style="display:inline;fill:#f60;fill-opacity:1;stroke:#000;stroke-width:1;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
			transform="translate(-40.229 -7.439)"
		/></g
	><g style="display:inline"
		><path
			style="display:inline;opacity:1;fill:none;fill-opacity:1;stroke:#000;stroke-width:1;stroke-linecap:butt;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
			d="m142.551 137.926 27.363-15.274 21.397 38.332-27.364 15.274z"
			transform="translate(-45.497 -74.79)"
		/><path
			d="m142.551 137.926 27.363-15.274 21.397 38.332-27.364 15.274-6.27-11.236c3.509-3.113 4.196-7.011 3.176-11.383l-7.391 3.833z"
			style="display:inline;opacity:1;fill:#2a3455;fill-opacity:1;stroke:#000;stroke-width:1;stroke-linecap:butt;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
			transform="translate(-45.497 -74.79)"
		/><g style="display:inline"
			><path
				d="M142.603 99.07c-.258.105-.309.673-.384 1.774-2.022-.94-2.1-.93-1.713 1.33l-.055.057c-2.18-.313-2.274-.284-1.285 1.703l-.086.148c-2.083.043-2.141.12-.867 1.89l-.034.095c-2.027.273-2.046.368-.562 2.003l-.033.163c-2.144.642-2.169.7-.324 2.173a.415.415 0 0 1-.004.035c-.672.754-.744 1.36-.866 2.185.516 1.317 2.383 1.854 4.925 2.289-1.226-.988-2.575-1.183-3.447-2.68.063-.698.496-1.111.83-1.553 6.013.79 11.127-1.318 14.972-6.413.552.072 1.152.05 1.697.495.458 1.671-.348 2.771-.501 4.338 1.47-2.119 2.394-3.828 1.816-5.119-.656-.476-1.133-.84-2.086-.906.224-2.22.123-2.199-1.848-1.11l-.08-.047c-.133-2.312-.2-2.346-1.903-1.017-.395-2.305-.464-2.286-2.086-.694-.783-2.276-.834-2.25-2.209-.375-1.381-1.917-1.436-1.886-2.19.245-.935-.747-1.416-1.115-1.677-1.009z"
				style="fill:#2a3455;fill-opacity:1;stroke:#ccc;stroke-width:.212762;stroke-linecap:butt;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
				transform="matrix(.86025 -.10203 .10203 .86025 -15.4 -2.856)"
			/><path
				d="M140.978 108.893c-.1.044-.162.081-.148.083 0 0-.81 3.094 1.597 3.635a2.074 1.761 15.02 0 0 1.058 1.73 2.074 1.761 15.02 0 0 2.4-.091l-1.408-1.395 1.864-.613a2.074 1.761 15.02 0 0-2.112-1.17 2.074 1.761 15.02 0 0-1.374.58c-.478-.115-1.314-.586-1.035-2.445.092-.615-.542-.442-.842-.314zM151.165 99.349a2.41 2.41 0 0 0-.391 1.594c.081.739.505 1.344 1.105 1.579.103 2.21-1.432 2.114-1.47 2.1-.286-.078-.717 1.138.535.982 1.956-.245 2.056-2.127 1.981-3.063.496-.165.92-.567 1.164-1.103.368-.811.258-1.775-.272-2.386l-1.285 1.51-.105-.254z"
				style="fill:#2a3455;fill-opacity:1;stroke:#ccc;stroke-width:.212762;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
				transform="matrix(.86025 -.10203 .10203 .86025 -15.4 -2.856)"
			/><path
				style="fill:#b1b3b8;fill-opacity:1;stroke:#b1b3b8;stroke-width:.165543;stroke-linecap:butt;stroke-linejoin:miter;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
				d="M157.038 77.94c-.102.05 1.499 1.066 1.159-.66-.023-.116-.278.727-1.16.66z"
				transform="matrix(1 0 0 1 -35.265 -2.637)"
			/><path
				style="fill:#b1b3b8;fill-opacity:1;stroke:#b1b3b8;stroke-width:.0438px;stroke-linecap:butt;stroke-linejoin:miter;stroke-opacity:1"
				d="M155.395 76.994c.203 1.998 2.049 1.618.976.211-.324-.44-.94-.822-.976-.21z"
				transform="matrix(1 0 0 1 -35.265 -2.637)"
			/><path
				style="fill:#fff;stroke:#fff;stroke-width:.0438px;stroke-linecap:butt;stroke-linejoin:miter;stroke-opacity:1"
				d="M155.777 76.93c-.04.317.314.552.324.575.377.091-.175-.815-.324-.575z"
				transform="matrix(1 0 0 1 -35.265 -2.637)"
			/><path
				d="M157.726 75.602c.204 1.998 2.05 1.618.977.211-.324-.44-.94-.822-.977-.21z"
				style="fill:#b1b3b8;fill-opacity:1;stroke:#b1b3b8;stroke-width:.0438px;stroke-linecap:butt;stroke-linejoin:miter;stroke-opacity:1"
				transform="matrix(1 0 0 1 -35.265 -2.637)"
			/><path
				d="M158.109 75.538c-.04.316.314.552.324.575.377.09-.176-.815-.324-.575z"
				style="fill:#fff;stroke:#fff;stroke-width:.0438px;stroke-linecap:butt;stroke-linejoin:miter;stroke-opacity:1"
				transform="matrix(1 0 0 1 -35.265 -2.637)"
			/></g
		><text
			xml:space="preserve"
			style="font-style:normal;font-weight:400;font-size:1.71585px;line-height:1.25;font-family:sans-serif;fill:#000;fill-opacity:1;stroke:none;stroke-width:.0428963"
			x="94.641"
			y="157.045"
			transform="rotate(-28.988 -22.733 66.89)"
			><tspan x="94.641" y="157.045" style="fill:#ccc;stroke-width:.0428963">PASSPORT</tspan></text
		><text
			xml:space="preserve"
			style="font-style:normal;font-weight:400;font-size:2.03519px;line-height:1.25;font-family:sans-serif;fill:#000;fill-opacity:1;stroke:none;stroke-width:.0508798"
			x="92.064"
			y="154.547"
			transform="rotate(-29.037 -22.724 66.77)"
			><tspan
				x="92.064"
				y="154.547"
				style="font-style:normal;font-variant:normal;font-weight:400;font-stretch:normal;font-family:'.PingFang SC';-inkscape-font-specification:'.PingFang SC';fill:#ccc;stroke-width:.0508798"
				>AUTHSTRALIA</tspan
			></text
		><text
			xml:space="preserve"
			style="font-style:normal;font-weight:400;font-size:2.70471px;line-height:1.25;font-family:sans-serif;fill:#ccc;fill-opacity:1;stroke:none;stroke-width:.0676183"
			x="93.163"
			y="128.624"
			transform="rotate(-29.176 -22.699 66.431)"
			><tspan x="93.163" y="128.624" style="fill:#ccc;stroke-width:.0676183">KANIDM</tspan></text
		></g
	><g style="display:inline"
		><path
			d="M152.41 96.95c3.51-3.114 4.197-7.011 3.177-11.383l-7.392 3.833z"
			style="display:inline;fill:#2a3455;fill-opacity:1;stroke:#000;stroke-width:1;stroke-linecap:butt;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
			transform="translate(-40.229 -7.439)"
		/><path
			d="M143.643 82.216c-2.182 1.424-3.845 3.666-4.599 6.196-.991 3.351-.256 6.742 1.919 8.852-3.58 9.77-10.054 6.556-10.191 6.427-1.1-.856-5.178 3.63.533 5.23 8.92 2.497 12.775-5.475 14.156-9.67 2.451.187 5.02-.782 7.052-2.661 3.069-2.846 4.345-7.222 3.158-10.835l-8.312 4.204.005-1.292z"
			style="display:inline;fill:#f60;fill-opacity:1;stroke:#000;stroke-width:1;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
			transform="translate(-40.229 -7.439)"
		/><path
			style="display:inline;opacity:1;fill:#f60;fill-opacity:1;stroke:#000;stroke-width:1;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
			d="M143.643 82.216c-2.182 1.424-3.845 3.666-4.599 6.196-.991 3.351-.256 6.742 1.919 8.852-3.58 9.77-10.054 6.556-10.191 6.427-1.1-.856-5.178 3.63.533 5.23 8.92 2.497 12.775-5.475 14.156-9.67 2.451.187 5.02-.782 7.052-2.661 3.069-2.846 4.345-7.222 3.158-10.835l-8.312 4.204.005-1.292z"
			transform="translate(-40.229 -7.439)"
		/><path
			d="M143.643 82.216c-2.182 1.424-3.845 3.666-4.599 6.196-.991 3.351-.256 6.742 1.919 8.852-3.58 9.77-10.054 6.556-10.191 6.427-1.1-.856-5.178 3.63.533 5.23 8.92 2.497 12.775-5.475 14.156-9.67 2.451.187 5.02-.782 7.052-2.661 3.069-2.846 4.345-7.222 3.158-10.835l-8.312 4.204.005-1.292z"
			style="display:inline;opacity:1;fill:#f60;fill-opacity:1;stroke:#000;stroke-width:1;stroke-linejoin:round;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1"
			transform="translate(-40.229 -7.439)"
		/></g
	></svg
>
