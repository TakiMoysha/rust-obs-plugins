use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Errors that can occur during avatar loading
#[derive(Debug)]
pub enum LoadError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    InvalidConfig(String),
    MissingFile(PathBuf),
}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError::IoError(e)
    }
}

impl From<serde_json::Error> for LoadError {
    fn from(e: serde_json::Error) -> Self {
        LoadError::JsonError(e)
    }
}

pub type Result<T> = std::result::Result<T, LoadError>;

/// Face configuration (face expressions)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FaceConfig {
    #[serde(rename = "HotKey")]
    pub hot_keys: Vec<String>,

    #[serde(rename = "FaceImageName")]
    pub face_images: Vec<String>,
}

impl FaceConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let json_path = path.join("config.json");
        let content = fs::read_to_string(&json_path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

/// Mode configuration (list of available modes)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModeListConfig {
    #[serde(rename = "ModelPath")]
    pub model_paths: Vec<String>,
}

impl ModeListConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let json_path = path.join("config.json");
        let content = fs::read_to_string(&json_path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

/// Individual mode configuration (e.g., keyboard, standard, etc.)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModeConfig {
    #[serde(rename = "BackgroundImageName")]
    pub background_image: String,

    #[serde(rename = "CatBackgroundImageName")]
    pub cat_background_image: String,

    #[serde(rename = "HasModel")]
    pub has_model: bool,

    #[serde(rename = "CatModelPath")]
    pub cat_model_path: Option<String>,

    // Keys configuration
    #[serde(rename = "KeysImagePath")]
    pub keys_image_path: Option<String>,

    #[serde(rename = "KeysImageName")]
    pub keys_images: Option<Vec<String>>,

    #[serde(rename = "KeyUse")]
    pub key_bindings: Option<Vec<String>>,

    // Left hand configuration
    #[serde(rename = "ModelHasLeftHandModel")]
    pub has_left_hand_model: bool,

    #[serde(rename = "ModelLeftHandModelPath")]
    pub left_hand_model_path: Option<String>,

    #[serde(rename = "LeftHandImagePath")]
    pub left_hand_image_path: Option<String>,

    #[serde(rename = "LeftHandUpImageName")]
    pub left_hand_up_image: Option<String>,

    #[serde(rename = "LeftHandImageName")]
    pub left_hand_images: Option<Vec<String>>,

    // Right hand configuration
    #[serde(rename = "ModelHasRightHandModel")]
    pub has_right_hand_model: bool,

    #[serde(rename = "ModelRightHandModelPath")]
    pub right_hand_model_path: Option<String>,

    #[serde(rename = "RightHandImagePath")]
    pub right_hand_image_path: Option<String>,

    #[serde(rename = "RightHandUpImageName")]
    pub right_hand_up_image: Option<String>,

    #[serde(rename = "RightHandImageName")]
    pub right_hand_images: Option<Vec<String>>,
}

