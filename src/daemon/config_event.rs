use notify::Event;
use std::path::Path;

/// Checks if the file event is related to the configuration file
pub fn is_config_file_event(event: &Event, config_file_path: &Path) -> bool {
    event.paths.iter().any(|path| {
        path.file_name() == config_file_path.file_name()
            && path.parent() == config_file_path.parent()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{
        event::{DataChange, ModifyKind},
        Event, EventKind,
    };
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_is_config_file_event() {
        let config_path = Path::new("/home/user/.config/known/config.json");

        // Test event that matches config file
        let matching_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![config_path.to_path_buf()],
            attrs: Default::default(),
        };

        assert!(is_config_file_event(&matching_event, config_path));

        // Test event that doesn't match config file
        let non_matching_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![Path::new("/some/other/file.txt").to_path_buf()],
            attrs: Default::default(),
        };

        assert!(!is_config_file_event(&non_matching_event, config_path));
    }

    #[test]
    fn test_is_config_file_event_matching() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        // Test matching config file event
        let matching_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![config_path.clone()],
            attrs: Default::default(),
        };

        assert!(is_config_file_event(&matching_event, &config_path));

        // Test non-matching event
        let other_file = temp_dir.path().join("other.json");
        fs::write(&other_file, "{}").unwrap();

        let non_matching_event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![other_file],
            attrs: Default::default(),
        };

        assert!(!is_config_file_event(&non_matching_event, &config_path));
    }

    #[test]
    fn test_is_config_file_event_different_directory() {
        let temp_dir1 = tempdir().unwrap();
        let temp_dir2 = tempdir().unwrap();

        let config_path1 = temp_dir1.path().join("config.json");
        let config_path2 = temp_dir2.path().join("config.json");

        fs::write(&config_path1, "{}").unwrap();
        fs::write(&config_path2, "{}").unwrap();

        // Test same filename but different directory
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![config_path2.clone()],
            attrs: Default::default(),
        };

        assert!(!is_config_file_event(&event, &config_path1));
        assert!(is_config_file_event(&event, &config_path2));
    }
}
