const plugin = require('tailwindcss/plugin')

/** @type {import('tailwindcss').Config} */
const config = {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	safelist: ['hljs', 'splitpanes__pane', 'splitpanes__splitter'],
	theme: {
		colors: {
			current: 'currentcolor',
			transparent: 'transparent',
			white: '#ffffff',
			black: '#000000',
			gray: {
				50: '#f9fafb',
				100: '#f3f4f6',
				200: '#e5e7eb',
				300: '#d1d5db',
				400: '#9ca3af',
				500: '#6b7280',
				600: '#4b5563',
				700: '#374151',
				800: '#1f2937',
				900: '#111827'
			},
			red: {
				50: '#fef2f2',
				100: '#fee2e2',
				200: '#fecaca',
				300: '#fca5a5',
				400: '#f87171',
				500: '#ef4444',
				600: '#dc2626',
				700: '#b91c1c',
				800: '#991b1b',
				900: '#7f1d1d'
			},
			orange: {
				100: '#ffedd5',
				200: '#ffedd5',
				400: '#fb923c',
				500: '#f97316',
				600: '#ea580c',
				700: '#c2410c',
				800: '#c2410c'
			},
			yellow: {
				50: '#fefce8',
				100: '#fef9c3',
				200: '#fef08a',
				300: '#fde047',
				400: '#facc15',
				500: '#eab308',
				600: '#ca8a04',
				700: '#a16207',
				800: '#854d0e',
				900: '#713f12'
			},
			green: {
				50: '#f0fdf4',
				100: '#dcfce7',
				200: '#bbf7d0',
				300: '#86efac',
				400: '#4ade80',
				500: '#22c55e',
				600: '#16a34a',
				700: '#15803d',
				800: '#166534',
				900: '#14532d'
			},
			blue: {
				50: '#eff6ff',
				100: '#dbeafe',
				200: '#bfdbfe',
				300: '#93c5fd',
				400: '#60a5fa',
				500: '#3b82f6',
				600: '#2563eb',
				700: '#1d4ed8',
				800: '#1e40af',
				900: '#1e3a8a'
			},
			indigo: {
				100: '#e0e7ff',
				200: '#c7d2fe',
				300: '#a5b4fc',
				500: '#6366f1',
				800: '#3730a3',
				900: '#312e81'
			}
		},
		fontFamily: {
			// add double quotes if there is space in font name
			main: ['Inter', 'sans-serif'],
			mono: [
				'ui-monospace',
				'SFMono-Regular',
				'Menlo',
				'Monaco',
				'Consolas',
				'"Liberation Mono"',
				'"Courier New"',
				'monospace'
			]
		},
		extend: {
			maxHeight: {
				'1/2': '50vh',
				'2/3': '66vh',
				'3/4': '75vh'
			},
			minWidth: {
				'1/4': '25%',
				'1/3': '33%',
				'1/2': '50%',
				'2/3': '66%',
				'3/4': '75%'
			},
			minHeight: {
				'1/2': '50vh'
			},
			height: {
				'2/3': '66vh'
			},
			transitionProperty: {
				height: 'height'
			},
			fontSize: {
				'2xs': '0.7rem'
			},
			screens: {
				fhd: '1900px',
				qhd: '2500px',
				'4k': '3800px'
			}
		}
	},

	plugins: [
		require('@tailwindcss/forms'),
		require('@tailwindcss/typography'),
		plugin(({ addBase, addComponents, addUtilities, theme }) => {
			addBase({
				html: {
					fontFamily: theme('fontFamily.main'),
					fontSize: theme('fontSize.base'),
					fontWeight: theme('fontWeight.normal'),
					color: theme('colors.gray.900'),
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: theme('fontSize.lg')
					}
				},
				h1: {
					fontSize: '24px',
					fontWeight: theme('fontWeight.extrabold'),
					lineHeight: '1.05',
					color: theme('colors.gray.800'),
					[`@media (min-width: ${theme('screens.lg')})`]: {
						fontSize: '26px'
					},
					[`@media (min-width: ${theme('screens.fhd')})`]: {
						fontSize: '29px'
					},
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '34px'
					}
				},
				h2: {
					fontSize: '20px',
					fontWeight: theme('fontWeight.extrabold'),
					lineHeight: '1.1',
					color: theme('colors.gray.700'),
					[`@media (min-width: ${theme('screens.fhd')})`]: {
						fontSize: '22px'
					},
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '25px'
					}
				},
				mark: {
					backgroundColor: theme('colors.yellow.200'),
					borderRadius: theme('borderRadius.sm')
				},
				h3: {
					fontSize: '18px',
					fontWeight: theme('fontWeight.bold'),
					lineHeight: '1.2',
					color: theme('colors.gray.600'),
					[`@media (min-width: ${theme('screens.fhd')})`]: {
						fontSize: '20px'
					},
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '22px'
					}
				},
				h4: {
					fontSize: '18px',
					fontWeight: theme('fontWeight.semibold'),
					lineHeight: '1.3',
					color: theme('colors.gray.600'),
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '20px'
					}
				},
				h5: {
					fontSize: '16px',
					fontWeight: theme('fontWeight.semibold'),
					lineHeight: '1.5',
					color: theme('colors.gray.600'),
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '18px'
					}
				},
				h6: {
					fontSize: '16px',
					fontWeight: theme('fontWeight.medium'),
					lineHeight: '1.5',
					color: theme('colors.gray.600'),
					[`@media (min-width: ${theme('screens.qhd')})`]: {
						fontSize: '18px'
					}
				},
				button: {
					fontWeight: theme('fontWeight.semibold')
				},
				a: {
					color: theme('colors.blue.500')
				},
				'input,input[type="text"],input[type="email"],input[type="url"],input[type="password"],input[type="number"],input[type="date"],input[type="datetime-local"],input[type="month"],input[type="search"],input[type="tel"],input[type="time"],input[type="week"],textarea,textarea[type="text"],select':
					{
						display: 'block',
						fontSize: theme('fontSize.sm'),
						width: '100%',
						padding: `${theme('spacing.1')} ${theme('spacing.2')}`,
						border: `1px solid ${theme('colors.gray.300')}`,
						borderRadius: theme('borderRadius.md'),
						boxShadow: theme('boxShadow.sm'),
						'&:focus': {
							'--tw-ring-color': theme('colors.indigo.100'),
							'--tw-ring-offset-shadow':
								'var(--tw-ring-inset) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color)',
							'--tw-ring-shadow':
								'var(--tw-ring-inset) 0 0 0 calc(3px + var(--tw-ring-offset-width)) var(--tw-ring-color)',
							boxShadow:
								'var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow, 0 0 #0000)'
						},
						'&:disabled,[disabled]': {
							backgroundColor: theme('colors.gray.100') + ' !important'
						}
					},
				button: {
					fontWeight: theme('fontWeight.semibold')
				},
				a: {
					color: theme('colors.blue.500')
				},
				'input,input[type="text"],input[type="email"],input[type="url"],input[type="password"],input[type="number"],input[type="date"],input[type="datetime-local"],input[type="month"],input[type="search"],input[type="tel"],input[type="time"],input[type="week"],textarea,select':
					{
						display: 'block',
						fontSize: theme('fontSize.sm'),
						width: '100%',
						padding: `${theme('spacing.1')} ${theme('spacing.2')}`,
						border: `1px solid ${theme('colors.gray.300')}`,
						borderRadius: theme('borderRadius.md'),
						boxShadow: theme('boxShadow.sm'),
						'&:focus': {
							'--tw-ring-color': theme('colors.indigo.100'),
							'--tw-ring-offset-shadow':
								'var(--tw-ring-inset) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color)',
							'--tw-ring-shadow':
								'var(--tw-ring-inset) 0 0 0 calc(3px + var(--tw-ring-offset-width)) var(--tw-ring-color)',
							boxShadow:
								'var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow, 0 0 #0000)'
						},
						'&:disabled,[disabled]': {
							backgroundColor: theme('colors.gray.100') + ' !important'
						}
					},
				'button:disabled,button[disabled=true],a:disabled,a[disabled=true]': {
					pointerEvents: 'none',
					cursor: 'default',
					filter: 'grayscale(1)'
				},
				'pre code.hljs': {
					padding: '0px !important',
					fontFamily: theme('fontFamily.mono'),
					fontSize: theme('fontSize.sm') + ' !important',
					lineHeight: theme('lineHeight.4') + ' !important',
					whiteSpace: 'pre-wrap'
				}
			})
			addComponents({
				'.table-custom': {
					'& th': {
						paddingTop: theme('spacing.3'),
						paddingRight: theme('spacing.1'),
						paddingLeft: theme('spacing.1'),
						paddingBottom: theme('spacing.3'),
						fontSize: theme('fontSize.sm'),
						textAlign: 'left',
						fontWeight: theme('fontWeight.semibold'),
						color: theme('colors.gray.900'),
						textTransform: 'capitalize'
					},
					'& td': {
						paddingRight: theme('spacing.2'),
						paddingTop: theme('spacing.2'),
						paddingBottom: theme('spacing.2'),
						fontSize: theme('fontSize.sm'),
						color: theme('colors.gray.700')
					},
					'& tbody > :not([hidden]) ~ :not([hidden])': {
						borderTop: `1px solid ${theme('colors.gray.200')}`
					}
				},
				'.commit-hash': {
					fontSize: theme('fontSize.2xs'),
					color: theme('colors.gray.500'),
					backgroundColor: theme('colors.gray.200'),
					fontFamily: theme('fontFamily.mono')
				},
				'.input-error': {
					borderColor: `${theme('colors.red.500')} !important`
				},
				'.box': {
					borderWidth: '1px',
					borderRadius: theme('borderRadius.sm'),
					boxShadow: theme('boxShadow.sm'),
					padding: theme('spacing.4')
				},
				'.animate-skeleton': {
					animation: theme('animation.pulse'),
					backgroundColor: theme('colors.blue.100'),
					borderRadius: theme('borderRadius.DEFAULT')
				},
				'.text-black-gradient': {
					color: 'transparent',
					backgroundClip: 'text',
					backgroundImage: `linear-gradient(to right, ${theme('colors.black')}, ${theme(
						'colors.gray.600'
					)})`
				},
				'.splitpanes__pane': {
					backgroundColor: theme('colors.white') + ' !important',
					overflow: 'auto !important'
				},
				'.splitpanes__splitter': {
					backgroundColor: theme('colors.gray.300') + ' !important',
					margin: '0 !important',
					border: 'none !important',
					'&::after': {
						backgroundColor: '#3f83f850 !important',
						margin: '0 !important',
						transform: 'none !important',
						zIndex: '1001 !important',
						transition: 'opacity 200ms !important',
						opacity: '0',
						'--splitter-hover-size': '5px',
						'--splitter-hover-adjustment': '-2px'
					},
					'&:hover::after': {
						opacity: '1'
					}
				},
				'.splitpanes--vertical>.splitpanes__splitter': {
					width: '1px !important',
					'&::before': {
						left: '1px !important',
						width: '0px !important',
						marginLeft: '0 !important'
					},
					'&::after': {
						top: '0 !important',
						height: '100% !important',
						left: 'var(--splitter-hover-adjustment) !important',
						width: 'var(--splitter-hover-size) !important'
					}
				},
				'.splitpanes--horizontal>.splitpanes__splitter': {
					height: '1px !important',
					'&::before': {
						top: '1px !important',
						height: '0px !important',
						marginTop: '0 !important'
					},
					'&::after': {
						top: 'var(--splitter-hover-adjustment) !important',
						height: 'var(--splitter-hover-size) !important',
						left: '0 !important',
						width: '100% !important'
					}
				}
			})
			addUtilities({
				'.separator': {
					backgroundColor: '#ddd !important'
				},
				'.center-center': {
					display: 'flex',
					justifyContent: 'center',
					alignItems: 'center'
				},
				'.ellipsize': {
					overflow: 'hidden',
					whiteSpace: 'nowrap',
					textOverflow: 'ellipsis'
				},
				/** Set the '-webkit-line-clamp' property to the desired number of lines.
				 *
				 * Eg.: `class="ellipsize-multi-line [-webkit-line-clamp:3]"`
				 */
				'.ellipsize-multi-line': {
					display: '-webkit-box',
					'-webkit-box-orient': 'vertical',
					overflow: 'hidden'
				},
				'.disabled': {
					pointerEvents: 'none',
					cursor: 'default',
					filter: 'grayscale(1)'
				},
				'.scrollbar-hidden': {
					'-ms-overflow-style': 'none',
					'scrollbar-width': 'none',
					'&::-webkit-scrollbar': {
						display: 'none',
						width: '0px'
					}
				}
			})
		})
	],
	darkMode: 'class'
}

module.exports = config
