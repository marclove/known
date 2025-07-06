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

## File System Watching with notify Crate
- On macOS, file system events may return paths with `/private` prefix while the watched directory path doesn't have this prefix
- Always canonicalize paths before comparing them in file watchers to handle symlinks and path resolution differences
- The `notify` crate requires proper path matching - use `path.starts_with(canonical_path)` for reliable event filtering
- File creation events often generate multiple events (Create, Modify metadata, Modify content) - handle all relevant event types
- Use `RecursiveMode::NonRecursive` when only watching direct files in a directory, not subdirectories
