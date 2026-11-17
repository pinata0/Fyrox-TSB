//! Per-user preferences, independent of the executable and its extraction cache.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const MIN_SENSITIVITY: f32 = 0.10;
pub const MAX_SENSITIVITY: f32 = 5.0;
pub const DEFAULT_SENSITIVITY: f32 = 1.0;
pub const BASE_LOOK_SENSITIVITY: f32 = 0.0022;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub mouse_sensitivity: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: DEFAULT_SENSITIVITY,
        }
    }
}

impl Settings {
    pub fn set_sensitivity(&mut self, value: f32) {
        self.mouse_sensitivity = if value.is_finite() {
            (value.clamp(MIN_SENSITIVITY, MAX_SENSITIVITY) * 100.0).round() / 100.0
        } else {
            DEFAULT_SENSITIVITY
        };
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let contents = match fs::read(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let mut settings: Self = serde_json::from_slice(&contents)?;
        settings.set_sensitivity(settings.mouse_sensitivity);
        Ok(settings)
    }

    /// Write beside the destination, then replace it atomically so interrupted writes
    /// cannot leave a truncated settings file. Unique names also allow multiple clients.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("Missing settings directory"))?;
        fs::create_dir_all(parent)?;
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let pending = parent.join(format!(
            ".settings-{}-{}.tmp",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&pending)?;
            let mut normalized = *self;
            normalized.set_sensitivity(self.mouse_sensitivity);
            file.write_all(&serde_json::to_vec_pretty(&normalized)?)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&pending, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(pending);
        }
        result
    }
}

pub fn settings_path() -> io::Result<PathBuf> {
    let nonempty = |key| std::env::var_os(key).filter(|value| !value.is_empty());
    #[cfg(target_os = "windows")]
    let root = nonempty("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| nonempty("USERPROFILE").map(|p| PathBuf::from(p).join("AppData/Local")));
    #[cfg(not(target_os = "windows"))]
    let root = nonempty("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| nonempty("HOME").map(|p| PathBuf::from(p).join(".config")));
    root.map(|root| root.join("TSB/settings.json"))
        .ok_or_else(|| io::Error::other("User settings directory is unavailable"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_reloads_across_instances_and_replaces_existing_file() {
        let root = std::env::temp_dir().join(format!("tsb-settings-test-{}", std::process::id()));
        let path = root.join("설정/settings.json");
        assert_eq!(Settings::load(&path).unwrap(), Settings::default());
        let mut settings = Settings::default();
        settings.set_sensitivity(2.35);
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), settings);
        settings.set_sensitivity(0.65);
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), settings);
        fs::write(&path, br#"{"mouse_sensitivity":900,"future_setting":true}"#).unwrap();
        assert_eq!(
            Settings::load(&path).unwrap().mouse_sensitivity,
            MAX_SENSITIVITY
        );
        fs::write(&path, b"{}").unwrap();
        assert_eq!(Settings::load(&path).unwrap(), Settings::default());
        fs::write(&path, b"broken json").unwrap();
        assert!(Settings::load(&path).is_err());
        // A failed save reports the error and does not silently claim persistence.
        assert!(settings.save(&path.join("settings.json")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sensitivity_rejects_nonfinite_values_and_clamps_bounds() {
        let mut settings = Settings::default();
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            settings.set_sensitivity(value);
            assert_eq!(settings.mouse_sensitivity, DEFAULT_SENSITIVITY);
        }
        settings.set_sensitivity(-1.0);
        assert_eq!(settings.mouse_sensitivity, MIN_SENSITIVITY);
        settings.set_sensitivity(8.0);
        assert_eq!(settings.mouse_sensitivity, MAX_SENSITIVITY);
    }
}
