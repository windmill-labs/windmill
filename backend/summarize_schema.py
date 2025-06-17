# This script is used to summarize the database schema.
# You can use pg_dump to dump the schema to a file.
# pg_dump --file "schema.sql" --host "localhost" --port "5432" --username "postgres" --no-password --format=c --large-objects --schema-only --no-owner --no-privileges --no-tablespaces --no-unlogged-table-data --no-comments --no-publications --no-subscriptions --no-security-labels --no-toast-compression --no-table-access-method --verbose --schema "public" "windmill"
# Then you can run python summarize_schema.py schema.sql to get the summarized schema.

import re
import sys
from collections import defaultdict

def summarize_schema(file_path):
    """
    Parses a PostgreSQL dump file and extracts a summarized schema.
    """
    tables = defaultdict(lambda: {'columns': [], 'pks': set(), 'fks': [], 'indexes': []})
    enums = defaultdict(list)
    
    # Use state variables to parse multi-line definitions
    current_table = None
    current_enum = None

    with open(file_path, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.strip()

            # --- State Resets ---
            if line.startswith(');'):
                current_table = None
                current_enum = None
                continue

            # --- Parse ENUM definitions ---
            match_enum = re.match(r"CREATE TYPE public\.(\w+) AS ENUM \($", line)
            if match_enum:
                current_enum = match_enum.group(1)
                continue
            
            if current_enum:
                # Extract enum values, which are typically like 'value',
                value = line.strip("',")
                if value and not value.startswith('--'):
                    enums[current_enum].append(value)
                continue

            # --- Parse TABLE definitions ---
            match_table = re.match(r"CREATE TABLE public\.(\w+) \($", line)
            if match_table:
                current_table = match_table.group(1)
                continue

            if current_table:
                # Parse columns within a CREATE TABLE block
                # e.g., "column_name type NOT NULL,"
                # e.g., "id bigint NOT NULL,"
                match_column = re.match(r'^"?(\w+)"?\s+([\w\d\.\[\]\(\)]+)', line)
                if match_column:
                    col_name = match_column.group(1)
                    col_type = match_column.group(2)
                    tables[current_table]['columns'].append(f"{col_name} ({col_type})")
                
                # Parse PRIMARY KEY defined inside the table
                match_pk = re.search(r"CONSTRAINT \w+ PRIMARY KEY \((.+)\)", line)
                if match_pk:
                    # Handle multiple PK columns: "col1, col2, col3"
                    pk_cols = [p.strip().strip('"') for p in match_pk.group(1).split(',')]
                    tables[current_table]['pks'].update(pk_cols)
                continue
            
            # --- Parse Foreign Keys (defined outside CREATE TABLE) ---
            match_fk = re.match(r"ALTER TABLE ONLY public\.(\w+)\s+ADD CONSTRAINT \w+ FOREIGN KEY \(([\w,\s\"]+)\) REFERENCES public\.(\w+)\(([\w,\s\"]+)\);", line)
            if match_fk:
                from_table, from_cols, to_table, to_cols = match_fk.groups()
                # Clean up column names
                from_cols_clean = ', '.join([c.strip().strip('"') for c in from_cols.split(',')])
                to_cols_clean = ', '.join([c.strip().strip('"') for c in to_cols.split(',')])
                
                fk_string = f"({from_cols_clean}) -> {to_table}({to_cols_clean})"
                tables[from_table]['fks'].append(fk_string)
            
            # --- Parse Index definitions ---
            match_index = re.match(r"CREATE (UNIQUE )?INDEX (\w+) ON public\.(\w+) USING (\w+) \((.+)\);", line)
            if match_index:
                is_unique = match_index.group(1) is not None
                index_name = match_index.group(2)
                table_name = match_index.group(3)
                index_type = match_index.group(4)
                columns = match_index.group(5)
                
                # Clean up column expressions
                columns_clean = columns.replace('"', '')
                
                unique_str = "UNIQUE " if is_unique else ""
                index_string = f"{unique_str}INDEX {index_name} ({index_type}) ON ({columns_clean})"
                tables[table_name]['indexes'].append(index_string)

    return enums, tables

def format_output(enums, tables):
    """
    Formats the parsed schema data into a clean, readable string.
    """
    output = []
    
    output.append("### Simplified Database Schema ###")
    output.append("\n--- Custom Data Types (ENUMs) ---\n")
    if not enums:
        output.append("No custom ENUM types found.")
    else:
        for name, values in sorted(enums.items()):
            output.append(f"{name}:")
            for v in values:
                output.append(f"  - {v}")
            output.append("")

    output.append("\n--- Tables and Relationships ---\n")
    if not tables:
        output.append("No tables found.")
    else:
        for name, data in sorted(tables.items()):
            output.append(f"TABLE: {name}")
            for col in data['columns']:
                col_name = col.split(' ')[0]
                marker = " (PK)" if col_name in data['pks'] else ""
                output.append(f"  - {col}{marker}")
            
            if data['fks']:
                output.append("  Relationships:")
                for fk in data['fks']:
                    output.append(f"    - {fk}")
            
            if data['indexes']:
                output.append("  Indexes:")
                for idx in data['indexes']:
                    output.append(f"    - {idx}")
            output.append("-" * 20)

    return "\n".join(output)

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage: python {sys.argv[0]} <path_to_dump.sql>")
        sys.exit(1)
        
    input_file = sys.argv[1]
    
    try:
        enums_data, tables_data = summarize_schema(input_file)
        formatted_summary = format_output(enums_data, tables_data)
        print(formatted_summary)
    except FileNotFoundError:
        print(f"Error: The file '{input_file}' was not found.")
        sys.exit(1)
    except Exception as e:
        print(f"An unexpected error occurred: {e}")
        sys.exit(1)