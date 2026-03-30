use std::process::{Child, Command};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Failed to start application '{path}': {source}")]
    StartError {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

pub struct ProcessManager {
    pub app1: Child,
    pub app2: Child,
}

impl ProcessManager {
    fn resolve_path(path: &str) -> String {
        if path.to_lowercase().ends_with(".lnk") {
            let script = format!("(New-Object -COM WScript.Shell).CreateShortcut('{}').TargetPath", path);
            if let Ok(output) = Command::new("powershell")
                .args(&["-NoProfile", "-Command", &script])
                .output()
            {
                let target = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !target.is_empty() {
                    return target;
                }
            }
        }
        path.to_string()
    }

    pub fn start_apps(path1: &str, path2: &str) -> Result<Self, ProcessError> {
        let resolved_path1 = Self::resolve_path(path1);
        let resolved_path2 = Self::resolve_path(path2);

        let app1 = Command::new(&resolved_path1)
            .spawn()
            .map_err(|source| ProcessError::StartError { path: path1.to_string(), source })?;
            
        let app2 = Command::new(&resolved_path2)
            .spawn()
            .map_err(|source| ProcessError::StartError { path: path2.to_string(), source })?;

        println!("🟢 Applications started successfully.");
        Ok(Self { app1, app2 })
    }

    pub fn are_both_running(&mut self) -> bool {
        let status1 = self.app1.try_wait().unwrap_or(Some(std::process::ExitStatus::default()));
        let status2 = self.app2.try_wait().unwrap_or(Some(std::process::ExitStatus::default()));
        
        status1.is_none() && status2.is_none()
    }

    pub fn kill_all(&mut self) {
        let _ = self.app1.kill();
        let _ = self.app2.kill();
        println!("🛑 Test finished. Applications stopped.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_path_normal() {
        let path = "C:/Program Files/NormalApp/app.exe";
        assert_eq!(ProcessManager::resolve_path(path), path);
    }
}