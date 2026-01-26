import { describe, expect, it } from 'vitest'
import { makeAlterTableQueries, diffTableEditorValues } from './alterTable'
import type { TableEditorValues } from '../tableEditor'

function normalize(sql?: string) {
	return sql?.replace(/\s+/g, ' ').trim()
}

describe('makeAlterTableQueries', () => {
	it('adds a column', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'addColumn',
						column: {
							name: 'age',
							datatype: 'INTEGER',
							nullable: true
						}
					}
				]
			},
			'postgresql'
		)

		expect(queries).toHaveLength(1)
		expect(normalize(queries[0])).toBe(
			normalize(`
				ALTER TABLE users
				ADD COLUMN age INTEGER;
			`)
		)
	})

	it('drops a column', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'dropColumn',
						name: 'age'
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toBe(normalize('ALTER TABLE users DROP COLUMN "age";'))
	})

	it('renames a column', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'alterColumn',
						changes: { name: 'name' },
						original: { datatype: 'VARCHAR', name: 'fullname' }
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toBe(
			normalize('ALTER TABLE users RENAME COLUMN fullname TO name;')
		)
	})

	it('alters column type and nullability', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'alterColumn',
						original: { datatype: 'VARCHAR', name: 'age', nullable: true },
						changes: {
							datatype: 'BIGINT',
							nullable: false
						}
					}
				]
			},
			'postgresql'
		)

		expect(queries).toHaveLength(2)

		expect(normalize(queries[0])).toBe(normalize('ALTER TABLE users ALTER COLUMN age TYPE BIGINT;'))

		expect(normalize(queries[1])).toBe(
			normalize('ALTER TABLE users ALTER COLUMN age SET NOT NULL;')
		)
	})

	it('sets and drops default values', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'alterColumn',
						original: { datatype: 'TIMESTAMPTZ', name: 'created_at', defaultValue: 'x' },
						changes: {
							defaultValue: '{now()}'
						}
					},
					{
						kind: 'alterColumn',
						original: { datatype: 'TIMESTAMPTZ', name: 'created_at', defaultValue: 'x' },
						changes: {
							defaultValue: undefined
						}
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toEqual(
			'ALTER TABLE users ALTER COLUMN created_at SET DEFAULT now();'
		)

		expect(normalize(queries[1])).toEqual('ALTER TABLE users ALTER COLUMN created_at DROP DEFAULT;')
	})

	it('adds a foreign key', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'posts',
				operations: [
					{
						kind: 'addForeignKey',
						foreignKey: {
							targetTable: 'users',
							columns: [
								{
									sourceColumn: 'user_id',
									targetColumn: 'id'
								}
							],
							onDelete: 'CASCADE',
							onUpdate: 'NO ACTION'
						}
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toBe(
			normalize(`
				ALTER TABLE posts
				ADD CONSTRAINT fk_posts_user_id_users_id
				FOREIGN KEY (user_id)
				REFERENCES users (id)
				ON DELETE CASCADE;
			`)
		)
	})

	it('drops a foreign key using constraint in PostgreSQL', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'posts',
				operations: [
					{
						kind: 'dropForeignKey',
						fk_constraint_name: 'fk_posts_user'
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toBe(
			normalize('ALTER TABLE posts DROP CONSTRAINT fk_posts_user;')
		)
	})

	it('drops a foreign key in MySQL', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'posts',
				operations: [
					{
						kind: 'dropForeignKey',
						fk_constraint_name: 'fk_posts_user'
					}
				]
			},
			'mysql'
		)

		expect(normalize(queries[0])).toBe(
			normalize('ALTER TABLE posts DROP FOREIGN KEY `fk_posts_user`;')
		)
	})

	it('uses schema when provided', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'addColumn',
						column: {
							name: 'age',
							datatype: 'INTEGER'
						}
					}
				]
			},
			'postgresql',
			'public'
		)

		expect(queries[0].startsWith('ALTER TABLE public.users')).toBe(true)
	})

	it('renames a table', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'old_users',
				operations: [
					{
						kind: 'renameTable',
						to: 'new_users'
					}
				]
			},
			'postgresql'
		)

		expect(queries).toHaveLength(1)
		expect(normalize(queries[0])).toBe(normalize('ALTER TABLE old_users RENAME TO new_users;'))
	})

	it('renames a table with schema', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'old_users',
				operations: [
					{
						kind: 'renameTable',
						to: 'new_users'
					}
				]
			},
			'postgresql',
			'public'
		)

		expect(queries).toHaveLength(1)
		expect(normalize(queries[0])).toBe(
			normalize('ALTER TABLE public.old_users RENAME TO new_users;')
		)
	})

	it('renames a table in MySQL', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'old_users',
				operations: [
					{
						kind: 'renameTable',
						to: 'new_users'
					}
				]
			},
			'mysql'
		)

		expect(queries).toHaveLength(1)
		expect(normalize(queries[0])).toBe(normalize('ALTER TABLE old_users RENAME TO new_users;'))
	})

	it('handles multiple operations including rename table', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'addColumn',
						column: {
							name: 'email',
							datatype: 'VARCHAR',
							datatype_length: 255,
							nullable: false
						}
					},
					{
						kind: 'alterColumn',
						original: { datatype: 'VARCHAR', name: 'fullname' },
						changes: { name: 'name' }
					},
					{
						kind: 'renameTable',
						to: 'customers'
					}
				]
			},
			'postgresql'
		)

		expect(queries).toHaveLength(3)
		expect(normalize(queries[0])).toBe(
			normalize('ALTER TABLE users ADD COLUMN email VARCHAR(255) NOT NULL;')
		)
		expect(normalize(queries[1])).toBe(
			normalize('ALTER TABLE users RENAME COLUMN fullname TO name;')
		)
		expect(normalize(queries[2])).toBe(normalize('ALTER TABLE users RENAME TO customers;'))
	})

	it('renames table with special characters in name', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'user_accounts',
				operations: [
					{
						kind: 'renameTable',
						to: 'customer_profiles'
					}
				]
			},
			'postgresql'
		)

		expect(queries).toHaveLength(1)
		expect(normalize(queries[0])).toBe(
			normalize('ALTER TABLE user_accounts RENAME TO customer_profiles;')
		)
	})
})

