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



/// Renderer for the right hand that follows the mouse
struct HandRenderer {
    pivot: Vec2,      // Point on the texture to rotate around (shoulder)
    position: Vec2,   // Base position on screen
    mouse_pos: Vec2,  // Current mouse position
    scale: f32,
    source_rect: Option<Rect>, // Region of the texture to draw (for atlases)
}

impl HandRenderer {
    fn new(pivot: Vec2, position: Vec2, mouse_pos: Vec2, source_rect: Option<Rect>) -> Self {
        Self {
            pivot,
            position,
            mouse_pos,
            scale: 1.0,
            source_rect,
        }
    }
}

impl TextureRenderer for HandRenderer {
    fn render(&self, texture: &Texture2D, _position: Vec2) {
        // Calculate vector from pivot (in screen space) to mouse
        // Assuming the texture is drawn at self.position
        // The pivot is relative to the texture top-left
        let screen_pivot = self.position + self.pivot;
        
        let diff = self.mouse_pos - screen_pivot;
        
        // Calculate angle
        // We add an offset because the sprite might not be pointing exactly right/up at 0 degrees
        // Usually 0 degrees is 3 o'clock (Right). 
        // Let's assume the hand sprite points UP or LEFT by default. 
        // We might need to tweak this 'angle_offset'.
        let angle_offset = 90.0f32.to_radians(); 
        let rotation = diff.y.atan2(diff.x) + angle_offset;

        // Clamp rotation to avoid breaking the arm
        // let rotation = rotation.clamp(-1.0, 2.0); 

        draw_texture_ex(
            texture,
            self.position.x,
            self.position.y,
            WHITE,
            DrawTextureParams {
                dest_size: None,
                source: self.source_rect,
                rotation,
                flip_x: false,
                flip_y: false,
                pivot: Some(self.pivot),
            },
        );
        
        // Debug: draw pivot and target line
        // draw_circle(screen_pivot.x, screen_pivot.y, 5.0, RED);
        // draw_line(screen_pivot.x, screen_pivot.y, self.mouse_pos.x, self.mouse_pos.y, 2.0, BLUE);
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
    key_frames: Option<&'a std::collections::HashMap<u32, Texture2D>>,
    pressed_key_code: Option<u32>,
}

impl<'a> KeyPressAnimationRenderer<'a> {
    fn new(
        hand_state: HandState,
        key_frames: Option<&'a std::collections::HashMap<u32, Texture2D>>,
        pressed_key_code: Option<u32>,
    ) -> Self {
        Self {
            hand_state,
            key_frames,
            pressed_key_code,
        }
    }
}

impl<'a> TextureRenderer for KeyPressAnimationRenderer<'a> {
    fn render(&self, texture: &Texture2D, position: Vec2) {
        let tex_to_draw = match self.hand_state {
            HandState::Up => texture,
            HandState::Down => {
                // If we have key frames and a pressed key code, use that frame
                if let (Some(frames), Some(key_code)) = (self.key_frames, self.pressed_key_code) {
                    if let Some(frame_tex) = frames.get(&key_code) {
                        frame_tex
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
    let mode_name = "standard";
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
    
    // Load left hand key frames from the new structure (keycode -> texture)
    let mut left_hand_key_frames: std::collections::HashMap<u32, Texture2D> = 
        std::collections::HashMap::new();
    for (&keycode, image_data) in &mode.left_hand_key_frames {
        left_hand_key_frames.insert(keycode, load_texture_from_image_data(image_data));
    }

    let mut right_hand_tex = mode
        .right_hand
        .as_ref()
        .map(|h| load_texture_from_image_data(&h.up_image));
    
    // Load right hand key frames from the new structure (keycode -> texture)
    let mut right_hand_key_frames: std::collections::HashMap<u32, Texture2D> = 
        std::collections::HashMap::new();
    for (&keycode, image_data) in &mode.right_hand_key_frames {
        right_hand_key_frames.insert(keycode, load_texture_from_image_data(image_data));
    }

    // EXPERIMENTAL: Try to load Live2D texture for right hand if standard mode
    if right_hand_tex.is_none() && mode.name == "standard" {
        let model_texture_path = "plugins/avatar-plugin/assets/bongo_cat/mode/standard/model/cat right hand/cat ori right hand.512/texture_00.png";
        if let Ok(image_data) = ImageData::load(Path::new(model_texture_path)) {
            println!("✓ Loaded fallback Live2D texture for right hand");
            right_hand_tex = Some(load_texture_from_image_data(&image_data));
        } else {
            println!("! Could not load fallback texture: {}", model_texture_path);
        }
    }
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
    // Simple renderer for all layers
    let simple_renderer = SimpleRenderer;

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
        // Check if any pressed key belongs to left or right hand
        let mut left_hand_pressed = false;
        let mut right_hand_pressed = false;
        let mut left_hand_pressed_key: Option<u32> = None;
        let mut right_hand_pressed_key: Option<u32> = None;
        
        // Check if any pressed key has a corresponding hand frame
        for &key_code in &pressed_keys {
            // Check left hand
            if left_hand_key_frames.contains_key(&key_code) {
                left_hand_pressed = true;
                left_hand_pressed_key = Some(key_code);
            }
            
            // Check right hand
            if right_hand_key_frames.contains_key(&key_code) {
                right_hand_pressed = true;
                right_hand_pressed_key = Some(key_code);
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

        // Render layers
        // Background
        if let Some(ref tex) = background_tex {
            simple_renderer.render(tex, Vec2::ZERO);
        }

        // Cat body
        if let Some(ref tex) = cat_bg_tex {
            simple_renderer.render(tex, Vec2::ZERO);
        }

        // Face
        if let Some(ref tex) = face_tex {
            simple_renderer.render(tex, Vec2::ZERO);
        }

        // Draw pressed keys images (before hands so hands are on top)
        for (key_str, tex) in &key_textures {
            // Try to parse key string as keycode
            if let Ok(key_code) = key_str.parse::<u32>() {
                if pressed_keys.contains(&key_code) {
                    simple_renderer.render(tex, Vec2::ZERO);
                }
            }
        }

        // Left hand - with key press animation (drawn after keys to be on top)
        if let Some(ref tex) = left_hand_tex {
            let renderer = KeyPressAnimationRenderer::new(
                left_hand_state,
                Some(&left_hand_key_frames),
                left_hand_pressed_key,
            );
            renderer.render(tex, Vec2::ZERO);
        }

        // Right hand - with key press animation OR manual deformation
        if let Some(ref tex) = right_hand_tex {
            if !right_hand_key_frames.is_empty() {
                // Keyboard mode / Legacy with key frames
                let renderer = KeyPressAnimationRenderer::new(
                    right_hand_state,
                    Some(&right_hand_key_frames),
                    right_hand_pressed_key,
                );
                renderer.render(tex, Vec2::ZERO);
            } else {
                // Standard mode (Live2D fallback) - Follow mouse
                // We assume the texture is roughly 512x512.
                // Let's place the shoulder at (700, 400) and pivot at (50, 50) of the texture.
                
                let hand_pos = Vec2::new(700.0, 400.0); // Approximate shoulder position on screen
                let pivot = Vec2::new(50.0, 50.0);      // Pivot within the hand texture (top-left of hand)
                
                // If using atlas (texture_00.png), you can specify the region here.
                // For example: Some(Rect::new(0.0, 0.0, 200.0, 200.0))
                let source_rect = None; 

                let renderer = HandRenderer::new(
                    pivot,
                    hand_pos,
                    Vec2::new(mouse_pos.0, mouse_pos.1),
                    source_rect,
                );
                renderer.render(tex, Vec2::ZERO); 
            }
        }

        // UI overlay
        draw_text(&format!("Mode: {}", mode.name), 20.0, 20.0, 30.0, BLACK);
        draw_text(
            "Press ESC to exit",
            20.0,
            50.0,
            20.0,
            DARKGRAY,
        );



        // Hand animation state
        let hand_state_text = match left_hand_state {
            HandState::Up => "Hands: UP",
            HandState::Down => "Hands: DOWN",
        };
        draw_text(
            hand_state_text,
            20.0,
            80.0,
            18.0,
            if matches!(left_hand_state, HandState::Down) {
                DARKGREEN
            } else {
                DARKGRAY
            },
        );

        // Input capture status
        let status_y = 110.0;
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
