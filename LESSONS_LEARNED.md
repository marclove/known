# Lessons Learned

## Testing File System Operations
- Use `tempfile` crate for isolated temporary directories instead of changing current directory
- Refactor functions to accept directory parameters for better testability
- Avoid `std::env::set_current_dir()` in tests - it affects the entire process and can cause race conditions
- Use absolute paths constructed from temporary directory paths instead of relative paths

## Claude Code Directory Access Restrictions
- Claude Code blocks access to `/tmp` and other system directories for security reasons
- When manually testing CLI functionality, stay within the project's working directory structure
- Unit tests using `tempfile::tempdir()` work correctly - they create temp directories in accessible locations
- Don't confuse manual CLI testing limitations with unit test capabilities - they use different mechanisms
