// input should contain a single statement. remove all comments before and after it
pub fn remove_comments(stmt: &str) -> &str {
    let mut in_stmt = false;
    let mut in_comment = false;
    let mut start = None;
    let mut end = stmt.len();

    let mut c = ' ';
    for (next_i, next_char) in stmt.char_indices() {
        if next_i > 0 {
            let i = next_i - 1;
            if !in_comment && in_stmt && c == ';' {
                end = i + 1;
                break;
            } else if in_comment && c == '\n' {
                in_comment = false;
            } else if c == '-' && next_char == '-' {
                in_comment = true;
            } else if !in_comment && !c.is_whitespace() && start == None {
                start = Some(i);
                in_stmt = true;
            }
        }
        c = next_char;
    }

    return &stmt[start.unwrap_or(0)..end];
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
