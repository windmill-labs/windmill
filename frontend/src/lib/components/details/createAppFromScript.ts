import { ccomponents } from '../apps/editor/component'

export function createAppFromScript(path: string, schema: Record<string, any> | undefined) {
	return {
		grid: [
			{
				'3': {
					fixed: false,
					x: 0,
					y: 2,
					w: 2,
					h: 8,
					fullHeight: false
				},
				'12': {
					fixed: false,
					x: 0,
					y: 2,
					w: 12,
					h: 21,
					fullHeight: false
				},
				data: {
					type: 'verticalsplitpanescomponent',
					configuration: {},
					panes: [50, 50],
					customCss: ccomponents['verticalsplitpanescomponent'].customCss,
					numberOfSubgrids: 2,
					id: 'a'
				},
				id: 'a'
			},
			{
				'3': {
					fixed: false,
					x: 0,
					y: 8,
					fullHeight: false,
					w: 6,
					h: 2
				},
				'12': {
					fixed: false,
					x: 0,
					y: 0,
					fullHeight: false,
					w: 12,
					h: 2
				},
				data: {
					type: 'containercomponent',
					configuration: {},
					customCss: {
						container: {
							class: '!p-0',
							style: ''
						}
					},
					actions: undefined,
					numberOfSubgrids: 1,
					id: 'topbar'
				},
				id: 'topbar'
			}
		],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: [],
		css: {},
		norefreshbar: false,
		hideLegacyTopBar: true,
		subgrids: {
			'a-0': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 1,
						w: 3,
						h: 8,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 19,
						fullHeight: false
					},
					data: {
						type: 'schemaformcomponent',
						configuration: {
							displayType: {
								type: 'static',
								value: false
							},
							largeGap: {
								type: 'static',
								value: false
							}
						},
						componentInput: {
							type: 'static',
							fieldType: 'schema',
							value: schema
						},
						customCss: ccomponents['schemaformcomponent'].customCss,
						id: 'c'
					},
					id: 'c'
				},
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						w: 1,
						h: 1,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 19,
						w: 12,
						h: 1,
						fullHeight: false
					},
					data: {
						type: 'buttoncomponent',
						configuration: {
							label: {
								type: 'static',
								value: 'Submit'
							},
							color: {
								type: 'static',
								value: 'dark'
							},
							size: {
								type: 'static',
								value: 'xs'
							},
							fillContainer: {
								type: 'static',
								value: false
							},
							disabled: {
								type: 'evalv2',
								expr: '!c.valid',
								connections: [
									{
										componentId: 'c',
										id: 'valid'
									}
								]
							},
							beforeIcon: {
								type: 'static',
								value: ''
							},
							afterIcon: {
								type: 'static',
								value: ''
							},
							triggerOnAppLoad: {
								type: 'static',
								value: false
							},
							onSuccess: {
								type: 'oneOf',
								selected: 'none',
								configuration: {
									none: {},
									gotoUrl: {
										url: {
											type: 'static',
											value: ''
										},
										newTab: {
											type: 'static',
											value: true
										}
									},
									setTab: {
										setTab: {
											type: 'static',
											value: []
										}
									},
									sendToast: {
										message: {
											type: 'static',
											value: ''
										}
									},
									openModal: {
										modalId: {
											type: 'static',
											value: ''
										}
									},
									closeModal: {
										modalId: {
											type: 'static',
											value: ''
										}
									},
									open: {
										id: {
											type: 'static',
											value: ''
										}
									},
									close: {
										id: {
											type: 'static',
											value: ''
										}
									}
								}
							},
							onError: {
								type: 'oneOf',
								selected: 'errorOverlay',
								configuration: {
									errorOverlay: {},
									gotoUrl: {
										url: {
											type: 'static',
											value: ''
										},
										newTab: {
											type: 'static',
											value: true
										}
									},
									setTab: {
										setTab: {
											type: 'static',
											value: []
										}
									},
									sendErrorToast: {
										message: {
											type: 'static',
											value: ''
										},
										appendError: {
											type: 'static',
											value: true
										}
									},
									openModal: {
										modalId: {
											type: 'static',
											value: ''
										}
									},
									closeModal: {
										modalId: {
											type: 'static',
											value: ''
										}
									},
									open: {
										id: {
											type: 'static',
											value: ''
										}
									},
									close: {
										id: {
											type: 'static',
											value: ''
										}
									}
								}
							}
						},
						componentInput: {
							type: 'runnable',
							fieldType: 'any',
							fields: convertSchemaToFields(schema),
							runnable: {
								type: 'path',
								path: path,
								runType: 'script',
								schema: schema,
								name: path
							},
							autoRefresh: true,
							recomputeOnInputChanged: true
						},
						customCss: ccomponents['buttoncomponent'].customCss,
						recomputeIds: [],
						horizontalAlignment: 'right',
						verticalAlignment: 'center',
						id: 'd'
					},
					id: 'd'
				}
			],
			'a-1': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						w: 2,
						h: 8,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 20,
						fullHeight: false
					},
					data: {
						type: 'tabscomponent',
						configuration: {
							tabsKind: {
								type: 'static',
								value: 'tabs'
							}
						},
						tabs: ['Result', 'Logs'],
						customCss: ccomponents['tabscomponent'].customCss,
						numberOfSubgrids: 2,
						id: 'b',
						disabledTabs: [
							{
								type: 'static',
								value: false,
								fieldType: 'boolean'
							},
							{
								type: 'static',
								value: false,
								fieldType: 'boolean'
							}
						]
					},
					id: 'b'
				}
			],
			'b-0': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						w: 2,
						h: 8,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 18,
						fullHeight: false
					},
					data: {
						type: 'displaycomponent',
						configuration: {},
						componentInput: {
							type: 'connected',
							fieldType: 'object',
							connection: {
								componentId: 'd',
								path: 'result'
							}
						},
						customCss: ccomponents['displaycomponent'].customCss,
						id: 'e'
					},
					id: 'e'
				}
			],
			'b-1': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						w: 2,
						h: 8,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 18,
						fullHeight: false
					},
					data: {
						type: 'jobidlogcomponent',
						configuration: {
							jobId: {
								type: 'connected',
								connection: {
									componentId: 'd',
									path: 'jobId'
								}
							}
						},
						customCss: ccomponents['jobidlogcomponent'].customCss,
						id: 'f'
					},
					id: 'f'
				}
			],
			'topbar-0': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						fullHeight: false,
						w: 6,
						h: 1
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						fullHeight: false,
						w: 6,
						h: 1
					},
					data: {
						type: 'textcomponent',
						configuration: {
							style: {
								type: 'static',
								value: 'Body'
							},
							copyButton: {
								type: 'static',
								value: false
							},
							tooltip: {
								type: 'evalv2',
								value: '',
								fieldType: 'text',
								expr: '`Author: ${ctx.author}`',
								connections: [
									{
										componentId: 'ctx',
										id: 'author'
									}
								]
							},
							disableNoText: {
								type: 'static',
								value: true,
								fieldType: 'boolean'
							}
						},
						componentInput: {
							type: 'templatev2',
							fieldType: 'template',
							eval: '${ctx.summary}',
							connections: [
								{
									id: 'summary',
									componentId: 'ctx'
								}
							]
						},
						customCss: {
							text: {
								class: 'text-xl font-semibold whitespace-nowrap truncate',
								style: ''
							},
							container: {
								class: '',
								style: ''
							}
						},
						actions: undefined,
						horizontalAlignment: 'left',
						verticalAlignment: 'center',
						id: 'title'
					},
					id: 'title'
				},
				{
					'3': {
						fixed: false,
						x: 0,
						y: 1,
						fullHeight: false,
						w: 3,
						h: 1
					},
					'12': {
						fixed: false,
						x: 6,
						y: 0,
						fullHeight: false,
						w: 6,
						h: 1
					},
					data: {
						type: 'recomputeallcomponent',
						configuration: {},
						customCss: {
							container: {
								style: '',
								class: ''
							}
						},
						menuItems: [],
						horizontalAlignment: 'right',
						verticalAlignment: 'center',
						id: 'recomputeall'
					},
					id: 'recomputeall'
				}
			]
		}
	}
}

