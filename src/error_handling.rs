use std::panic::AssertUnwindSafe;

/// Safely execute a parsing operation and handle panics with user-friendly error messages
pub fn safe_parse<T, F>(operation: F) -> Result<T, String>
where
        F: FnOnce() -> T,
{
        match std::panic::catch_unwind(AssertUnwindSafe(operation)) {
                Ok(result) => Ok(result),
                Err(e) => Err(format_parse_error(&e)),
        }
}

/// Format panic error messages into user-friendly error strings
pub fn format_parse_error(e: &Box<dyn std::any::Any + Send>) -> String {
        if let Some(msg) = e.downcast_ref::<String>() {
                msg.clone()
        } else if let Some(msg) = e.downcast_ref::<&str>() {
                msg.to_string()
        } else {
                "Error: parsing failed".to_string()
        }
}
