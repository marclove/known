use std::fs;
use std::io;

pub fn create_agents_file() -> io::Result<()> {
    let current_dir = std::env::current_dir()?;
    
    for entry in fs::read_dir(&current_dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy().to_lowercase();
        
        if file_name_str == "agents.md" {
            return Ok(());
        }
    }
    
    let agents_path = current_dir.join("AGENTS.md");
    fs::write(agents_path, "")?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_agents_file() {
        use std::fs;
        
        let test_dir = std::env::temp_dir().join("test_agents");
        fs::create_dir_all(&test_dir).unwrap();
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&test_dir).unwrap();
        
        let result = create_agents_file();
        assert!(result.is_ok());
        
        let agents_path = test_dir.join("AGENTS.md");
        assert!(agents_path.exists());
        
        let content = fs::read_to_string(&agents_path).unwrap();
        assert_eq!(content, "");
        
        let result_second = create_agents_file();
        assert!(result_second.is_ok());
        
        std::env::set_current_dir(original_dir).unwrap();
        fs::remove_dir_all(&test_dir).unwrap();
    }

    #[test]
    fn test_cli_init_command() {
        use std::process::Command;
        use std::fs;
        
        let test_dir = std::env::temp_dir().join("test_cli_init");
        fs::create_dir_all(&test_dir).unwrap();
        
        let project_dir = std::env::current_dir().unwrap();
        
        let output = Command::new("cargo")
            .args(&["run", "--", "init"])
            .current_dir(&project_dir)
            .env("PWD", &test_dir)
            .output()
            .expect("Failed to execute command");
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            panic!("Command failed. Status: {}, Stdout: {}, Stderr: {}", 
                   output.status, stdout, stderr);
        }
        
        let agents_path = project_dir.join("AGENTS.md");
        assert!(agents_path.exists());
        
        let content = fs::read_to_string(&agents_path).unwrap();
        assert_eq!(content, "");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Successfully initialized project with AGENTS.md"));
        
        fs::remove_file(&agents_path).unwrap_or(());
        fs::remove_dir_all(&test_dir).unwrap();
    }
}