describe('diffTableEditorValues - Edge Cases', () => {
	it('renaming a column that is primary key should only detect one change (rename)', () => {
		const original: TableEditorValues = {
			name: 'users',
			columns: [
				{
					name: 'user_id',
					datatype: 'INTEGER',
					primaryKey: true
				},
				{
					name: 'email',
					datatype: 'VARCHAR'
				}
			],
			foreignKeys: [],
			pk_constraint_name: 'users_pkey'
		}

		const updated: TableEditorValues = {
			name: 'users',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true,
					initialName: 'user_id'
				},
				{
					name: 'email',
					datatype: 'VARCHAR',
					initialName: 'email'
				}
			],
			foreignKeys: [],
			pk_constraint_name: 'users_pkey'
		}

		const diff = diffTableEditorValues(original, updated)

		// Should only detect one alterColumn operation (rename), not a PK change
		expect(diff.operations).toHaveLength(1)
		expect(diff.operations[0]).toEqual({
			kind: 'alterColumn',
			original: {
				name: 'user_id',
				datatype: 'INTEGER',
				primaryKey: true
			},
			changes: {
				name: 'id'
			}
		})
	})

	it('changing primary keys that contain a column that was renamed should work correctly', () => {
		const original: TableEditorValues = {
			name: 'users',
			columns: [
				{
					name: 'user_id',
					datatype: 'INTEGER',
					primaryKey: true
				},
				{
					name: 'email',
					datatype: 'VARCHAR'
				}
			],
			foreignKeys: [],
			pk_constraint_name: 'users_pkey'
		}

		const updated: TableEditorValues = {
			name: 'users',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true,
					initialName: 'user_id'
				},
				{
					name: 'email',
					datatype: 'VARCHAR',
					primaryKey: true,
					initialName: 'email'
				}
			],
			foreignKeys: [],
			pk_constraint_name: 'users_pkey'
		}

		const diff = diffTableEditorValues(original, updated)

		// Should detect: rename + drop old PK + add new composite PK
		expect(diff.operations).toHaveLength(3)

		const dropPkOp = diff.operations.find((op) => op.kind === 'dropPrimaryKey')
		const addPkOp = diff.operations.find((op) => op.kind === 'addPrimaryKey')
		const alterColOp = diff.operations.find((op) => op.kind === 'alterColumn')

		expect(dropPkOp).toBeDefined()
		expect(addPkOp).toEqual({
			kind: 'addPrimaryKey',
			columns: ['id', 'email']
		})
		expect(alterColOp).toEqual({
			kind: 'alterColumn',
			original: {
				name: 'user_id',
				datatype: 'INTEGER',
				primaryKey: true
			},
			changes: {
				name: 'id'
			}
		})
	})

	it('column a gets renamed to b, foreign key a to c becomes b to c - should only detect one change (rename)', () => {
		const original: TableEditorValues = {
			name: 'posts',
			columns: [
				{
					name: 'post_id',
					datatype: 'INTEGER',
					primaryKey: true
				},
				{
					name: 'author_id',
					datatype: 'INTEGER'
				}
			],
			foreignKeys: [
				{
					targetTable: 'users',
					columns: [
						{
							sourceColumn: 'author_id',
							targetColumn: 'id'
						}
					],
					onDelete: 'CASCADE',
					onUpdate: 'NO ACTION',
					fk_constraint_name: 'fk_posts_author'
				}
			]
		}

		const updated: TableEditorValues = {
			name: 'posts',
			columns: [
				{
					name: 'post_id',
					datatype: 'INTEGER',
					primaryKey: true,
					initialName: 'post_id'
				},
				{
					name: 'user_id',
					datatype: 'INTEGER',
					initialName: 'author_id'
				}
			],
			foreignKeys: [
				{
					targetTable: 'users',
					columns: [
						{
							sourceColumn: 'user_id',
							targetColumn: 'id'
						}
					],
					onDelete: 'CASCADE',
					onUpdate: 'NO ACTION'
				}
			]
		}

		const diff = diffTableEditorValues(original, updated)

		// Should only detect column rename, not FK change
		expect(diff.operations).toHaveLength(1)
		expect(diff.operations[0]).toEqual({
			kind: 'alterColumn',
			original: {
				name: 'author_id',
				datatype: 'INTEGER'
			},
			changes: {
				name: 'user_id'
			}
		})
	})

	it('renaming the table should be detected', () => {
		const original: TableEditorValues = {
			name: 'old_users',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true
				}
			],
			foreignKeys: []
		}

		const updated: TableEditorValues = {
			name: 'new_users',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true,
					initialName: 'id'
				}
			],
			foreignKeys: []
		}

		const diff = diffTableEditorValues(original, updated)

		expect(diff.operations).toHaveLength(1)
		expect(diff.operations[0]).toEqual({
			kind: 'renameTable',
			to: 'new_users'
		})
	})

	it('changing nullable should be detected', () => {
		const original: TableEditorValues = {
			name: 'users',
			columns: [
				{
					name: 'email',
					datatype: 'VARCHAR',
					nullable: true
				}
			],
			foreignKeys: []
		}

		const updated: TableEditorValues = {
			name: 'users',
			columns: [
				{
					name: 'email',
					datatype: 'VARCHAR',
					nullable: false,
					initialName: 'email'
				}
			],
			foreignKeys: []
		}

		const diff = diffTableEditorValues(original, updated)

		expect(diff.operations).toHaveLength(1)
		expect(diff.operations[0]).toEqual({
			kind: 'alterColumn',
			original: {
				name: 'email',
				datatype: 'VARCHAR',
				nullable: true
			},
			changes: {
				nullable: false
			}
		})
	})

	it('dropping column with FK and adding new column with same name should detect FK change', () => {
		const original: TableEditorValues = {
			name: 'posts',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true
				},
				{
					name: 'author_id',
					datatype: 'INTEGER'
				}
			],
			foreignKeys: [
				{
					targetTable: 'users',
					columns: [
						{
							sourceColumn: 'author_id',
							targetColumn: 'id'
						}
					],
					onDelete: 'CASCADE',
					onUpdate: 'NO ACTION',
					fk_constraint_name: 'fk_posts_author'
				}
			]
		}

		const updated: TableEditorValues = {
			name: 'posts',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true,
					initialName: 'id'
				},
				{
					name: 'author_id',
					datatype: 'VARCHAR' // Different datatype, so it's a different column
				}
			],
			foreignKeys: []
		}

		const diff = diffTableEditorValues(original, updated)

		// Should detect: drop FK, drop column, add new column
		const operations = diff.operations

		const dropFkOp = operations.find((op) => op.kind === 'dropForeignKey')
		const dropColOp = operations.find((op) => op.kind === 'dropColumn')
		const addColOp = operations.find((op) => op.kind === 'addColumn')

		expect(dropFkOp).toBeDefined()
		expect(dropFkOp).toEqual({
			kind: 'dropForeignKey',
			fk_constraint_name: 'fk_posts_author'
		})

		expect(dropColOp).toBeDefined()
		expect(dropColOp).toEqual({
			kind: 'dropColumn',
			name: 'author_id'
		})

		expect(addColOp).toBeDefined()
		expect(addColOp).toEqual({
			kind: 'addColumn',
			column: {
				name: 'author_id',
				datatype: 'VARCHAR'
			}
		})
	})

	it('composite FK with multiple columns renamed should not detect FK change', () => {
		const original: TableEditorValues = {
			name: 'order_items',
			columns: [
				{
					name: 'order_id',
					datatype: 'INTEGER'
				},
				{
					name: 'product_id',
					datatype: 'INTEGER'
				}
			],
			foreignKeys: [
				{
					targetTable: 'orders',
					columns: [
						{
							sourceColumn: 'order_id',
							targetColumn: 'id'
						},
						{
							sourceColumn: 'product_id',
							targetColumn: 'product_id'
						}
					],
					onDelete: 'CASCADE',
					onUpdate: 'NO ACTION',
					fk_constraint_name: 'fk_order_items'
				}
			]
		}

		const updated: TableEditorValues = {
			name: 'order_items',
			columns: [
				{
					name: 'o_id',
					datatype: 'INTEGER',
					initialName: 'order_id'
				},
				{
					name: 'p_id',
					datatype: 'INTEGER',
					initialName: 'product_id'
				}
			],
			foreignKeys: [
				{
					targetTable: 'orders',
					columns: [
						{
							sourceColumn: 'o_id',
							targetColumn: 'id'
						},
						{
							sourceColumn: 'p_id',
							targetColumn: 'product_id'
						}
					],
					onDelete: 'CASCADE',
					onUpdate: 'NO ACTION'
				}
			]
		}

		const diff = diffTableEditorValues(original, updated)

		// Should only detect two column renames, no FK change
		expect(diff.operations).toHaveLength(2)
		expect(diff.operations.filter((op) => op.kind === 'alterColumn')).toHaveLength(2)
		expect(diff.operations.filter((op) => op.kind === 'dropForeignKey')).toHaveLength(0)
		expect(diff.operations.filter((op) => op.kind === 'addForeignKey')).toHaveLength(0)
	})

	it('changing FK onDelete should detect FK change', () => {
		const original: TableEditorValues = {
			name: 'posts',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true
				},
				{
					name: 'author_id',
					datatype: 'INTEGER'
				}
			],
			foreignKeys: [
				{
					targetTable: 'users',
					columns: [
						{
							sourceColumn: 'author_id',
							targetColumn: 'id'
						}
					],
					onDelete: 'CASCADE',
					onUpdate: 'NO ACTION',
					fk_constraint_name: 'fk_posts_author'
				}
			]
		}

		const updated: TableEditorValues = {
			name: 'posts',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true,
					initialName: 'id'
				},
				{
					name: 'author_id',
					datatype: 'INTEGER',
					initialName: 'author_id'
				}
			],
			foreignKeys: [
				{
					targetTable: 'users',
					columns: [
						{
							sourceColumn: 'author_id',
							targetColumn: 'id'
						}
					],
					onDelete: 'SET NULL',
					onUpdate: 'NO ACTION'
				}
			]
		}

		const diff = diffTableEditorValues(original, updated)

		// Should detect: drop old FK, add new FK with different onDelete
		const operations = diff.operations

		expect(operations.filter((op) => op.kind === 'dropForeignKey')).toHaveLength(1)
		expect(operations.filter((op) => op.kind === 'addForeignKey')).toHaveLength(1)

		const addFkOp = operations.find((op) => op.kind === 'addForeignKey')
		expect(addFkOp).toBeDefined()
		if (addFkOp && addFkOp.kind === 'addForeignKey') {
			expect(addFkOp.foreignKey.onDelete).toBe('SET NULL')
		}
	})

	it('changing FK target table should detect FK change', () => {
		const original: TableEditorValues = {
			name: 'posts',
			columns: [
				{
					name: 'author_id',
					datatype: 'INTEGER'
				}
			],
			foreignKeys: [
				{
					targetTable: 'users',
					columns: [
						{
							sourceColumn: 'author_id',
							targetColumn: 'id'
						}
					],
					onDelete: 'CASCADE',
					onUpdate: 'NO ACTION',
					fk_constraint_name: 'fk_posts_author'
				}
			]
		}

		const updated: TableEditorValues = {
			name: 'posts',
			columns: [
				{
					name: 'author_id',
					datatype: 'INTEGER',
					initialName: 'author_id'
				}
			],
			foreignKeys: [
				{
					targetTable: 'accounts',
					columns: [
						{
							sourceColumn: 'author_id',
							targetColumn: 'id'
						}
					],
					onDelete: 'CASCADE',
					onUpdate: 'NO ACTION'
				}
			]
		}

		const diff = diffTableEditorValues(original, updated)

		// Should detect: drop old FK, add new FK with different target table
		expect(diff.operations.filter((op) => op.kind === 'dropForeignKey')).toHaveLength(1)
		expect(diff.operations.filter((op) => op.kind === 'addForeignKey')).toHaveLength(1)
	})

	it('multiple column changes with rename and nullable should all be detected', () => {
		const original: TableEditorValues = {
			name: 'users',
			columns: [
				{
					name: 'user_name',
					datatype: 'VARCHAR',
					nullable: true
				}
			],
			foreignKeys: []
		}

		const updated: TableEditorValues = {
			name: 'users',
			columns: [
				{
					name: 'username',
					datatype: 'VARCHAR',
					nullable: false,
					initialName: 'user_name'
				}
			],
			foreignKeys: []
		}

		const diff = diffTableEditorValues(original, updated)

		expect(diff.operations).toHaveLength(1)
		expect(diff.operations[0]).toEqual({
			kind: 'alterColumn',
			original: {
				name: 'user_name',
				datatype: 'VARCHAR',
				nullable: true
			},
			changes: {
				name: 'username',
				nullable: false
			}
		})
	})

	it('operation ordering should be correct', () => {
		const original: TableEditorValues = {
			name: 'posts',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true
				},
				{
					name: 'author_id',
					datatype: 'INTEGER'
				},
				{
					name: 'old_col',
					datatype: 'VARCHAR'
				}
			],
			foreignKeys: [
				{
					targetTable: 'users',
					columns: [
						{
							sourceColumn: 'author_id',
							targetColumn: 'id'
						}
					],
					onDelete: 'CASCADE',
					onUpdate: 'NO ACTION',
					fk_constraint_name: 'fk_posts_author'
				}
			],
			pk_constraint_name: 'posts_pkey'
		}

		const updated: TableEditorValues = {
			name: 'articles',
			columns: [
				{
					name: 'id',
					datatype: 'INTEGER',
					primaryKey: true,
					initialName: 'id'
				},
				{
					name: 'author_id',
					datatype: 'INTEGER',
					nullable: false,
					initialName: 'author_id'
				},
				{
					name: 'new_col',
					datatype: 'VARCHAR'
				}
			],
			foreignKeys: []
		}

		const diff = diffTableEditorValues(original, updated)

		// Expected operations in order:
		// 1. dropForeignKey
		// 2. dropColumn (old_col)
		// 3. alterColumn (author_id nullable)
		// 4. addColumn (new_col)
		// 5. renameTable

		const operationKinds = diff.operations.map((op) => op.kind)

		// Check that foreign keys are dropped first
		const dropFkIndex = operationKinds.indexOf('dropForeignKey')
		const addColIndex = operationKinds.indexOf('addColumn')
		const dropColIndex = operationKinds.indexOf('dropColumn')
		const renameTableIndex = operationKinds.indexOf('renameTable')

		expect(dropFkIndex).toBeLessThan(dropColIndex)
		expect(dropColIndex).toBeLessThan(addColIndex)
		expect(addColIndex).toBeLessThan(renameTableIndex)
	})
})
