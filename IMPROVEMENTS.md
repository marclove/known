# Test Coverage Improvement Plan

## Current State Analysis

**Overall Coverage**: 68.33% (438/641 lines) - **Updated after Phase 2 completion**

**Per-Module Coverage** (Updated after Phase 2):
- `agents.rs`: 85.1% (40/47) - **GOOD**
- `symlinks.rs`: 91.9% (57/62) - **EXCELLENT**
- `single_instance.rs`: 66.7% (52/78) - **GOOD** (improved edge case coverage)
- `config.rs`: 76.5% (65/85) - **GOOD** (+2.4% improvement)
- `daemon/mod.rs`: 85.2% (23/27) - **EXCELLENT** (+14.8% improvement)
- `daemon/events.rs`: 80.6% (58/72) - **GOOD**
- `daemon/watchers.rs`: 90.2% (37/41) - **EXCELLENT**
- `daemon/config_handler.rs`: 40.6% (26/64) - **NEEDS IMPROVEMENT**
- `daemon/symlinks.rs`: 100% (23/23) - **EXCELLENT**
- `autostart.rs`: 36.4% (16/44) - **ACCEPTABLE** (system integration)
- `main.rs`: 41.8% (41/98) - **GOOD** (+9.2% improvement, covered by integration tests)

## Strategic Assessment

The current test coverage is **good for a system tool** with comprehensive integration testing. Phase 1 improvements in daemon modules have been successfully completed, with most daemon modules now showing good coverage. The primary remaining target is `daemon/config_handler.rs` which needs improvement.

## Improvement Plan

### (COMPLETE) Phase 1: Core Daemon Logic (High Priority)
**Target**: Improve `daemon.rs` from 50.3% to 70%+

#### (COMPLETE) 1.1 File Event Handling Logic
- **Lines to cover**: 174-176, 178, 180-181, 184-185, 191-192
- **Test**: `handle_file_event` function with various file system events
- **Approach**: Create temporary file structures and simulate rename, create, delete events
- **Expected effort**: 2-3 focused unit tests

#### (COMPLETE) 1.2 Directory Watcher Setup
- **Lines to cover**: 136-138, 170, 172
- **Test**: `setup_directory_watchers` with edge cases
- **Approach**: Test with nonexistent directories, permission denied scenarios
- **Expected effort**: 2 unit tests

#### (COMPLETE) 1.3 Configuration Change Handling
- **Lines to cover**: 201, 206, 209, 211-213
- **Test**: Live configuration file updates during daemon operation
- **Approach**: Modify config file while daemon runs, verify watcher updates
- **Expected effort**: 2-3 integration-style tests

### (COMPLETE) Phase 2: Error Path Coverage (Medium Priority)
**Target**: Improve error handling coverage across modules

#### (COMPLETE) 2.1 Single Instance Lock Edge Cases
- **Lines covered**: single_instance.rs edge cases for corrupted PID files, parsing errors, concurrent access
- **Tests implemented**: `test_corrupted_pid_file_handling`, `test_pid_file_with_extra_content`, `test_pid_file_parsing_edge_cases`
- **Result**: Improved edge case coverage for critical single instance functionality

#### (COMPLETE) 2.2 Configuration Error Scenarios
- **Lines covered**: config.rs error paths for nonexistent directories, serialization edge cases
- **Tests implemented**: `test_remove_directory_nonexistent`, `test_contains_directory_nonexistent`, `test_config_serialization_error_scenarios`
- **Result**: Enhanced configuration error handling coverage (+2.4% in config.rs)

### Phase 3: Integration Test Enhancements (Low Priority)
**Target**: Improve end-to-end workflow coverage

#### 3.1 Daemon Lifecycle Testing
- **Test**: Full daemon start/stop/restart cycles
- **Approach**: Integration tests with real file watching
- **Expected effort**: 2-3 integration tests

#### 3.2 Cross-Platform Compatibility
- **Test**: Symlink creation across different filesystems
- **Approach**: Platform-specific test conditions
- **Expected effort**: 2-3 conditional tests

## Implementation Strategy

### Step 1: Prioritize High-Impact Areas
Focus on `daemon.rs` first as it's the most complex module with the lowest coverage. The file event handling logic is critical for application reliability.

### Step 2: Incremental Approach
Add 2-3 tests at a time, validating coverage improvement after each addition. Target specific line ranges rather than attempting comprehensive coverage.

### Step 3: Test Quality Over Quantity
Each new test should:
- Test a specific failure scenario or edge case
- Be maintainable and not brittle to refactoring
- Focus on observable behavior rather than internal implementation

### Step 4: Continuous Monitoring
Run `cargo tarpaulin` after each test addition to track progress and identify remaining gaps.

## Expected Outcomes

**After Phase 1**: Overall coverage improved to 66.46% ✓ COMPLETE
**After Phase 2**: Overall coverage reached 68.33% ✓ COMPLETE (+1.87% improvement)
**After Phase 3**: Overall coverage should reach ~75-78%

## Success Metrics

- **Quantitative**: Achieve 70%+ overall coverage
- **Qualitative**: Improved confidence in daemon reliability
- **Practical**: Fewer production issues related to file watching and configuration handling

## Timeline

- **Phase 1**: 1-2 weeks (high priority)
- **Phase 2**: 2-3 weeks (medium priority)
- **Phase 3**: 3-4 weeks (low priority, as needed)

## Notes

- Avoid testing `main.rs` CLI dispatch (already covered by integration tests)
- Accept lower coverage in `autostart.rs` (thin wrapper around external crate)
- Focus on error paths and edge cases that provide real value
- Maintain the existing high-quality testing standards outlined in `TEST_COVERAGE.md`
