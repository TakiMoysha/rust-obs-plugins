use std::error::Error;
use std::fmt;

#[cfg(all(target_os = "linux", feature = "wayland"))]
use std::os::unix::io::AsRawFd;

/// Represents different types of input events
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// Key press event with key code
    KeyPress(u32),
    /// Key release event with key code
    KeyRelease(u32),
    /// Mouse move event with delta values (x, y)
    MouseMove(i32, i32),
    /// Mouse button press event with button code
    MouseButtonPress(u32),
    /// Mouse button release event with button code
    MouseButtonRelease(u32),
    /// Mouse scroll event with delta values (horizontal, vertical)
    MouseScroll(i32, i32),
}

/// Error type for input capture operations
#[derive(Debug, thiserror::Error)]
pub enum InputCaptureError {
    #[error("Failed to initialize input capture: {0}")]
    InitError(String),
    #[error("Failed to poll events: {0}")]
    PollError(String),
    #[error("Platform not supported")]
    UnsupportedPlatform,
}

/// Main struct for capturing input events
pub struct InputCapture {
    #[cfg(target_os = "windows")]
    inner: windows::WindowsInputCapture,

    #[cfg(all(target_os = "linux", feature = "x11"))]
    inner: x11::X11InputCapture,

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    inner: wayland::WaylandInputCapture,

    #[cfg(not(any(
        target_os = "windows",
        all(target_os = "linux", feature = "x11"),
        all(target_os = "linux", feature = "wayland")
    )))]
    inner: unsupported::UnsupportedInputCapture,
}

impl InputCapture {
    /// Creates a new InputCapture instance
    pub fn new() -> Result<Self, InputCaptureError> {
        #[cfg(target_os = "windows")]
        let inner = windows::WindowsInputCapture::new()?;

        #[cfg(all(target_os = "linux", feature = "x11"))]
        let inner = x11::X11InputCapture::new()?;

        #[cfg(all(target_os = "linux", feature = "wayland"))]
        let inner = wayland::WaylandInputCapture::new()?;

        #[cfg(not(any(
            target_os = "windows",
            all(target_os = "linux", feature = "x11"),
            all(target_os = "linux", feature = "wayland")
        )))]
        let inner = unsupported::UnsupportedInputCapture::new()?;

        Ok(Self { inner })
    }

    /// Polls for new input events.
    /// This method should be called periodically (e.g. in video_tick).
    /// Returns a list of events that occurred since the last poll.
    pub fn poll(&mut self) -> Vec<InputEvent> {
        self.inner.poll()
    }
}

// Platform-specific implementations

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    pub struct WindowsInputCapture {
        // TODO: Add Windows-specific fields
    }

    impl WindowsInputCapture {
        pub fn new() -> Result<Self, InputCaptureError> {
            Ok(Self {})
        }

        pub fn poll(&mut self) -> Vec<InputEvent> {
            // TODO: Implement Windows polling (e.g. GetAsyncKeyState or message loop check)
            Vec::new()
        }
    }
}

#[cfg(all(target_os = "linux", feature = "x11"))]
mod x11 {
    use super::*;

    pub struct X11InputCapture {
        // TODO: Add X11-specific fields
    }

    impl X11InputCapture {
        pub fn new() -> Result<Self, InputCaptureError> {
            Ok(Self {})
        }

        pub fn poll(&mut self) -> Vec<InputEvent> {
            // TODO: Implement X11 polling (XPending + XNextEvent)
            Vec::new()
        }
    }
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
mod wayland {
    use super::*;
    use evdev::{Device, InputEvent, Key};
    use std::os::unix::io::AsRawFd;
    use std::path::PathBuf;

    pub struct WaylandInputCapture {
        devices: Vec<Device>,
    }

    impl WaylandInputCapture {
        pub fn new() -> Result<Self, InputCaptureError> {
            // check access to /dev/input
            let input_dir = std::path::Path::new("/dev/input");
            if !input_dir.exists() {
                return Err(InputCaptureError::InitError(
                    "Directory /dev/input does not exist".to_string(),
                ));
            }

            // Находим все клавиатуры
            let mut keyboards = Vec::new();

            // Сканируем event* файлы
            if let Ok(entries) = std::fs::read_dir(input_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                        if fname.starts_with("event") {
                            if let Ok(device) = Device::open(&path) {
                                if is_keyboard(&device) {
                                    println!(
                                        "Found keyboard: {} ({})",
                                        device.name().unwrap_or("Unknown"),
                                        path.display()
                                    );
                                    keyboards.push(path);
                                }
                            }
                        }
                    }
                }
            }

            if keyboards.is_empty() {
                println!("Warning: No keyboard devices found in /dev/input/");
            } else {
                println!("Found {} keyboard device(s)", keyboards.len());
            }

            let mut devices = Vec::new();
            for path in keyboards {
                match Device::open(&path) {
                    Ok(mut device) => {
                        // Устанавливаем NON-BLOCKING режим
                        let fd = device.as_raw_fd();
                        unsafe {
                            let flags = libc::fcntl(fd, libc::F_GETFL);
                            if flags >= 0 {
                                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                            }
                        }

                        println!("Opened device (non-blocking): {}", path.display());
                        devices.push(device);
                    }
                    Err(e) => {
                        eprintln!("Failed to open device {}: {}", path.display(), e);
                    }
                }
            }

            Ok(Self { devices })
        }

        pub fn poll(&mut self) -> Vec<InputEvent> {
            let mut events = Vec::new();

            for device in &mut self.devices {
                // fetch_events is non-blocking (due to 0_NONBLOCK flag)
                match device.fetch_events() {
                    Ok(iterator) => {
                        for ev in iterator {
                            if let InputEventKind::Key(key) = ev.event_type() {
                                let event = match ev.value() {
                                    1 => Some(InputEvent::KeyPress(key.code().into())),
                                    0 => Some(InputEvent::KeyRelease(key.code().into())),
                                    _ => None, // Игнорируем repeat events (value=2)
                                };

                                if let Some(e) = event {
                                    events.push(e);
                                }
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) => {}
                }
            }

            events
        }
    }

    fn is_keyboard(device: &Device) -> bool {
        // Проверяем наличие клавиш A, Z и ENTER
        device.supported_keys().map_or(false, |keys| {
            keys.contains(Key::KEY_A) && keys.contains(Key::KEY_Z) && keys.contains(Key::KEY_ENTER)
        })
    }
}

#[cfg(not(any(
    target_os = "windows",
    all(target_os = "linux", feature = "x11"),
    all(target_os = "linux", feature = "wayland")
)))]
mod unsupported {
    use super::*;

    pub struct UnsupportedInputCapture;

    impl UnsupportedInputCapture {
        pub fn new() -> Result<Self, InputCaptureError> {
            Err(InputCaptureError::UnsupportedPlatform)
        }

        pub fn poll(&mut self) -> Vec<InputEvent> {
            Vec::new()
        }
    }
}
