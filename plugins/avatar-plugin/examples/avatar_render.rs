use macroquad::prelude::*;
use std::path::Path;
use std::collections::HashSet;
use avatarplugin::loader::{Avatar, ImageData};
use avatarplugin::input_capture::{InputCapture, InputEvent};

fn load_texture_from_image_data(image_data: &ImageData) -> Texture2D {
    Texture2D::from_rgba8(
        image_data.width as u16,
        image_data.height as u16,
        &image_data.data
    )
}

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

    // Select mode (default or first available)
    let mode_name = avatar.settings.as_ref()
        .map(|s| s.default_mode.as_str())
        .unwrap_or_else(|| avatar.available_modes.first().map(|s| s.as_str()).unwrap_or("keyboard"));
    
    let mode = avatar.get_mode(mode_name).expect("Failed to get default mode");
    println!("Active mode: {}", mode.name);

    // Upload textures to GPU
    let background_tex = mode.background.as_ref().map(load_texture_from_image_data);
    let cat_bg_tex = mode.cat_background.as_ref().map(load_texture_from_image_data);
    
    let left_hand_tex = mode.left_hand.as_ref().map(|h| load_texture_from_image_data(&h.up_image));
    let right_hand_tex = mode.right_hand.as_ref().map(|h| load_texture_from_image_data(&h.up_image));

    // Load face textures
    let default_face_name = avatar.settings.as_ref().and_then(|s| s.default_face.as_ref());
    let face_tex = if let Some(face_name) = default_face_name {
         avatar.get_face_by_key(face_name).map(load_texture_from_image_data)
    } else {
        None
    };

    // Initialize input capture
    let mut input_capture = match InputCapture::new() {
        Ok(capture) => {
            println!("✓ Input capture initialized");
            Some(capture)
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize input capture: {:?}", e);
            eprintln!("  Continuing without input capture (macroquad keys only)");
            None
        }
    };

    // Track pressed keys
    let mut pressed_keys: HashSet<u32> = HashSet::new();
    let mut last_events: Vec<String> = Vec::new();

    loop {
        // Check for ESC key (macroquad)
        if is_key_down(KeyCode::Escape) {
            break;
        }

        // Poll input capture events
        if let Some(ref mut capture) = input_capture {
            let events = capture.poll();
            for event in events {
                match event {
                    InputEvent::KeyPress(code) => {
                        pressed_keys.insert(code);
                        last_events.push(format!("↓ Key {:#06x}", code));
                        // Keep only last 10 events
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

        clear_background(LIGHTGRAY);

        // Draw Background
        if let Some(tex) = &background_tex {
            draw_texture(tex, 0.0, 0.0, WHITE);
        }

        // Draw Cat Body
        if let Some(tex) = &cat_bg_tex {
            draw_texture(tex, 0.0, 0.0, WHITE);
        }

        // Draw Face
        if let Some(tex) = &face_tex {
             draw_texture(tex, 0.0, 0.0, WHITE);
        }

        // Draw Hands (Up position for now)
        if let Some(tex) = &left_hand_tex {
            draw_texture(tex, 0.0, 0.0, WHITE);
        }
        if let Some(tex) = &right_hand_tex {
            draw_texture(tex, 0.0, 0.0, WHITE);
        }

        // Draw UI overlay
        draw_text(&format!("Mode: {}", mode.name), 20.0, 20.0, 30.0, BLACK);
        draw_text("Press ESC to exit", 20.0, 50.0, 20.0, DARKGRAY);
        
        // Draw input capture status
        let status_y = 80.0;
        if input_capture.is_some() {
            draw_text(
                &format!("Input Capture: Active ({} keys pressed)", pressed_keys.len()),
                20.0,
                status_y,
                20.0,
                DARKGREEN
            );
        } else {
            draw_text("Input Capture: Disabled", 20.0, status_y, 20.0, RED);
        }

        // Draw pressed keys
        if !pressed_keys.is_empty() {
            let mut y = status_y + 30.0;
            draw_text("Pressed keys:", 20.0, y, 18.0, BLACK);
            y += 20.0;
            
            for (i, key) in pressed_keys.iter().enumerate() {
                if i >= 5 { // Show max 5 keys
                    draw_text("...", 40.0, y, 16.0, DARKGRAY);
                    break;
                }
                draw_text(&format!("  {:#06x}", key), 40.0, y, 16.0, BLUE);
                y += 18.0;
            }
        }

        // Draw recent events log
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
