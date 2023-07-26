export function createAppFromScript(path: string, schema: Schema) {
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
					h: 22,
					id: 'a'
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
						h: 5
					},
					'12': {
						fixed: false,
						x: 0,
						y: 0,
						w: 12,
						h: 21,
						id: 'jn'
					},
					data: {
						type: 'formcomponent',
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
									}
								}
							}
						},
						componentInput: {
							type: 'runnable',
							fieldType: 'any',
							fields: {
								a: {
									type: 'user',
									value: null,
									fieldType: 'number'
								},
								b: {
									type: 'user',
									value: null,
									fieldType: 'select',
									selectOptions: ['my', 'enum']
								},
								d: {
									type: 'user',
									value: 'inferred type string from default arg',
									fieldType: 'string'
								},
								e: {
									type: 'user',
									value: {
										nested: 'object'
									},
									fieldType: 'object'
								}
							},
							autoRefresh: false,
							recomputeOnInputChanged: false,
							runnable: {
								type: 'runnableByPath',
								path,
								runType: 'script',
								schema,
								name: path
							}
						},
						customCss: {},
						recomputeIds: [],
						horizontalAlignment: 'center',
						id: 'jn'
					},
					id: 'jn'
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
						h: 9,
						id: 'b'
					},
					data: {
						type: 'logcomponent',
						configuration: {
							jobId: {
								type: 'connected',
								connection: {
									componentId: 'jn',
									path: 'jobId'
								}
							}
						},
						customCss: {},
						id: 'b'
					},
					id: 'b'
				},
				{
					'3': {
						fixed: false,
						x: 0,
						y: 16,
						w: 2,
						h: 8
					},
					'12': {
						fixed: false,
						x: 0,
						y: 9,
						w: 12,
						h: 12,
						id: 'd'
					},
					data: {
						type: 'displaycomponent',
						configuration: {},
						componentInput: {
							type: 'connected',
							fieldType: 'object',
							connection: {
								componentId: 'jn',
								path: 'result'
							}
						},
						customCss: {},
						id: 'd'
					},
					id: 'd'
				}
			]
		}
	}
}
