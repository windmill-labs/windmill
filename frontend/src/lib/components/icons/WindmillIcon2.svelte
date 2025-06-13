<!-- @migration-task Error while migrating Svelte code: $$props is used together with named props in a way that cannot be automatically migrated. -->
<script lang="ts">
	import { run } from 'svelte/legacy'

	import { customIcon } from './store'

	interface Props {
		white?: boolean
		size?: string
		color?: string | undefined
		spin?: 'slow' | 'medium' | 'fast' | 'veryfast' | undefined
		class?: string
	}

	let {
		white = false,
		size = '24px',
		color = undefined,
		spin = undefined,
		class: classNames = ''
	}: Props = $props()

	function hslToHex(h, s, l) {
		s /= 100
		l /= 100

		let c = (1 - Math.abs(2 * l - 1)) * s
		let x = c * (1 - Math.abs(((h / 60) % 2) - 1))
		let m = l - c / 2
		let r = 0
		let g = 0
		let b = 0

		if (0 <= h && h < 60) {
			r = c
			g = x
			b = 0
		} else if (60 <= h && h < 120) {
			r = x
			g = c
			b = 0
		} else if (120 <= h && h < 180) {
			r = 0
			g = c
			b = x
		} else if (180 <= h && h < 240) {
			r = 0
			g = x
			b = c
		} else if (240 <= h && h < 300) {
			r = x
			g = 0
			b = c
		} else if (300 <= h && h < 360) {
			r = c
			g = 0
			b = x
		}

		let rs = Math.round((r + m) * 255)
			.toString(16)
			.padStart(2, '0')
		let gs = Math.round((g + m) * 255)
			.toString(16)
			.padStart(2, '0')
		let bs = Math.round((b + m) * 255)
			.toString(16)
			.padStart(2, '0')

		return `#${rs}${gs}${bs}`
	}

	function hexToHsl(hex) {
		let r: number = parseInt(hex.slice(1, 3), 16) / 255
		let g: number = parseInt(hex.slice(3, 5), 16) / 255
		let b: number = parseInt(hex.slice(5, 7), 16) / 255

		const max = Math.max(r, g, b),
			min = Math.min(r, g, b)
		let h,
			s,
			l = (max + min) / 2

		if (max === min) {
			h = s = 0 // Achromatic
		} else {
			const d = max - min
			s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
			switch (max) {
				case r:
					h = (g - b) / d + (g < b ? 6 : 0)
					break
				case g:
					h = (b - r) / d + 2
					break
				case b:
					h = (r - g) / d + 4
					break
			}
			h /= 6
		}

		return [h * 360, s * 100, l * 100]
	}

	function reduceSaturation(hex: string, reductionPercent: number) {
		// Convert HEX to HSL

		// Convert the hex to HSL
		let [h, s, l] = hexToHsl(hex)

		// Reduce the saturation by the specified percentage
		l = Math.max(0, l - reductionPercent)

		// Convert back to hex
		return hslToHex(h, s, l)
	}

	let lessSaturatedColor: string | undefined = $state()

	run(() => {
		color ? (lessSaturatedColor = reduceSaturation(color, -16)) : (lessSaturatedColor = undefined)
	})
</script>

{#if customIcon.white || customIcon.normal}
	{#if white}
		<img
			src={customIcon.white}
			alt="Windmill Custom icon"
			width={size}
			height={size}
			class={classNames}
		/>
	{:else}
		<img
			src={customIcon.normal}
			alt="Windmill Custom icon"
			width={size}
			height={size}
			class={classNames}
		/>
	{/if}
{:else}
	<svg
		class={classNames}
		class:animate-[spin_2s_linear_infinite]={spin === 'veryfast'}
		class:animate-[spin_5s_linear_infinite]={spin === 'fast'}
		class:animate-[spin_15s_linear_infinite]={spin === 'medium'}
		class:animate-[spin_50s_linear_infinite]={spin === 'slow'}
		version="1.1"
		id="Calque_1"
		xmlns="http://www.w3.org/2000/svg"
		xmlns:xlink="http://www.w3.org/1999/xlink"
		x="0px"
		y="0px"
		width={size}
		height={size}
		viewBox="0 0 256 256"
		style="enable-background:new 0 0 256 256;"
		xml:space="preserve"
	>
		<g>
			<!-- Use color or fallback to defaults (white or blue) -->
			<polygon
				fill={lessSaturatedColor || (white ? '#cccccc' : '#bcd4fc')}
				points="134.78,14.22 114.31,48.21 101.33,69.75 158.22,69.75 177.97,36.95 191.67,14.22"
			/>
			<polygon
				fill={color || (white ? '#ffffff' : '#3b82f6')}
				points="227.55,69.75 186.61,69.75 101.33,69.75 129.78,119.02 158.16,119.02 228.61,119.02 256,119.02"
			/>
			<polygon
				fill={color || (white ? '#ffffff' : '#3b82f6')}
				points="136.93,132.47 116.46,167.93 73.82,241.78 130.71,241.78 144.9,217.2 180.13,156.18 193.82,132.46"
			/>
			<polygon
				fill={color || (white ? '#ffffff' : '#3b82f6')}
				points="121.7,131.95 101.23,96.49 58.59,22.63 30.15,71.91 44.34,96.49 79.57,157.5 93.26,181.22"
			/>
			<polygon
				fill={lessSaturatedColor || (white ? '#cccccc' : '#bcd4fc')}
				points="64.81,131.95 25.15,131.21 0,130.74 28.44,180.01 66.73,180.72 93.26,181.21"
			/>
			<polygon
				fill={lessSaturatedColor || (white ? '#cccccc' : '#bcd4fc')}
				points="165.38,181.74 184.58,216.46 196.75,238.47 225.19,189.2 206.66,155.69 193.83,132.46"
			/>
		</g>
	</svg>
{/if}
