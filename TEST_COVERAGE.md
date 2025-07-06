# Test Coverage Strategy

This document outlines our approach to test coverage for the `known` CLI and daemon. It serves as a living document to guide our thinking and ensure we maintain a robust and effective test suite.

## Guiding Principles

1.  **Meaningful Testing**: We prioritize tests that cover critical logic, common use cases, and essential error-handling paths. Our goal is not simply to achieve a high percentage, but to build a test suite that provides real value and confidence.
2.  **Avoid Brittle Tests**: Tests should be resilient to change. We favor testing public APIs and observable behavior over internal implementation details, which are more likely to be refactored.
3.  **Integration Tests for the CLI**: The behavior of the main binary (`src/main.rs`) is best tested through integration tests that execute the compiled application. This is the most effective way to validate command-line argument parsing, command dispatch, and end-to-end functionality.
4.  **Focus on Error Handling**: A significant portion of our logic involves handling potential failures (e.g., file I/O, network issues). We write tests that specifically trigger these error conditions to ensure the application behaves gracefully and provides clear feedback to the user.

## Current State & Future Guidance

Our test suite provides a strong safety net against regressions in the most critical areas of the application.

### Strengths of the Current Test Suite

*   **High-Value Integration Tests**: We have a solid suite of integration tests for the CLI that validate the primary user workflows (`init`, `add`, `remove`, `symlink`, `start`, `stop`). These tests ensure that the core functionality of the application works as expected from the user's perspective.
*   **Robust Error Handling**: Our unit tests for `config.rs`, `agents.rs`, `symlinks.rs`, and `single_instance.rs` specifically target I/O error conditions, such as permission denied errors and malformed configuration files. This gives us confidence that the application can handle unexpected states gracefully.

### Areas for Future Improvement

While our coverage is strong, there are areas where we can make pragmatic improvements over time.

*   **`src/daemon.rs`**: This is our most complex module. While the core start/stop functionality is tested, the intricate logic of the event loop (e.g., handling live configuration changes, specific file rename events) presents an opportunity for more comprehensive testing. Future work in this area could involve creating a more advanced test harness, potentially with file system mocking, to simulate these scenarios.
*   **`src/autostart.rs`**: This module is a thin wrapper around an external crate (`auto-launch`). Mocking this dependency would be complex and provide limited value, as the internal logic is minimal. We accept a lower coverage percentage here as a pragmatic trade-off.

### General Strategy Moving Forward

As we add new features or refactor existing code, we will continue to follow these principles. Any new CLI commands should be accompanied by integration tests that cover both success and failure cases. New file system or I/O logic should include unit tests that simulate error conditions. By adhering to these guidelines, we can ensure that our test suite remains a valuable asset that helps us build a reliable and stable application.
