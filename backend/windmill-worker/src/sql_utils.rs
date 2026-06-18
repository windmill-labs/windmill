// input should contain a single statement. remove all comments before and after it
pub fn remove_comments(stmt: &str) -> &str {
    let mut in_stmt = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut in_string = false;
    let mut string_delimiter = '\0';
    let mut start_byte = None;
    let mut end_byte = stmt.len();

    let mut prev_char = '\0';
    let mut char_indices = stmt.char_indices().peekable();

    while let Some((byte_pos, c)) = char_indices.next() {
        let next_char = char_indices.peek().map(|(_, ch)| *ch).unwrap_or('\0');

        // Handle string literals (single or double quotes)
        if !in_line_comment && !in_block_comment {
            if (c == '\'' || c == '"') && prev_char != '\\' {
                if in_string && c == string_delimiter {
                    // Exiting string
                    in_string = false;
                    string_delimiter = '\0';
                } else if !in_string {
                    // Entering string
                    in_string = true;
                    string_delimiter = c;
                }
            }
        }

        // Handle comments only when not inside a string
        if !in_string {
            // Check for start of line comment
            if !in_block_comment && c == '-' && next_char == '-' {
                in_line_comment = true;
            }
            // Check for end of line comment
            else if in_line_comment && c == '\n' {
                in_line_comment = false;
            }
            // Check for start of block comment
            else if !in_line_comment && c == '/' && next_char == '*' {
                in_block_comment = true;
            }
            // Check for end of block comment
            else if in_block_comment && c == '*' && next_char == '/' {
                in_block_comment = false;
                // Skip the closing '/' by advancing the iterator
                char_indices.next();
                prev_char = '/';
                continue;
            }
        }

        // Track statement boundaries
        if !in_line_comment && !in_block_comment && !in_string {
            // Mark start of statement
            if !in_stmt && !c.is_whitespace() {
                start_byte = Some(byte_pos);
                in_stmt = true;
            }
            // Mark end of statement at semicolon
            if in_stmt && c == ';' {
                end_byte = byte_pos + c.len_utf8();
                break;
            }
        }

        prev_char = c;
    }

    &stmt[start_byte.unwrap_or(0)..end_byte]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for remove_comments function
    #[test]
    fn test_remove_comments_single_line() {
        let sql = "-- This is a comment\nSELECT * FROM table;";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_multi_line() {
        let sql = "-- This is a comment\nSELECT * FROM table;\n-- Another comment";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_inline_comment() {
        let sql = "   SELECT * FROM table;    -- This is an inline comment  ";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_no_comments() {
        let sql = "SELECT * FROM table;";
        assert_eq!(remove_comments(sql), "SELECT * FROM table;");
    }
    #[test]
    fn test_remove_comments_empty_string() {
        let sql = "";
        assert_eq!(remove_comments(sql), "");
    }
    #[test]
    fn test_remove_comments_with_whitespace() {
        let sql = "   -- Comment\n  -- Comment2\n  -- Comment3\n   SELECT\n\n * FROM\n table\n;\n\n -- end comment   ";
        assert_eq!(remove_comments(sql), "SELECT\n\n * FROM\n table\n;");
    }
    #[test]
    fn test_remove_comments_comment_in_string() {
        let sql = "SELECT '-- not a comment' FROM table;";
        let result = remove_comments(sql);
        assert_eq!(result, "SELECT '-- not a comment' FROM table;");
    }
    #[test]
    fn test_remove_comments_multiple_dashes() {
        let sql = "SELECT 5 - - 3;";
        let result = remove_comments(sql);
        // This correctly handles the subtraction of negative number
        assert_eq!(result, sql);
    }

    #[test]
    fn test_remove_comments_invalid_truncate() {
        let sql = r#"-- Mise à jour de la table café
UPDATE xyz.abcd t
SET
  uio = s.uio
FROM table1 s
WHERE t.attrib = s.attrib;
"#;
        let expected = r#"UPDATE xyz.abcd t
SET
  uio = s.uio
FROM table1 s
WHERE t.attrib = s.attrib;"#;
        let result = remove_comments(sql);
        assert_eq!(result.trim(), expected);
    }
}
