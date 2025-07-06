# Lessons Learned

## Testing File System Operations
- Use `tempfile` crate for isolated temporary directories instead of changing current directory
- Refactor functions to accept directory parameters for better testability
- Avoid `std::env::set_current_dir()` in tests - it affects the entire process and can cause race conditions
- Use absolute paths constructed from temporary directory paths instead of relative paths
