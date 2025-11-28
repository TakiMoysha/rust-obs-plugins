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

    fn render(&self, renderer: &dyn TextureRenderer, position: Vec2) {
        if let Some(ref tex) = self.texture {
            renderer.render(tex, position);
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

#[macroquad::main("Avatar Render - Decorator Pattern")]
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
    let right_hand_tex = mode
        .right_hand
        .as_ref()
        .map(|h| load_texture_from_image_data(&h.up_image));
    let face_tex = avatar
        .settings
        .as_ref()
        .and_then(|s| s.default_face.as_ref())
        .and_then(|face_name| {
            avatar
                .get_face_by_key(face_name)
                .map(load_texture_from_image_data)
        });

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
        Layer::new("cat_body", cat_bg_tex, cat_config),
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
    let mut enable_deformation = true;
    let start_time = get_time();

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

        // Poll input capture
        if let Some(ref mut capture) = input_capture {
            for event in capture.poll() {
                match event {
                    InputEvent::KeyPress(code) => {
                        pressed_keys.insert(code);
                        last_events.push(format!("↓ Key {:#06x}", code));
                        if last_events.len() > 10 {
                            last_events.remove(0);
                        }
                    }
                    InputEvent::KeyRelease(code) => {
                        pressed_keys.remove(&code);
                        last_events.push(format!("↑ Key {:#06x}", code));
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

        // Render all layers using appropriate renderer
        for layer in &layers {
            let renderer: &dyn TextureRenderer = if enable_deformation {
                &DeformationRenderer::new(layer.config.clone(), mouse_influence, current_time)
            } else {
                &simple_renderer
            };

            layer.render(renderer, Vec2::ZERO);
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

        let deform_color = if enable_deformation {
            DARKGREEN
        } else {
            RED
        };
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
                &format!("Mouse: ({:.2}, {:.2})", mouse_influence.x, mouse_influence.y),
                20.0,
                110.0,
                18.0,
                DARKGRAY,
            );
        }

        // Input capture status
        let status_y = 140.0;
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
