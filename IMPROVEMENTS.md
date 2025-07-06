# Test Coverage Improvement Plan

## Current State Analysis

**Overall Coverage**: 58.95% (359/609 lines)

**Per-Module Coverage**:
- `agents.rs`: 85.1% (40/47) - **GOOD**
- `symlinks.rs`: 90.3% (56/62) - **EXCELLENT**
- `single_instance.rs`: 70.5% (55/78) - **GOOD**
- `config.rs`: 72.9% (62/85) - **GOOD**
- `daemon.rs`: 50.3% (98/195) - **NEEDS IMPROVEMENT**
- `autostart.rs`: 36.4% (16/44) - **ACCEPTABLE** (system integration)
- `main.rs`: 32.7% (32/98) - **ACCEPTABLE** (covered by integration tests)

## Strategic Assessment

The current test coverage is **appropriate for a system tool** with comprehensive integration testing. However, targeted improvements in `daemon.rs` would provide significant value given its complexity and critical role in the application.

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

### Phase 2: Error Path Coverage (Medium Priority)
**Target**: Improve error handling coverage across modules

#### 2.1 Single Instance Lock Edge Cases
- **Lines to cover**: 42-44, 48, 51, 100, 104, 106-110
- **Test**: Race conditions, corrupted PID files, permission scenarios
- **Approach**: Concurrent test execution, file permission manipulation
- **Expected effort**: 2-3 unit tests

#### 2.2 Configuration Error Scenarios
- **Lines to cover**: 42, 60, 73, 101-103, 156, 160-162
- **Test**: Malformed JSON, disk full scenarios, concurrent access
- **Approach**: Create invalid config files, mock filesystem errors
- **Expected effort**: 2-3 unit tests

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

**After Phase 1**: Overall coverage should improve to ~65-68%
**After Phase 2**: Overall coverage should reach ~70-73%
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
