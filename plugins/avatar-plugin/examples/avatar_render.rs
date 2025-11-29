use avatarplugin::input_capture::{InputCapture, InputEvent};
use avatarplugin::loader::{Avatar, ImageData};
use macroquad::prelude::*;
use std::collections::HashSet;
use std::path::Path;

fn load_texture_from_image_data(image_data: &ImageData) -> Texture2D {
    Texture2D::from_rgba8(
        image_data.width as u16,
        image_data.height as u16,
        &image_data.data,
    )
}

// ============================================================================
// DECORATOR PATTERN FOR RENDERING
// ============================================================================

/// Trait for rendering textures
trait TextureRenderer {
    fn render(&self, texture: &Texture2D, position: Vec2);
}

/// Simple renderer - draws texture as-is
struct SimpleRenderer;

impl TextureRenderer for SimpleRenderer {
    fn render(&self, texture: &Texture2D, position: Vec2) {
        draw_texture(texture, position.x, position.y, WHITE);
    }
}

/// Deformation configuration
#[derive(Debug, Clone)]
struct DeformConfig {
    pivot: Vec2,
    max_rotation: f32,
    max_translation: Vec2,
    breath_amplitude: f32,
}

impl Default for DeformConfig {
    fn default() -> Self {
        Self {
            pivot: Vec2::ZERO,
            max_rotation: 0.0,
            max_translation: Vec2::ZERO,
            breath_amplitude: 0.0,
        }
    }
}

/// Deformation renderer - decorates rendering with transformations
struct DeformationRenderer {
    config: DeformConfig,
    mouse_influence: Vec2,
    time: f32,
}

impl DeformationRenderer {
    fn new(config: DeformConfig, mouse_influence: Vec2, time: f32) -> Self {
        Self {
            config,
            mouse_influence,
            time,
        }
    }
}

impl TextureRenderer for DeformationRenderer {
    fn render(&self, texture: &Texture2D, position: Vec2) {
        let rotation = self.mouse_influence.x * self.config.max_rotation.to_radians();
        let translation = Vec2::new(
            self.mouse_influence.x * self.config.max_translation.x,
            self.mouse_influence.y * self.config.max_translation.y,
        );

        // Breathing animation (sine wave)
        let breath_offset = (self.time * 2.0).sin() * self.config.breath_amplitude;

        let final_position = position + translation + Vec2::new(0.0, breath_offset);

        draw_texture_ex(
            texture,
            final_position.x,
            final_position.y,
            WHITE,
            DrawTextureParams {
                dest_size: None,
                source: None,
                rotation,
                flip_x: false,
                flip_y: false,
                pivot: Some(self.config.pivot),
            },
        );
    }
}

/// Hand animation state
#[derive(Debug, Clone, Copy, PartialEq)]
enum HandState {
    Up,
    Down,
}

/// Key press animation renderer - swaps hand textures based on key presses
struct KeyPressAnimationRenderer<'a> {
    hand_state: HandState,
    frames: Option<&'a Vec<Texture2D>>,
    frame_index: usize,
}

impl<'a> KeyPressAnimationRenderer<'a> {
    fn new(hand_state: HandState, frames: Option<&'a Vec<Texture2D>>, frame_index: usize) -> Self {
        Self {
            hand_state,
            frames,
            frame_index,
        }
    }
}

impl<'a> TextureRenderer for KeyPressAnimationRenderer<'a> {
    fn render(&self, texture: &Texture2D, position: Vec2) {
        let tex_to_draw = match self.hand_state {
            HandState::Up => texture,
            HandState::Down => {
                // If we have frames, use the selected frame, otherwise use default texture
                if let Some(frames) = self.frames {
                    if !frames.is_empty() {
                        &frames[self.frame_index % frames.len()]
                    } else {
                        texture
                    }
                } else {
                    texture
                }
            }
        };

        draw_texture(tex_to_draw, position.x, position.y, WHITE);
    }
}

/// Layer - represents a drawable layer with optional texture
struct Layer {
    #[allow(dead_code)]
    name: String,
    texture: Option<Texture2D>,
    config: DeformConfig,
}

impl Layer {
    fn new(name: impl Into<String>, texture: Option<Texture2D>, config: DeformConfig) -> Self {
        Self {
            name: name.into(),
            texture,
            config,
        }
    }

