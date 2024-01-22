export function createAppFromScript(path: string, schema: Record<string, any> | undefined) {
	return {
		grid: [
			{
				'3': {
					fixed: false,
					x: 0,
					y: 0,
					w: 2,
					h: 8
				},
				'12': {
					fixed: false,
					x: 0,
					y: 0,
					w: 12,
					h: 21
				},
				data: {
					type: 'verticalsplitpanescomponent',
					configuration: {},
					panes: [50, 50],
					customCss: {},
					numberOfSubgrids: 2,
					id: 'a'
				},
				id: 'a'
			}
		],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: [],
		css: {},
		norefreshbar: false,
		subgrids: {
			'a-0': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 1,
						w: 3,
						h: 8
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 19
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
						customCss: {},
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
						h: 1
					},
					'12': {
						fixed: false,
						x: 0,
						y: 19,
						w: 12,
						h: 1
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
								type: 'eval',
								expr: '!c.valid'
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
								type: 'runnableByPath',
								path: path,
								runType: 'script',
								schema: schema,
								name: path
							},
							autoRefresh: true,
							recomputeOnInputChanged: true
						},
						customCss: {},
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
						h: 8
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 20
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
						customCss: {},
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
						h: 8
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 18
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
						customCss: {},
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
						h: 8
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 18
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
						customCss: {},
						id: 'f'
					},
					id: 'f'
				}
			]
		}
	}
}

type Field = {
	type: 'static' | 'connected'
	value: any
	fieldType?: string
	connection?: {
		componentId: string
		path: string
	}
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
					y: 0,
					w: 2,
					h: 8
				},
				'12': {
					fixed: false,
					x: 0,
					y: 0,
					w: 12,
					h: 21
				},
				data: {
					type: 'verticalsplitpanescomponent',
					configuration: {},
					panes: [50, 50],
					customCss: {},
					numberOfSubgrids: 2,
					id: 'a'
				},
				id: 'a'
			}
		],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: [],
		css: {},
		norefreshbar: false,
		subgrids: {
			'a-0': [
				{
					'3': {
						fixed: false,
						x: 0,
						y: 1,
						w: 3,
						h: 8
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 19
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
						customCss: {},
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
						h: 1
					},
					'12': {
						fixed: false,
						x: 0,
						y: 19,
						w: 12,
						h: 1
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
								type: 'eval',
								expr: '!c.valid'
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
								type: 'runnableByPath',
								path: path,
								runType: 'flow',
								schema: schema,
								name: path
							},
							autoRefresh: false,
							recomputeOnInputChanged: false
						},
						customCss: {},
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
						h: 8
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 20
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
						customCss: {},
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
						h: 8
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 18
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
						customCss: {},
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
						h: 8
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 18
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
						customCss: {},
						id: 'f'
					},
					id: 'f'
				}
			]
		}
	}
}
