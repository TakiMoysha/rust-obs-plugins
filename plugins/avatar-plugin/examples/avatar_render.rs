use macroquad::prelude::*;
use std::path::Path;
use avatarplugin::loader::{Avatar, ImageData};



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
    let avatar_path = Path::new("assets/bongo_cat/avatar.json");
    
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
    // For simplicity, let's just load the default face if available
    let default_face_name = avatar.settings.as_ref().and_then(|s| s.default_face.as_ref());
    let face_tex = if let Some(face_name) = default_face_name {
         avatar.get_face_by_key(face_name).map(load_texture_from_image_data)
    } else {
        None
    };

    loop {
        if is_key_down(KeyCode::Escape) {
            break;
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

        draw_text(&format!("Mode: {}", mode.name), 20.0, 20.0, 30.0, BLACK);
        draw_text("Press ESC to exit", 20.0, 50.0, 20.0, DARKGRAY);

        next_frame().await
    }
}