impl ModeConfig {
    pub fn load(mode_path: &Path) -> Result<Self> {
        let json_path = mode_path.join("config.json");
        let content = fs::read_to_string(&json_path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

/// Loaded image data
#[derive(Debug, Clone)]
pub struct ImageData {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl ImageData {
    pub fn load(path: &Path) -> Result<Self> {
        use image::GenericImageView;

        let img = image::open(path)
            .map_err(|e| LoadError::InvalidConfig(format!("Failed to load image: {}", e)))?;

        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();

        Ok(ImageData {
            path: path.to_path_buf(),
            width,
            height,
            data: rgba.into_raw(),
        })
    }
}

/// Hand state with multiple animation frames
#[derive(Debug, Clone)]
pub struct HandData {
    pub up_image: ImageData,
    pub frame_images: Vec<ImageData>,
}

/// Loaded mode with all assets
#[derive(Debug)]
pub struct LoadedMode {
    pub name: String,
    pub config: ModeConfig,
    pub base_path: PathBuf,

    // Images
    pub background: Option<ImageData>,
    pub cat_background: Option<ImageData>,

    // Hands
    pub left_hand: Option<HandData>,
    pub right_hand: Option<HandData>,

    // Keys
    pub key_images: HashMap<String, ImageData>,

    // Face expressions
    pub face_images: Vec<ImageData>,
}

impl LoadedMode {
    pub fn load(mode_path: &Path, mode_name: &str) -> Result<Self> {
        let config = ModeConfig::load(mode_path)?;

        let mut loaded = LoadedMode {
            name: mode_name.to_string(),
            config: config.clone(),
            base_path: mode_path.to_path_buf(),
            background: None,
            cat_background: None,
            left_hand: None,
            right_hand: None,
            key_images: HashMap::new(),
            face_images: Vec::new(),
        };

        // Load background images
        loaded.background = Self::load_optional_image(mode_path, &config.background_image);
        loaded.cat_background = Self::load_optional_image(mode_path, &config.cat_background_image);

        // Load left hand
        if let Some(path) = &config.left_hand_image_path {
            loaded.left_hand = Self::load_hand_data(
                mode_path,
                path,
                config.left_hand_up_image.as_deref(),
                config.left_hand_images.as_ref(),
            )
            .ok();
        }

        // Load right hand
        if let Some(path) = &config.right_hand_image_path {
            loaded.right_hand = Self::load_hand_data(
                mode_path,
                path,
                config.right_hand_up_image.as_deref(),
                config.right_hand_images.as_ref(),
            )
            .ok();
        }

        // Load key images
        if let (Some(key_path), Some(key_images), Some(key_bindings)) = (
            &config.keys_image_path,
            &config.keys_images,
            &config.key_bindings,
        ) {
            let keys_dir = mode_path.join(key_path);
            for (i, key_name) in key_bindings.iter().enumerate() {
                if let Some(image_name) = key_images.get(i) {
                    let image_path = keys_dir.join(image_name);
                    if let Ok(img) = ImageData::load(&image_path) {
                        loaded.key_images.insert(key_name.clone(), img);
                    }
                }
            }
        }

        Ok(loaded)
    }

    fn load_optional_image(base_path: &Path, name: &str) -> Option<ImageData> {
        let path = base_path.join(name);
        ImageData::load(&path).ok()
    }

    fn load_hand_data(
        base_path: &Path,
        hand_path: &str,
        up_image_name: Option<&str>,
        frame_names: Option<&Vec<String>>,
    ) -> Result<HandData> {
        let hand_dir = base_path.join(hand_path);

        // Load up image
        let up_image = if let Some(name) = up_image_name {
            ImageData::load(&hand_dir.join(name))?
        } else {
            return Err(LoadError::InvalidConfig("Missing up image for hand".into()));
        };

        // Load frame images
        let mut frame_images = Vec::new();
        if let Some(names) = frame_names {
            for name in names {
                let path = hand_dir.join(name);
                if let Ok(img) = ImageData::load(&path) {
                    frame_images.push(img);
                }
            }
        }

        Ok(HandData {
            up_image,
            frame_images,
        })
    }
}

/// Complete avatar with all modes and face expressions
#[derive(Debug)]
pub struct Avatar {
    pub name: String,
    pub base_path: PathBuf,
    pub config_pahth: PathBuf,

    pub face_config: FaceConfig,
    pub face_images: HashMap<String, ImageData>,

    pub available_modes: Vec<String>,
    pub modes: HashMap<String, LoadedMode>,
}

impl Avatar {
    /// Load avatar from JSON config file (e.g., "avatar.json")
    pub fn load_from_config(config_path: &Path) -> Result<Self> {
        // Get base directory from config file path
        let base_path = config_path
            .parent()
            .ok_or_else(|| LoadError::InvalidConfig("Invalid config path".into()))?;

        // Just use the existing load method with the base directory
        Self::load_from_file(base_path)
    }

    /// Load avatar from directory (e.g., "bongo_cat")
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let canonical_config_path = path
            .canonicalize()
            .map_err(|_| LoadError::InvalidConfig("Invalid config path".into()))?;
        let canonical_base_path = canonical_config_path
            .parent()
            .ok_or_else(|| LoadError::InvalidConfig("No parent directory".into()))?
            .to_path_buf();

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("avatar")
            .to_string();

        // Load face configuration
        let face_path = path.join("face");
        let face_config = FaceConfig::load(&face_path)?;

        // Load face images
        let mut face_images = HashMap::new();
        for (key, img_name) in face_config
            .hot_keys
            .iter()
            .zip(face_config.face_images.iter())
        {
            let img_path = face_path.join(img_name);
            if let Ok(img) = ImageData::load(&img_path) {
                face_images.insert(key.clone(), img);
            }
        }

        // Load mode list
        let mode_path = path.join("mode");
        let mode_list = ModeListConfig::load(&mode_path)?;

        // Load each mode
        let mut modes = HashMap::new();
        for mode_name in &mode_list.model_paths {
            let mode_dir = mode_path.join(mode_name);
            match LoadedMode::load(&mode_dir, mode_name) {
                Ok(loaded_mode) => {
                    modes.insert(mode_name.clone(), loaded_mode);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load mode '{}': {:?}", mode_name, e);
                }
            }
        }

        Ok(Avatar {
            name,
            base_path: path.to_path_buf(),
            config_pahth: canonical_config_path,
            face_config,
            face_images,
            available_modes: mode_list.model_paths,
            modes,
        })
    }

    /// Get a specific mode by name
    pub fn get_mode(&self, name: &str) -> Option<&LoadedMode> {
        self.modes.get(name)
    }

    /// Get face image by hotkey
    pub fn get_face_by_key(&self, key: &str) -> Option<&ImageData> {
        self.face_images.get(key)
    }

    /// Get default mode (first available)
    pub fn get_default_mode(&self) -> Option<&LoadedMode> {
        self.available_modes
            .first()
            .and_then(|name| self.modes.get(name))
    }
}

// ======================================================================

pub struct AvatarLoader {
    cache: HashMap<PathBuf, Avatar>,
}

impl AvatarLoader {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Load an avatar, using cache if available
    pub fn load(&mut self, path: &Path) -> Result<&Avatar> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        if !self.cache.contains_key(&canonical) {
            let avatar = Avatar::load_from_file(path)?;
            self.cache.insert(canonical.clone(), avatar);
        }

        Ok(self.cache.get(&canonical).unwrap())
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Reload an avatar
    pub fn reload(&mut self, path: &Path) -> Result<&Avatar> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        self.cache.remove(&canonical);
        self.load(path)
    }
}

impl Default for AvatarLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ====================================================================== WIP

// use serde::{Deserialize, Serialize};
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct AvatarConfig {
//     pub name: String,
//     pub version: String,
//     pub author: String,
//     pub description: String,
//     pub settings: Settings,
//     pub faces: Faces,
//     pub modes: Modes,
//     pub keybindings: Keybindings,
//     pub animation: Animation,
//     pub rendering: Rendering,
//     pub audio: Audio,
//     pub metadata: Metadata,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Settings {
//     pub default_mode: String,
//     pub default_face: String,
//     pub canvas_width: u32,
//     pub canvas_height: u32,
//     pub fps: u32,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Faces {
//     pub enabled: bool,
//     pub base_path: String,
//     pub config_file: String,
//     pub expressions: Vec<Expression>,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Expression {
//     pub name: String,
//     pub file: String,
//     pub description: String,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Modes {
//     pub enabled: bool,
//     pub base_path: String,
//     pub config_file: String,
//     pub available: Vec<Mode>,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Mode {
//     pub id: String,
//     pub name: String,
//     pub description: String,
//     pub config: String,
//     pub features: Vec<String>,
//     pub recommended: bool,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Keybindings {
//     pub face_expressions: HashMap<String, String>,
//     pub mode_switch: HashMap<String, String>,
//     pub special_actions: HashMap<String, String>,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Animation {
//     pub hand_speed: f32,
//     pub key_press_duration: f32,
//     pub face_transition_time: f32,
//     pub idle_animation: IdleAnimation,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct IdleAnimation {
//     pub enabled: bool,
//     pub breathing: bool,
//     pub breathing_speed: f32,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Rendering {
//     pub scale: f32,
//     pub position: Position,
//     pub layers: Layers,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Position {
//     pub x: i32,
//     pub y: i32,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Layers {
//     pub background: u32,
//     pub cat_body: u32,
//     pub left_hand: u32,
//     pub right_hand: u32,
//     pub keys: u32,
//     pub face: u32,
//     pub effects: u32,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Audio {
//     pub enabled: bool,
//     pub reactive: bool,
//     pub threshold: f32,
//     pub smoothing: f32,
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// pub struct Metadata {
//     pub created: String,
//     pub format_version: String,
//     pub compatible_with: String,
//     pub license: String,
//     pub source: String,
// }
//
// impl AvatarConfig {
//     pub fn load_from_file(path: &Path) -> Result<Self> {
//         let json = fs::read_to_string(path).map_err(LoadError::IoError)?;
//         serde_json::from_str(&json).map_err(LoadError::JsonError)
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_avatar() {
        let json = r#"
        {
            "avatar": {
                "name": "Bongo Cat",
                "version": "1.0.0",
                "author": "Original by @StrayRogue, Ported by TakiMoysha",
                "description": "Classic Bongo Cat avatar with keyboard mode support",
                "settings": {
                    "default_mode": "keyboard",
                    "default_face": "f1",
                    "canvas_width": 1280,
                    "canvas_height": 768,
                    "fps": 60
                },
                "faces": {
                    "enabled": true,
                    "base_path": "face",
                    "config_file": "face/config.json",
                    "expressions": {
                        "f1": {
                            "name": "Normal",
                            "file": "face/0.png",
                            "description": "Default neutral expression"
                        }
                    }
                },
                "modes": {
                    "enabled": true,
                    "base_path": "mode",
                    "config_file": "mode/config.json",
                    "available": [
                        {
                            "id": "keyboard",
                            "name": "Keyboard Mode",
                            "description": "Bongo Cat plays on keyboard",
                            "config": "mode/keyboard/config.json",
                            "features": ["hands", "keys", "background"],
                            "recommended": true
                        }
                    ]
                }
            }
        }
        "#;

        let avatar = AvatarConfig::load_from_json(json).unwrap();
        assert_eq!(config.name, "Bongo Cat");
    }

    #[test]
    fn test_avatar_loader() {
        let mut loader = AvatarLoader::new();
        assert_eq!(loader.cache.len(), 0);
    }
}
