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
    #[serde(rename = "version")]
    pub version: Option<String>,

    #[serde(rename = "BackgroundImageName")]
    pub background_image: String,

    #[serde(rename = "CatBackgroundImageName")]
    pub cat_background_image: String,

    #[serde(rename = "HasModel")]
    pub has_model: bool,

    #[serde(rename = "CatModelPath")]
    pub cat_model_path: Option<String>,

    // New KeyMapping structure: key_name -> [key_image_path, hand_image_path]
    #[serde(rename = "KeyMapping")]
    pub key_mapping: Option<HashMap<String, Vec<String>>>,

    // Hand up images (direct paths from mode root)
    #[serde(rename = "LeftHandUpImageName")]
    pub left_hand_up_image: Option<String>,

    #[serde(rename = "RightHandUpImageName")]
    pub right_hand_up_image: Option<String>,

    // Legacy fields for backward compatibility
    #[serde(rename = "KeysImagePath")]
    pub keys_image_path: Option<String>,

    #[serde(rename = "KeysImageName")]
    pub keys_images: Option<Vec<String>>,

    #[serde(rename = "KeyUse")]
    pub key_bindings: Option<Vec<String>>,

    #[serde(rename = "LeftHandImagePath")]
    pub left_hand_image_path: Option<String>,

    #[serde(rename = "LeftHandImageName")]
    pub left_hand_images: Option<Vec<String>>,

    #[serde(rename = "RightHandImagePath")]
    pub right_hand_image_path: Option<String>,

    #[serde(rename = "RightHandImageName")]
    pub right_hand_images: Option<Vec<String>>,

    // Model configuration
    #[serde(rename = "ModelHasLeftHandModel")]
    pub has_left_hand_model: bool,

    #[serde(rename = "ModelLeftHandModelPath")]
    pub left_hand_model_path: Option<String>,

    #[serde(rename = "ModelHasRightHandModel")]
    pub has_right_hand_model: bool,

    #[serde(rename = "ModelRightHandModelPath")]
    pub right_hand_model_path: Option<String>,
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

    // Keys: key_name -> key_image
    pub key_images: HashMap<String, ImageData>,
    
    // Hand frames for each key: keycode -> hand_frame_image
    pub left_hand_key_frames: HashMap<u32, ImageData>,
    pub right_hand_key_frames: HashMap<u32, ImageData>,

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
            left_hand_key_frames: HashMap::new(),
            right_hand_key_frames: HashMap::new(),
            face_images: Vec::new(),
        };

        // Load background images
        loaded.background = Self::load_optional_image(mode_path, &config.background_image);
        loaded.cat_background = Self::load_optional_image(mode_path, &config.cat_background_image);

        // Load hands using new direct path format
        if let Some(ref left_up_path) = config.left_hand_up_image {
            if !left_up_path.is_empty() {
                if let Ok(up_image) = ImageData::load(&mode_path.join(left_up_path)) {
                    loaded.left_hand = Some(HandData {
                        up_image,
                        frame_images: Vec::new(), // Will be filled from KeyMapping
                    });
                }
            }
        }

        if let Some(ref right_up_path) = config.right_hand_up_image {
            if !right_up_path.is_empty() {
                if let Ok(up_image) = ImageData::load(&mode_path.join(right_up_path)) {
                    loaded.right_hand = Some(HandData {
                        up_image,
                        frame_images: Vec::new(), // Will be filled from KeyMapping
                    });
                }
            }
        }

        // Load from new KeyMapping structure
        if let Some(ref key_mapping) = config.key_mapping {
            // Create key name -> keycode mapping
            let key_to_code = Self::get_key_code_mapping();
            
            for (key_name, paths) in key_mapping {
                // paths[0] = key image path, paths[1] = hand image path
                if paths.len() >= 2 {
                    // Load key image
                    let key_img_path = mode_path.join(&paths[0]);
                    if let Ok(key_img) = ImageData::load(&key_img_path) {
                        loaded.key_images.insert(key_name.clone(), key_img);
                    }

                    // Load hand frame image and determine which hand
                    // Load hand frame image and determine which hand
                    let hand_img_path = mode_path.join(&paths[1]);
                    if let Ok(hand_img) = ImageData::load(&hand_img_path) {
                        // Try to parse key as number first, then look up in map
                        let keycode_opt = key_name.parse::<u32>().ok()
                            .or_else(|| key_to_code.get(key_name.as_str()).cloned());

                        if let Some(keycode) = keycode_opt {
                            // Determine which hand based on path or key code
                            // If path contains "lefthand", it's left hand.
                            // If path contains "righthand", it's right hand.
                            // Fallback: arrow keys (103, 108, 105, 106) are right hand, others left.
                            let is_right_hand = paths[1].contains("righthand") || 
                                                [103, 108, 105, 106].contains(&keycode);

                            if is_right_hand {
                                loaded.right_hand_key_frames.insert(keycode, hand_img);
                            } else {
                                loaded.left_hand_key_frames.insert(keycode, hand_img);
                            }
                        }
                    }
                }
            }
        } else {
            // Fallback to legacy format for backward compatibility
            Self::load_legacy_keys(&mut loaded, mode_path, &config)?;
        }

        Ok(loaded)
    }

    // Legacy key loading for backward compatibility
    fn load_legacy_keys(loaded: &mut LoadedMode, mode_path: &Path, config: &ModeConfig) -> Result<()> {
        // Load left hand (legacy)
        if let Some(path) = &config.left_hand_image_path {
            if !path.is_empty() {
                loaded.left_hand = Self::load_hand_data(
                    mode_path,
                    path,
                    config.left_hand_up_image.as_deref(),
                    config.left_hand_images.as_ref(),
                )
                .ok();
            }
        }

        // Load right hand (legacy)
        if let Some(path) = &config.right_hand_image_path {
            if !path.is_empty() {
                loaded.right_hand = Self::load_hand_data(
                    mode_path,
                    path,
                    config.right_hand_up_image.as_deref(),
                    config.right_hand_images.as_ref(),
                )
                .ok();
            }
        }

        // Load key images (legacy)
        if let (Some(key_path), Some(key_images), Some(key_bindings)) = (
            &config.keys_image_path,
            &config.keys_images,
            &config.key_bindings,
        ) {
            if !key_path.is_empty() {
                let keys_dir = mode_path.join(key_path);
                for (i, key_name) in key_bindings.iter().enumerate() {
                    if let Some(image_name) = key_images.get(i) {
                        if !image_name.is_empty() {
                            let img_path = keys_dir.join(image_name);
                            if let Ok(img) = ImageData::load(&img_path) {
                                loaded.key_images.insert(key_name.clone(), img);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    // Get evdev keycode mapping for key names
    fn get_key_code_mapping() -> HashMap<&'static str, u32> {
        let mut map = HashMap::new();
        
        // Control keys
        map.insert("lctrl", 29);
        map.insert("rctrl", 97);
        map.insert("lshift", 42);
        map.insert("rshift", 54);
        map.insert("lalt", 56);
        map.insert("ralt", 100);
        map.insert("space", 57);
        map.insert("enter", 28);
        map.insert("tab", 15);
        map.insert("backspace", 14);
        map.insert("escape", 1);
        
        // Arrow keys
        map.insert("up", 103);
        map.insert("down", 108);
        map.insert("left", 105);
        map.insert("right", 106);
        
        // Letters
        map.insert("a", 30); map.insert("b", 48); map.insert("c", 46); map.insert("d", 32);
        map.insert("e", 18); map.insert("f", 33); map.insert("g", 34); map.insert("h", 35);
        map.insert("i", 23); map.insert("j", 36); map.insert("k", 37); map.insert("l", 38);
        map.insert("m", 50); map.insert("n", 49); map.insert("o", 24); map.insert("p", 25);
        map.insert("q", 16); map.insert("r", 19); map.insert("s", 31); map.insert("t", 20);
        map.insert("u", 22); map.insert("v", 47); map.insert("w", 17); map.insert("x", 45);
        map.insert("y", 21); map.insert("z", 44);
        
        // Numbers
        map.insert("0", 11); map.insert("1", 2); map.insert("2", 3); map.insert("3", 4);
        map.insert("4", 5); map.insert("5", 6); map.insert("6", 7); map.insert("7", 8);
        map.insert("8", 9); map.insert("9", 10);
        
        map
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

/// Settings from avatar.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AvatarSettings {
    pub default_mode: String,
    pub default_face: Option<String>,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub fps: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AvatarConfigInner {
    pub settings: AvatarSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AvatarConfigFile {
    pub avatar: AvatarConfigInner,
}

/// Complete avatar with all modes and face expressions
#[derive(Debug)]
pub struct Avatar {
    pub name: String,
    pub base_path: PathBuf,
    pub config_path: PathBuf, // Fixed typo: config_pahth -> config_path

    pub face_config: FaceConfig,
    pub face_images: HashMap<String, ImageData>,

    pub available_modes: Vec<String>,
    pub modes: HashMap<String, LoadedMode>,

    pub settings: Option<AvatarSettings>,
}

impl Avatar {
    /// Load avatar from JSON config file (e.g., "avatar.json")
    pub fn load_from_config(config_path: &Path) -> Result<Self> {
        // 1. Parse config file to get settings
        let content = fs::read_to_string(config_path)?;
        // We use a lenient parse or just try to parse what we need
        // If parsing fails, we might still want to load the avatar but without settings?
        // For now, let's assume if it's a config file, it must be valid.
        let config_file: AvatarConfigFile =
            serde_json::from_str(&content).map_err(LoadError::JsonError)?;

        // 2. Get base directory
        let base_path = config_path
            .parent()
            .ok_or_else(|| LoadError::InvalidConfig("Invalid config path".into()))?;

        // 3. Load resources using base path
        let mut avatar = Self::load_from_file(base_path)?;

        // 4. Attach settings and correct config path
        avatar.settings = Some(config_file.avatar.settings);
        avatar.config_path = config_path.to_path_buf();

        Ok(avatar)
    }

    /// Load avatar from directory (e.g., "bongo_cat")
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let canonical_config_path = path
            .canonicalize()
            .map_err(|_| LoadError::InvalidConfig("Invalid config path".into()))?;

        // If path is a file, get parent. If dir, use it.
        // But this method assumes 'path' is the directory containing 'face', 'mode' etc.
        // The previous implementation of load_from_file logic:

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
            config_path: canonical_config_path, // Will be updated if loaded via config
            face_config,
            face_images,
            available_modes: mode_list.model_paths,
            modes,
            settings: None,
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

// ======================================================= !TODO: refactored later...

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

    #[ignore = "WIP"]
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
