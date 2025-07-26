pub const STREAM_PREFIX: &str = "WM_STREAM: ";

pub fn extract_stream_from_logs(line: &str) -> Option<String> {
    if line.starts_with(STREAM_PREFIX) {
        // Extract the content after "[wm_stream]:" prefix
        let stream_content = line.strip_prefix(STREAM_PREFIX).unwrap_or("").trim();
        if !stream_content.is_empty() {
            return Some(stream_content.to_string().replace("\\n", "\n"))
        }
    }
    None
}