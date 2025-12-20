// input should contain a single statement. remove all comments before and after it
pub fn remove_comments(stmt: &str) -> &str {
    let mut in_stmt = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut in_string = false;
    let mut string_delimiter = '\0';
    let mut start = None;
    let mut end = stmt.len();

    let chars: Vec<char> = stmt.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i];
        let next_char = if i + 1 < len { chars[i + 1] } else { '\0' };
        let prev_char = if i > 0 { chars[i - 1] } else { '\0' };

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
                // Skip the closing '/' by continuing after incrementing i in the loop
                continue;
            }
        }

        // Track statement boundaries
        if !in_line_comment && !in_block_comment && !in_string {
            // Mark start of statement
            if !in_stmt && !c.is_whitespace() {
                start = Some(i);
                in_stmt = true;
            }
            // Mark end of statement at semicolon
            if in_stmt && c == ';' {
                end = i + 1;
                break;
            }
        }
    }

    &stmt[start.unwrap_or(0)..end]
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
}