    #[allow(dead_code)]
    fn render(&self, renderer: &dyn TextureRenderer, position: Vec2) {
        if let Some(ref tex) = self.texture {
            renderer.render(tex, position);
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

#[macroquad::main("Avatar Render")]
async fn main() {
    // Load avatar
    let avatar_path = Path::new("plugins/avatar-plugin/assets/bongo_cat/avatar.json");

    let avatar = match Avatar::load_from_config(avatar_path) {
        Ok(av) => av,
        Err(e) => {
            eprintln!("Failed to load avatar from {:?}: {:?}", avatar_path, e);
            return;
        }
    };

    println!("Loaded avatar: {}", avatar.name);
    println!("Available modes: {:?}", avatar.available_modes);

    // Select mode
    let mode_name = avatar
        .settings
        .as_ref()
        .map(|s| s.default_mode.as_str())
        .unwrap_or_else(|| {
            avatar
                .available_modes
                .first()
                .map(|s| s.as_str())
                .unwrap_or("keyboard")
        });

    let mode = avatar
        .get_mode(mode_name)
        .expect("Failed to get default mode");
    println!("Active mode: {}", mode.name);

    // Upload textures to GPU
    let background_tex = mode.background.as_ref().map(load_texture_from_image_data);
    let cat_bg_tex = mode
        .cat_background
        .as_ref()
        .map(load_texture_from_image_data);
    let left_hand_tex = mode
        .left_hand
        .as_ref()
        .map(|h| load_texture_from_image_data(&h.up_image));
    let left_hand_frames: Vec<Texture2D> = mode
        .left_hand
        .as_ref()
        .map(|h| {
            h.frame_images
                .iter()
                .map(load_texture_from_image_data)
                .collect()
        })
        .unwrap_or_default();

    let right_hand_tex = mode
        .right_hand
        .as_ref()
        .map(|h| load_texture_from_image_data(&h.up_image));
    let right_hand_frames: Vec<Texture2D> = mode
        .right_hand
        .as_ref()
        .map(|h| {
            h.frame_images
                .iter()
                .map(load_texture_from_image_data)
                .collect()
        })
        .unwrap_or_default();
    let face_tex = avatar
        .settings
        .as_ref()
        .and_then(|s| s.default_face.as_ref())
        .and_then(|face_name| {
            avatar
                .get_face_by_key(face_name)
                .map(load_texture_from_image_data)
        });

    // Load key textures
    let mut key_textures: std::collections::HashMap<String, Texture2D> =
        std::collections::HashMap::new();
    for (key_name, image_data) in &mode.key_images {
        key_textures.insert(key_name.clone(), load_texture_from_image_data(image_data));
    }

    // Create key mapping (key name -> evdev key code)
    // Common key codes from evdev (linux/input-event-codes.h)
    // This mapping should ideally come from a config file or be auto-detected
    let mut key_mapping: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    
    // Control keys
    key_mapping.insert("lctrl", 29);      // KEY_LEFTCTRL
    key_mapping.insert("rctrl", 97);      // KEY_RIGHTCTRL
    key_mapping.insert("lshift", 42);     // KEY_LEFTSHIFT
    key_mapping.insert("rshift", 54);     // KEY_RIGHTSHIFT
    key_mapping.insert("lalt", 56);       // KEY_LEFTALT
    key_mapping.insert("ralt", 100);      // KEY_RIGHTALT
    key_mapping.insert("space", 57);      // KEY_SPACE
    key_mapping.insert("enter", 28);      // KEY_ENTER
    key_mapping.insert("tab", 15);        // KEY_TAB
    key_mapping.insert("backspace", 14);  // KEY_BACKSPACE
    key_mapping.insert("escape", 1);      // KEY_ESC
    
    // Arrow keys
    key_mapping.insert("up", 103);        // KEY_UP
    key_mapping.insert("down", 108);      // KEY_DOWN
    key_mapping.insert("left", 105);      // KEY_LEFT
    key_mapping.insert("right", 106);     // KEY_RIGHT
    
    // Letter keys (a-z)
    key_mapping.insert("a", 30);
    key_mapping.insert("b", 48);
    key_mapping.insert("c", 46);
    key_mapping.insert("d", 32);
    key_mapping.insert("e", 18);
    key_mapping.insert("f", 33);
    key_mapping.insert("g", 34);
    key_mapping.insert("h", 35);
    key_mapping.insert("i", 23);
    key_mapping.insert("j", 36);
    key_mapping.insert("k", 37);
    key_mapping.insert("l", 38);
    key_mapping.insert("m", 50);
    key_mapping.insert("n", 49);
    key_mapping.insert("o", 24);
    key_mapping.insert("p", 25);
    key_mapping.insert("q", 16);
    key_mapping.insert("r", 19);
    key_mapping.insert("s", 31);
    key_mapping.insert("t", 20);
    key_mapping.insert("u", 22);
    key_mapping.insert("v", 47);
    key_mapping.insert("w", 17);
    key_mapping.insert("x", 45);
    key_mapping.insert("y", 21);
    key_mapping.insert("z", 44);
    
    // Number keys (0-9)
    key_mapping.insert("0", 11);
    key_mapping.insert("1", 2);
    key_mapping.insert("2", 3);
    key_mapping.insert("3", 4);
    key_mapping.insert("4", 5);
    key_mapping.insert("5", 6);
    key_mapping.insert("6", 7);
    key_mapping.insert("7", 8);
    key_mapping.insert("8", 9);
    key_mapping.insert("9", 10);

    println!("Key mapping loaded with {} entries", key_mapping.len());

    // Define which keys belong to which hand based on KeyUse from config
    // Right hand: arrow keys (up, down, left, right)
    // Left hand: everything else
    let mut right_hand_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut left_hand_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    
    if let Some(key_bindings) = &mode.config.key_bindings {
        for key_name in key_bindings {
            // Arrow keys go to right hand
            if key_name == "up" || key_name == "down" || key_name == "left" || key_name == "right" {
                right_hand_keys.insert(key_name.clone());
            } else {
                // Everything else goes to left hand
                left_hand_keys.insert(key_name.clone());
            }
        }
    }

    println!("Loaded {} key textures", key_textures.len());
    for key_name in key_textures.keys() {
        println!("  - {}", key_name);
    }
    println!("Left hand keys: {:?}", left_hand_keys);
    println!("Right hand keys: {:?}", right_hand_keys);

    // === DEFORMATION CONFIGS ===

    let background_config = DeformConfig::default(); // No deformation for background

    let cat_config = DeformConfig {
        pivot: Vec2::new(640.0, 400.0),
        max_rotation: 3.0,
        max_translation: Vec2::new(10.0, 5.0),
        breath_amplitude: 3.0,
    };

    let face_config = DeformConfig {
        pivot: Vec2::new(640.0, 300.0),
        max_rotation: 8.0,
        max_translation: Vec2::new(20.0, 15.0),
        breath_amplitude: 2.0,
    };

    let left_hand_config = DeformConfig {
        pivot: Vec2::new(100.0, 50.0),
        max_rotation: 15.0,
        max_translation: Vec2::new(5.0, 10.0),
        breath_amplitude: 1.0,
    };

    let right_hand_config = DeformConfig {
        pivot: Vec2::new(100.0, 50.0),
        max_rotation: -15.0,
        max_translation: Vec2::new(-5.0, 10.0),
        breath_amplitude: 1.0,
    };

    // Create layers
    let layers = vec![
        Layer::new("background", background_tex, background_config),
        Layer::new("cat_body", cat_bg_tex, cat_config.clone()),
        Layer::new("face", face_tex, face_config),
        Layer::new("left_hand", left_hand_tex, left_hand_config),
        Layer::new("right_hand", right_hand_tex, right_hand_config),
    ];

    // Initialize input capture
    let mut input_capture = match InputCapture::new() {
        Ok(capture) => {
            println!("✓ Input capture initialized");
            Some(capture)
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize input capture: {:?}", e);
            eprintln!("  Continuing without input capture");
            None
        }
    };

    // State
    let mut pressed_keys: HashSet<u32> = HashSet::new();
    let mut last_events: Vec<String> = Vec::new();
    let mut enable_deformation = false; // Deformation OFF by default
    let start_time = get_time();

    // Hand animation state
    let mut left_hand_state = HandState::Up;
    #[allow(unused_assignments)]
    let mut right_hand_state = HandState::Up;
    let mut left_hand_frame_index = 0;
    let mut right_hand_frame_index = 0;
    // Renderers
    let simple_renderer = SimpleRenderer;

    loop {
        let current_time = (get_time() - start_time) as f32;

        // Input handling
        if is_key_down(KeyCode::Escape) {
            break;
        }

        if is_key_pressed(KeyCode::D) {
            enable_deformation = !enable_deformation;
            println!(
                "Deformation: {}",
                if enable_deformation { "ON" } else { "OFF" }
            );
        }

        // Check if any pressed key belongs to left or right hand
        let mut left_hand_pressed = false;
        let mut right_hand_pressed = false;
        let mut left_hand_key_code: Option<u32> = None;
        let mut right_hand_key_code: Option<u32> = None;
        
        if let Some(key_bindings) = &mode.config.key_bindings {
            for key_name in key_bindings {
                if let Some(&key_code) = key_mapping.get(key_name.as_str()) {
                    if pressed_keys.contains(&key_code) {
                        // Check if this key belongs to left hand
                        if left_hand_keys.contains(key_name.as_str()) {
                            left_hand_pressed = true;
                            left_hand_key_code = Some(key_code);
                        }
                        // Check if this key belongs to right hand
                        if right_hand_keys.contains(key_name.as_str()) {
                            right_hand_pressed = true;
                            right_hand_key_code = Some(key_code);
                        }
                    }
                }
            }
        }

        // Update frame indices based on pressed keys for each hand
        if left_hand_pressed {
            if let Some(code) = left_hand_key_code {
                left_hand_frame_index = code as usize;
            }
        }
        
        if right_hand_pressed {
            if let Some(code) = right_hand_key_code {
                right_hand_frame_index = code as usize;
            }
        }

        // Update hand states independently
        left_hand_state = if left_hand_pressed {
            HandState::Down
        } else {
            HandState::Up
        };

        right_hand_state = if right_hand_pressed {
            HandState::Down
        } else {
            HandState::Up
        };

        // Poll input capture
        if let Some(ref mut capture) = input_capture {
            for event in capture.poll() {
                match event {
                    InputEvent::KeyPress(code) => {
                        pressed_keys.insert(code);
                        last_events.push(format!("Press {:#}", code));
                        if last_events.len() > 10 {
                            last_events.remove(0);
                        }
                    }
                    InputEvent::KeyRelease(code) => {
                        pressed_keys.remove(&code);
                        last_events.push(format!("Release {:#}", code));
                        if last_events.len() > 10 {
                            last_events.remove(0);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Calculate mouse influence
        let mouse_pos = mouse_position();
        let screen_center = Vec2::new(screen_width() / 2.0, screen_height() / 2.0);
        let mouse_offset = Vec2::new(mouse_pos.0 - screen_center.x, mouse_pos.1 - screen_center.y);
        let mouse_influence = Vec2::new(
            (mouse_offset.x / screen_width()).clamp(-1.0, 1.0),
            (mouse_offset.y / screen_height()).clamp(-1.0, 1.0),
        );

        clear_background(LIGHTGRAY);

        // Render layers with appropriate renderers
        // Background
        if let Some(ref tex) = layers[0].texture {
            simple_renderer.render(tex, Vec2::ZERO);
        }

        // Cat body
        if let Some(ref tex) = layers[1].texture {
            if enable_deformation {
                let renderer = DeformationRenderer::new(
                    layers[1].config.clone(),
                    mouse_influence,
                    current_time,
                );
                renderer.render(tex, Vec2::ZERO);
            } else {
                simple_renderer.render(tex, Vec2::ZERO);
            }
        }

        // Face
        if let Some(ref tex) = layers[2].texture {
            if enable_deformation {
                let renderer = DeformationRenderer::new(
                    layers[2].config.clone(),
                    mouse_influence,
                    current_time,
                );
                renderer.render(tex, Vec2::ZERO);
            } else {
                simple_renderer.render(tex, Vec2::ZERO);
            }
        }

        // Draw pressed keys images (before hands so hands are on top)
        if let (Some(key_bindings), Some(key_images)) =
            (&mode.config.key_bindings, &mode.config.keys_images)
        {
            for (i, key_name) in key_bindings.iter().enumerate() {
                // Get the corresponding image name
                if let Some(_) = key_images.get(i) {
                    // Get the key code for this key name
                    if let Some(&key_code) = key_mapping.get(key_name.as_str()) {
                        // Check if key is pressed
                        if pressed_keys.contains(&key_code) {
                            // Draw the texture
                            // Note: key_textures is keyed by key_name (e.g. "lctrl"), not image_name
                            if let Some(tex) = key_textures.get(key_name.as_str()) {
                                // Apply deformation if enabled (keys usually move with the table/cat)
                                if enable_deformation {
                                    let renderer = DeformationRenderer::new(
                                        cat_config.clone(), // Use cat config for keys so they move with body
                                        mouse_influence,
                                        current_time,
                                    );
                                    renderer.render(tex, Vec2::ZERO);
                                } else {
                                    simple_renderer.render(tex, Vec2::ZERO);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Left hand - with key press animation (drawn after keys to be on top)
        if let Some(ref tex) = layers[3].texture {
            let renderer = KeyPressAnimationRenderer::new(
                left_hand_state,
                Some(&left_hand_frames),
                left_hand_frame_index,
            );
            renderer.render(tex, Vec2::ZERO);
        }

        // Right hand - with key press animation (drawn after keys to be on top)
        if let Some(ref tex) = layers[4].texture {
            let renderer = KeyPressAnimationRenderer::new(
                right_hand_state,
                Some(&right_hand_frames),
                right_hand_frame_index,
            );
            renderer.render(tex, Vec2::ZERO);
        }

        // UI overlay
        draw_text(&format!("Mode: {}", mode.name), 20.0, 20.0, 30.0, BLACK);
        draw_text(
            "Press ESC to exit | D to toggle deformation",
            20.0,
            50.0,
            20.0,
            DARKGRAY,
        );

        let deform_color = if enable_deformation { DARKGREEN } else { RED };
        draw_text(
            &format!(
                "Deformation: {}",
                if enable_deformation { "ON" } else { "OFF" }
            ),
            20.0,
            80.0,
            24.0,
            deform_color,
        );

        if enable_deformation {
            draw_text(
                &format!(
                    "Mouse: ({:.2}, {:.2})",
                    mouse_influence.x, mouse_influence.y
                ),
                20.0,
                110.0,
                18.0,
                DARKGRAY,
            );
        }

        // Hand animation state
        let hand_state_text = match left_hand_state {
            HandState::Up => "Hands: UP",
            HandState::Down => "Hands: DOWN",
        };
        draw_text(
            hand_state_text,
            20.0,
            if enable_deformation { 140.0 } else { 110.0 },
            18.0,
            if matches!(left_hand_state, HandState::Down) {
                DARKGREEN
            } else {
                DARKGRAY
            },
        );

        // Input capture status
        let status_y = if enable_deformation { 170.0 } else { 140.0 };
        if input_capture.is_some() {
            draw_text(
                &format!("Input Capture: Active ({} keys)", pressed_keys.len()),
                20.0,
                status_y,
                18.0,
                DARKGREEN,
            );
        } else {
            draw_text("Input Capture: Disabled", 20.0, status_y, 18.0, RED);
        }

        // Pressed keys
        if !pressed_keys.is_empty() {
            let mut y = status_y + 25.0;
            draw_text("Pressed:", 20.0, y, 16.0, BLACK);
            y += 18.0;

            for (i, key) in pressed_keys.iter().enumerate() {
                if i >= 5 {
                    draw_text("...", 40.0, y, 14.0, DARKGRAY);
                    break;
                }
                draw_text(&format!("{:#06x}", key), 40.0, y, 14.0, BLUE);
                y += 16.0;
            }
        }

        // Event log
        if !last_events.is_empty() {
            let log_x = screen_width() - 250.0;
            let mut y = 20.0;
            draw_text("Event Log:", log_x, y, 18.0, BLACK);
            y += 20.0;

            for event in last_events.iter().rev().take(10) {
                draw_text(event, log_x, y, 14.0, DARKGRAY);
                y += 16.0;
            }
        }

        next_frame().await
    }

    println!("Shutting down...");
}
