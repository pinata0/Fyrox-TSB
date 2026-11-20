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
pub const DEFAULT_SFX_VOLUME: f32 = 0.65;
pub const DEFAULT_MUSIC_VOLUME: f32 = 0.35;
pub const DEFAULT_CHARACTER_VOICE_VOLUME: f32 = 0.75;
pub const DEFAULT_FRAME_LIMIT: u32 = 120;
pub const FRAME_LIMITS: [u32; 7] = [60, 90, 120, 144, 165, 240, 0];
pub const BASE_LOOK_SENSITIVITY: f32 = 0.0022;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub mouse_sensitivity: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub character_voice_volume: f32,
    pub vsync: bool,
    pub frame_limit: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: DEFAULT_SENSITIVITY,
            sfx_volume: DEFAULT_SFX_VOLUME,
            music_volume: DEFAULT_MUSIC_VOLUME,
            character_voice_volume: DEFAULT_CHARACTER_VOICE_VOLUME,
            vsync: false,
            frame_limit: DEFAULT_FRAME_LIMIT,
        }
    }
}

impl Settings {
    pub fn set_frame_limit(&mut self, limit: u32) {
        self.frame_limit = if limit == 0 { 0 } else {
            *FRAME_LIMITS[..6].iter().min_by_key(|&&n| n.abs_diff(limit)).unwrap()
        };
    }
    pub fn set_character_voice_volume(&mut self, value: f32) {
        self.character_voice_volume = if value.is_finite() {
            (value.clamp(0.0, 1.0) * 100.0).round() / 100.0
        } else {
            DEFAULT_CHARACTER_VOICE_VOLUME
        };
    }
    pub fn set_music_volume(&mut self, value: f32) {
        self.music_volume = if value.is_finite() {
            (value.clamp(0.0, 1.0) * 100.0).round() / 100.0
        } else {
            DEFAULT_MUSIC_VOLUME
        };
    }

    pub fn set_sfx_volume(&mut self, value: f32) {
        self.sfx_volume = if value.is_finite() {
            (value.clamp(0.0, 1.0) * 100.0).round() / 100.0
        } else {
            DEFAULT_SFX_VOLUME
        };
    }

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
        settings.set_sfx_volume(settings.sfx_volume);
        settings.set_music_volume(settings.music_volume);
        settings.set_character_voice_volume(settings.character_voice_volume);
        settings.set_frame_limit(settings.frame_limit);
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
            normalized.set_sfx_volume(self.sfx_volume);
            normalized.set_music_volume(self.music_volume);
            normalized.set_character_voice_volume(self.character_voice_volume);
            normalized.set_frame_limit(self.frame_limit);
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
    fn video_settings_migrate_and_preserve_unlimited_and_vsync() {
        let mut settings: Settings = serde_json::from_str(r#"{"mouse_sensitivity":2.35}"#).unwrap();
        assert!(!settings.vsync);
        assert_eq!(settings.frame_limit, DEFAULT_FRAME_LIMIT);
        settings.set_frame_limit(145);
        assert_eq!(settings.frame_limit, 144);
        settings.set_frame_limit(u32::MAX);
        assert_eq!(settings.frame_limit, 240);
        settings.set_frame_limit(1);
        assert_eq!(settings.frame_limit, 60);
        settings.set_frame_limit(0);
        settings.vsync = true;
        let decoded: Settings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(decoded, settings);
        assert_eq!(decoded.frame_limit, 0);
    }

    #[test]
    fn character_voice_migrates_and_keeps_independent_mute() {
        let mut settings: Settings = serde_json::from_str(r#"{"sfx_volume":0.2,"music_volume":0.3}"#).unwrap();
        assert_eq!(settings.character_voice_volume, DEFAULT_CHARACTER_VOICE_VOLUME);
        settings.set_character_voice_volume(0.0);
        let decoded: Settings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(decoded.character_voice_volume, 0.0);
        assert_eq!(decoded.sfx_volume, 0.2);
        assert_eq!(decoded.music_volume, 0.3);
        settings.set_character_voice_volume(2.0);
        assert_eq!(settings.character_voice_volume, 1.0);
        settings.set_character_voice_volume(f32::NAN);
        assert_eq!(settings.character_voice_volume, DEFAULT_CHARACTER_VOICE_VOLUME);
    }

    #[test]
    fn sfx_volume_migrates_old_settings_and_preserves_mute() {
        let old: Settings = serde_json::from_str(r#"{"mouse_sensitivity":2.35}"#).unwrap();
        assert_eq!(old.sfx_volume, DEFAULT_SFX_VOLUME);
        let mut settings = old;
        settings.set_sfx_volume(0.0);
        let decoded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(decoded.sfx_volume, 0.0);
        assert_eq!(decoded.mouse_sensitivity, 2.35);
        settings.set_sfx_volume(3.0);
        assert_eq!(settings.sfx_volume, 1.0);
        settings.set_sfx_volume(-1.0);
        assert_eq!(settings.sfx_volume, 0.0);
        settings.set_sfx_volume(f32::NAN);
        assert_eq!(settings.sfx_volume, DEFAULT_SFX_VOLUME);
    }

    #[test]
    fn music_volume_migrates_and_remains_independent() {
        let mut settings: Settings =
            serde_json::from_str(r#"{"mouse_sensitivity":2.35,"sfx_volume":0.65}"#).unwrap();
        assert_eq!(settings.music_volume, DEFAULT_MUSIC_VOLUME);
        settings.set_music_volume(0.0);
        assert_eq!(settings.sfx_volume, 0.65);
        assert_eq!(settings.mouse_sensitivity, 2.35);
        let decoded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(decoded.music_volume, 0.0);
        settings.set_music_volume(3.0);
        assert_eq!(settings.music_volume, 1.0);
        settings.set_music_volume(-1.0);
        assert_eq!(settings.music_volume, 0.0);
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            settings.set_music_volume(value);
            assert_eq!(settings.music_volume, DEFAULT_MUSIC_VOLUME);
        }
    }

    #[test]
    fn saves_and_reloads_across_instances_and_replaces_existing_file() {
        let root = std::env::temp_dir().join(format!("tsb-settings-test-{}", std::process::id()));
        let path = root.join("설정/settings.json");
        assert_eq!(Settings::load(&path).unwrap(), Settings::default());
        let mut settings = Settings::default();
        settings.set_sensitivity(2.35);
        settings.set_sfx_volume(0.35);
        settings.set_music_volume(0.2);
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
