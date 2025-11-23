//! Простой тест для проверки перехвата клавиш на Wayland
//!
//! Запуск:
//! ```bash
//! cargo run --example wayland_test --features wayland
//! ```

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

// Импортируем модуль напрямую из исходников
#[path = "../src/input_capture.rs"]
mod input_capture;

use input_capture::{InputCapture, InputEvent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Wayland Input Capture Test (Polling Mode) ===\n");
    println!("Starting keyboard capture...");
    println!("Press keys to see events.");
    println!("Press Ctrl+C to exit.\n");

    // Создаем экземпляр перехватчика
    let mut capture = InputCapture::new()?;

    println!("[DONE] Capture initialized successfully!\n");
    println!("Monitoring keyboard events...\n");

    // Флаг для остановки
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("\n\nReceived Ctrl+C, stopping...");
        r.store(false, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    let mut event_count = 0;

    // Главный цикл polling
    while running.load(Ordering::Relaxed) {
        let events = capture.poll();

        for event in events {
            event_count += 1;
            match event {
                InputEvent::KeyPress(key) => {
                    println!(
                        "[{}] Key PRESSED:  code={} (0x{:04X})",
                        event_count, key, key
                    );

                    match key {
                        1 => println!("\t\t -> ESC"),
                        28 => println!("\t\t -> ENTER"),
                        57 => println!("\t\t -> SPACE"),
                        30 => println!("\t\t -> A"),
                        48 => println!("\t\t -> B"),
                        _ => {}
                    }
                }
                InputEvent::KeyRelease(key) => {
                    println!(
                        "[{}] Key RELEASED: code={} (0x{:04X})",
                        event_count, key, key
                    );
                }
                _ => {}
            }
            if !running.load(Ordering::Relaxed) {
                break;
            }
        }

        // Имитируем частоту кадров 60 FPS
        thread::sleep(Duration::from_millis(16));
    }

    println!("\n=== Summary ===");
    println!("Total events captured: {}", event_count);
    println!("Input capture stopped.");

    Ok(())
}