type Field = {
	type: 'static' | 'connected'
	value: any
	fieldType?: string
	format?: string
	connection?: {
		componentId: string
		path: string
	}
	allowUserResources?: boolean
}

function convertSchemaToFields(schema: Record<string, any> | undefined): { [key: string]: Field } {
	const fields: { [key: string]: Field } = {}

	if (!schema) {
		return fields
	}

	Object.entries(schema.properties).forEach(([fieldName, fieldInfo]: [string, any]) => {
		fields[fieldName] = {
			type: 'connected',
			value: fieldInfo.default,
			fieldType: fieldInfo.type,
			format: fieldInfo.format,
			allowUserResources: true,
			connection: {
				componentId: 'c',
				path: `values.${fieldName}`
			}
		}
	})

	return fields
}

export function createAppFromFlow(path: string, schema: Record<string, any> | undefined) {
	return {
		grid: [
			{
				'3': {
					fixed: false,
					x: 0,
					y: 2,
					w: 2,
					h: 8,
					fullHeight: false
				},
				'12': {
					fixed: false,
					x: 0,
					y: 2,
					w: 12,
					h: 21,
					fullHeight: false
				},
				data: {
					type: 'verticalsplitpanescomponent',
					configuration: {},
					panes: [50, 50],
					customCss: ccomponents['verticalsplitpanescomponent'].customCss,
					numberOfSubgrids: 2,
					id: 'a'
				},
				id: 'a'
			},
			{
				'3': {
					fixed: true,
					x: 0,
					y: 8,
					fullHeight: false,
					w: 6,
					h: 2
				},
				'12': {
					fixed: true,
					x: 0,
					y: 0,
					fullHeight: false,
					w: 12,
					h: 2
				},
				data: {
					type: 'containercomponent',
					configuration: {},
					customCss: {
						container: {
							class: '!p-0',
							style: ''
						}
					},
					numberOfSubgrids: 1,
					id: 'topbar'
				},
				id: 'topbar'
			}
		],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: [],
		css: {},
		norefreshbar: false,
		hideLegacyTopBar: true,
		subgrids: {
			'a-0': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 1,
						w: 3,
						h: 8,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 19,
						fullHeight: false
					},
					data: {
						type: 'schemaformcomponent',
						configuration: {
							displayType: {
								type: 'static',
								value: false
							},
							largeGap: {
								type: 'static',
								value: false
							}
						},
						componentInput: {
							type: 'static',
							fieldType: 'schema',
							value: schema
						},
						customCss: ccomponents['schemaformcomponent'].customCss,
						id: 'c'
					},
					id: 'c'
				},
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						w: 1,
						h: 1,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 19,
						w: 12,
						h: 1,
						fullHeight: false
					},
					data: {
						type: 'buttoncomponent',
						configuration: {
							label: {
								type: 'static',
								value: 'Submit'
							},
							color: {
								type: 'static',
								value: 'dark'
							},
							size: {
								type: 'static',
								value: 'xs'
							},
							fillContainer: {
								type: 'static',
								value: false
							},
							disabled: {
								type: 'evalv2',
								expr: '!c.valid',
								connections: [
									{
										componentId: 'c',
										id: 'valid'
									}
								]
							},
							beforeIcon: {
								type: 'static',
								value: ''
							},
							afterIcon: {
								type: 'static',
								value: ''
							},
							triggerOnAppLoad: {
								type: 'static',
								value: false
							},
							onSuccess: {
								type: 'oneOf',
								selected: 'none',
								configuration: {
									none: {},
									gotoUrl: {
										url: {
											type: 'static',
											value: ''
										},
										newTab: {
											type: 'static',
											value: true
										}
									},
									setTab: {
										setTab: {
											type: 'static',
											value: []
										}
									},
									sendToast: {
										message: {
											type: 'static',
											value: ''
										}
									},
									openModal: {
										modalId: {
											type: 'static',
											value: ''
										}
									},
									closeModal: {
										modalId: {
											type: 'static',
											value: ''
										}
									},
									open: {
										id: {
											type: 'static',
											value: ''
										}
									},
									close: {
										id: {
											type: 'static',
											value: ''
										}
									}
								}
							},
							onError: {
								type: 'oneOf',
								selected: 'errorOverlay',
								configuration: {
									errorOverlay: {},
									gotoUrl: {
										url: {
											type: 'static',
											value: ''
										},
										newTab: {
											type: 'static',
											value: true
										}
									},
									setTab: {
										setTab: {
											type: 'static',
											value: []
										}
									},
									sendErrorToast: {
										message: {
											type: 'static',
											value: ''
										},
										appendError: {
											type: 'static',
											value: true
										}
									},
									openModal: {
										modalId: {
											type: 'static',
											value: ''
										}
									},
									closeModal: {
										modalId: {
											type: 'static',
											value: ''
										}
									},
									open: {
										id: {
											type: 'static',
											value: ''
										}
									},
									close: {
										id: {
											type: 'static',
											value: ''
										}
									}
								}
							}
						},
						componentInput: {
							type: 'runnable',
							fieldType: 'any',
							fields: convertSchemaToFields(schema),
							runnable: {
								type: 'path',
								path: path,
								runType: 'flow',
								schema: schema,
								name: path
							},
							autoRefresh: false,
							recomputeOnInputChanged: false
						},
						customCss: ccomponents['buttoncomponent'].customCss,
						recomputeIds: [],
						horizontalAlignment: 'right',
						verticalAlignment: 'center',
						id: 'd'
					},
					id: 'd'
				}
			],
			'a-1': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						w: 2,
						h: 8,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 20,
						fullHeight: false
					},
					data: {
						type: 'tabscomponent',
						configuration: {
							tabsKind: {
								type: 'static',
								value: 'tabs'
							}
						},
						tabs: ['Result', 'Logs'],
						customCss: ccomponents['tabscomponent'].customCss,
						numberOfSubgrids: 2,
						id: 'b',
						disabledTabs: [
							{
								type: 'static',
								value: false,
								fieldType: 'boolean'
							},
							{
								type: 'static',
								value: false,
								fieldType: 'boolean'
							}
						]
					},
					id: 'b'
				}
			],
			'b-0': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						w: 2,
						h: 8,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 18,
						fullHeight: false
					},
					data: {
						type: 'jobidflowstatuscomponent',
						configuration: {
							jobId: {
								type: 'connected',
								value: '',
								connection: {
									componentId: 'd',
									path: 'jobId'
								}
							}
						},
						customCss: ccomponents['jobidflowstatuscomponent'].customCss,
						id: 'e'
					},
					id: 'e'
				}
			],
			'b-1': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						w: 2,
						h: 8,
						fullHeight: false
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 18,
						fullHeight: false
					},
					data: {
						type: 'jobidlogcomponent',
						configuration: {
							jobId: {
								type: 'connected',
								connection: {
									componentId: 'd',
									path: 'jobId'
								}
							}
						},
						customCss: ccomponents['jobidlogcomponent'].customCss,
						id: 'f'
					},
					id: 'f'
				}
			],
			'topbar-0': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 0,
						fullHeight: false,
						w: 6,
						h: 1
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						fullHeight: false,
						w: 6,
						h: 1
					},
					data: {
						type: 'textcomponent',
						configuration: {
							style: {
								type: 'static',
								value: 'Body'
							},
							copyButton: {
								type: 'static',
								value: false
							},
							tooltip: {
								type: 'evalv2',
								value: '',
								fieldType: 'text',
								expr: '`Author: ${ctx.author}`',
								connections: [
									{
										componentId: 'ctx',
										id: 'author'
									}
								]
							},
							disableNoText: {
								type: 'static',
								value: true,
								fieldType: 'boolean'
							}
						},
						componentInput: {
							type: 'templatev2',
							fieldType: 'template',
							eval: '${ctx.summary}',
							connections: [
								{
									id: 'summary',
									componentId: 'ctx'
								}
							]
						},
						customCss: {
							text: {
								class: 'text-xl font-semibold whitespace-nowrap truncate',
								style: ''
							},
							container: {
								class: '',
								style: ''
							}
						},
						horizontalAlignment: 'left',
						verticalAlignment: 'center',
						id: 'title'
					},
					id: 'title'
				},
				{
					'3': {
						fixed: false,
						x: 0,
						y: 1,
						fullHeight: false,
						w: 3,
						h: 1
					},
					'12': {
						fixed: false,
						x: 6,
						y: 0,
						fullHeight: false,
						w: 6,
						h: 1
					},
					data: {
						type: 'recomputeallcomponent',
						configuration: {},
						customCss: {
							container: {
								style: '',
								class: ''
							}
						},
						menuItems: [],
						horizontalAlignment: 'right',
						verticalAlignment: 'center',
						id: 'recomputeall'
					},
					id: 'recomputeall'
				}
			]
		}
	}
}
