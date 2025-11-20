use obs_wrapper::{obs_register_module, obs_string, obs_sys, prelude::*, properties::*, source::*};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

// Avatar loader module
mod loader;

use loader::{Avatar, AvatarLoader, ImageData};

/// Кэш текстур для предотвращения повторной загрузки
struct TextureCache {
    /// Карта путь -> текстура OBS (raw pointer)
    textures: HashMap<PathBuf, *mut obs_sys::gs_texture_t>,
}

unsafe impl Send for TextureCache {}
unsafe impl Sync for TextureCache {}

impl TextureCache {
    fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    /// Получить текстуру или создать новую из ImageData
    /// Должно вызываться только в графическом контексте (video_render)
    fn get_or_create(&mut self, image: &ImageData) -> Option<*mut obs_sys::gs_texture_t> {
        if !self.textures.contains_key(&image.path) {
            unsafe {
                let data_ptr = image.data.as_ptr();
                let mut data_ptr_ptr = data_ptr;

                let texture = obs_sys::gs_texture_create(
                    image.width,
                    image.height,
                    obs_sys::gs_color_format_GS_RGBA,
                    1,
                    &mut data_ptr_ptr as *mut *const u8,
                    0,
                );

                if !texture.is_null() {
                    self.textures.insert(image.path.clone(), texture);
                }
            }
        }

        self.textures.get(&image.path).copied()
    }

    /// Очистить кэш
    fn clear(&mut self) {
        unsafe {
            for (_, texture) in self.textures.drain() {
                obs_sys::gs_texture_destroy(texture);
            }
        }
    }
}

impl Drop for TextureCache {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Главный источник аватара
struct AvatarSource {
    /// Ссылка на источник
    source: SourceRef,

    /// Avatar loader с кэшированием
    loader: AvatarLoader,

    /// Кэш текстур OBS
    texture_cache: TextureCache,

    /// Загруженный аватар
    avatar: Option<Avatar>,

    /// Текущий активный режим
    current_mode: String,

    /// Текущее выражение лица
    current_face: String,

    /// Состояние рук (левая и правая): текущий кадр анимации
    left_hand_frame: usize,
    right_hand_frame: usize,

    /// Нажатые клавиши (для анимации)
    pressed_keys: std::collections::HashSet<String>,

    /// Текущий уровень аудио (0.0 - 1.0)
    audio_level: f32,

    /// Флаг для определения, говорит ли аватар
    is_speaking: bool,

    /// Порог для определения речи
    speech_threshold: f32,

    /// Path to avatar_config.json
    avatar_path: PathBuf,

    /// Ширина и высота canvas
    width: u32,
    height: u32,
}

impl Sourceable for AvatarSource {
    fn get_id() -> ObsString {
        obs_string!("avatar_source")
    }

    fn get_type() -> SourceType {
        SourceType::Input
    }

    fn create(create: &mut CreatableSourceContext<Self>, source: SourceRef) -> Self {
        let settings = &create.settings;

        // Получаем путь к директории аватара
        let avatar_path = settings
            .get::<Cow<'_, str>>(obs_string!("avatar_path"))
            .map(|s| PathBuf::from(s.as_ref()))
            .unwrap_or_else(|| PathBuf::from("./assets/bongo_cat"));

        println!("Avatar path: {}", avatar_path.display());
        let width = settings.get(obs_string!("width")).unwrap_or(1280);
        let height = settings.get(obs_string!("height")).unwrap_or(768);
        let speech_threshold = settings
            .get(obs_string!("speech_threshold"))
            .unwrap_or(0.15);

        let current_mode = settings
            .get::<Cow<'_, str>>(obs_string!("mode"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "keyboard".to_string());

        // Загружаем аватар из конфиг-файла
        let avatar = if avatar_path.is_file() {
            println!("Loading avatar from config file: {}", avatar_path.display());
            match Avatar::load_from_config(&avatar_path) {
                Ok(av) => {
                    println!("✓ Avatar loaded successfully!");
                    println!("  Name: {}", av.name);
                    println!("  Available modes: {:?}", av.available_modes);
                    println!("  Face images: {} loaded", av.face_images.len());
                    println!("  Modes loaded: {}", av.modes.len());
                    Some(av)
                }
                Err(e) => {
                    eprintln!("✗ Failed to load avatar from config: {:?}", e);
                    None
                }
            }
        } else if avatar_path.is_dir() {
            println!("Loading avatar from directory: {}", avatar_path.display());
            match Avatar::load_from_file(&avatar_path) {
                Ok(av) => {
                    println!("✓ Avatar loaded successfully!");
                    println!("  Name: {}", av.name);
                    println!("  Available modes: {:?}", av.available_modes);
                    Some(av)
                }
                Err(e) => {
                    eprintln!("✗ Failed to load avatar from directory: {:?}", e);
                    None
                }
            }
        } else {
            eprintln!("✗ Avatar path is neither file nor directory: {}", avatar_path.display());
            None
        };

        if avatar.is_none() {
            eprintln!("Failed to load avatar from: {:?}", avatar_path);
        }

        Self {
            source,
            loader: AvatarLoader::new(),
            texture_cache: TextureCache::new(),
            avatar,
            current_mode,
            current_face: "f1".to_string(),
            left_hand_frame: 0,
            right_hand_frame: 0,
            pressed_keys: std::collections::HashSet::new(),
            audio_level: 0.0,
            is_speaking: false,
            speech_threshold,
            avatar_path,
            width,
            height,
        }
    }
}

impl GetNameSource for AvatarSource {
    fn get_name() -> ObsString {
        obs_string!("Avatar Source")
    }
}

impl GetWidthSource for AvatarSource {
    fn get_width(&mut self) -> u32 {
        self.width
    }
}

impl GetHeightSource for AvatarSource {
    fn get_height(&mut self) -> u32 {
        self.height
    }
}

impl GetPropertiesSource for AvatarSource {
    fn get_properties(&mut self) -> Properties {
        let mut properties = Properties::new();

        // Path to avatar config.json
        properties.add(
            obs_string!("avatar_path"),
            obs_string!("Avatar JSON file"),
            PathProp::new(PathType::File),
        );

        // Текущий режим (текстовое поле)
        properties.add(
            obs_string!("mode"),
            obs_string!("Current Mode (e.g., keyboard, standard)"),
            TextProp::new(TextType::Default),
        );

        // Размеры canvas
        properties.add(
            obs_string!("width"),
            obs_string!("Canvas Width"),
            NumberProp::new_int().with_range(100u32..=3840),
        );

        properties.add(
            obs_string!("height"),
            obs_string!("Canvas Height"),
            NumberProp::new_int().with_range(100u32..=2160),
        );

        // Порог для определения речи
        properties.add(
            obs_string!("speech_threshold"),
            obs_string!("Speech Detection Threshold"),
            NumberProp::new_float(0.01)
                .with_range(0.0..=1.0)
                .with_slider(),
        );

        // Скорость анимации
        properties.add(
            obs_string!("animation_speed"),
            obs_string!("Animation Speed"),
            NumberProp::new_float(0.1)
                .with_range(0.1..=20.0)
                .with_slider(),
        );

        properties
    }
}

impl UpdateSource for AvatarSource {
    fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
        // Обновляем путь к аватару и перезагружаем если изменился
        if let Some(path) = settings.get::<Cow<'_, str>>(obs_string!("avatar_path")) {
            println!("New avatar path: {}", path.as_ref());
            let new_path = PathBuf::from(path.as_ref());
            if new_path != self.avatar_path {
                self.avatar_path = new_path.clone();
                // Очищаем кэш текстур
                self.texture_cache.clear();

                // Перезагружаем аватар
                self.avatar = if new_path.is_file() {
                    Avatar::load_from_config(&new_path).ok()
                } else if new_path.is_dir() {
                    Avatar::load_from_file(&new_path).ok()
                } else {
                    None
                };

                if self.avatar.is_none() {
                    eprintln!("Failed to reload avatar from: {:?}", new_path);
                }
            }
        }

        // Обновляем текущий режим
        if let Some(mode) = settings.get::<Cow<'_, str>>(obs_string!("mode")) {
            self.current_mode = mode.to_string();
        }

        if let Some(width) = settings.get(obs_string!("width")) {
            self.width = width;
        }

        if let Some(height) = settings.get(obs_string!("height")) {
            self.height = height;
        }

        if let Some(threshold) = settings.get(obs_string!("speech_threshold")) {
            self.speech_threshold = threshold;
        }
    }
}

impl VideoTickSource for AvatarSource {
    fn video_tick(&mut self, _seconds: f32) {
        // Обновляем состояние речи на основе уровня аудио
        self.is_speaking = self.audio_level > self.speech_threshold;

        // TODO: Анимация рук на основе нажатых клавиш
        // TODO: Анимация рта при речи
    }
}

impl VideoRenderSource for AvatarSource {
    fn video_render(&mut self, _context: &mut GlobalContext, _render: &mut VideoRenderContext) {
        // Деструктуризация для раздельного заимствования полей
        let Self {
            texture_cache,
            avatar,
            current_mode,
            current_face,
            ..
        } = self;

        let Some(avatar) = avatar.as_ref() else {
            return;
        };
        
        let Some(mode) = avatar.get_mode(current_mode) else {
            static LOGGED_NO_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !LOGGED_NO_MODE.load(std::sync::atomic::Ordering::Relaxed) {
                eprintln!("✗ Mode '{}' not found. Available modes: {:?}", 
                         current_mode, avatar.available_modes);
                LOGGED_NO_MODE.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            return;
        };

        // Отладочный вывод один раз
        static FIRST_RENDER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
        if FIRST_RENDER.load(std::sync::atomic::Ordering::Relaxed) {
            println!("\n=== AVATAR RENDERING ===");
            println!("Mode: {}", current_mode);
            println!("Face: {}", current_face);
            println!("Has background: {}", mode.background.is_some());
            println!("Has cat_background: {}", mode.cat_background.is_some());
            println!("Has left_hand: {}", mode.left_hand.is_some());
            println!("Has right_hand: {}", mode.right_hand.is_some());
            println!("=========================\n");
            FIRST_RENDER.store(false, std::sync::atomic::Ordering::Relaxed);
        }

        unsafe {
            // Получаем базовый effect ОДИН раз
            let effect = obs_sys::obs_get_base_effect(obs_sys::obs_base_effect_OBS_EFFECT_DEFAULT);
            let image_param = obs_sys::gs_effect_get_param_by_name(
                effect,
                "image\0".as_ptr() as *const i8,
            );

            // Хелпер для рисования спрайта (ПРАВИЛЬНЫЙ способ)
            let mut draw_sprite = |image: &ImageData, x: f32, y: f32| {
                if let Some(tex_ptr) = texture_cache.get_or_create(image) {
                    // 1. Устанавливаем текстуру в шейдер
                    obs_sys::gs_effect_set_texture(image_param, tex_ptr);
                    
                    // 2. Применяем трансформацию (если нужна позиция)
                    if x != 0.0 || y != 0.0 {
                        obs_sys::gs_matrix_push();
                        let mut pos: obs_sys::vec3 = std::mem::zeroed();
                        let ptr = &mut pos as *mut obs_sys::vec3 as *mut f32;
                        *ptr.offset(0) = x;
                        *ptr.offset(1) = y;
                        *ptr.offset(2) = 0.0;
                        *ptr.offset(3) = 0.0;
                        obs_sys::gs_matrix_translate(&mut pos);
                    }
                    
                    // 3. Рисуем спрайт
                    obs_sys::gs_draw_sprite(tex_ptr, 0, image.width, image.height);
                    
                    // 4. Восстанавливаем матрицу
                    if x != 0.0 || y != 0.0 {
                        obs_sys::gs_matrix_pop();
                    }
                }
            };

            // РЕНДЕРИМ ВСЕ СЛОИ (БЕЗ gs_effect_loop!)
            
            // 1. Отрисовка фона
            if let Some(ref bg) = mode.background {
                draw_sprite(bg, 0.0, 0.0);
            }

            // 2. Отрисовка тела кота
            if let Some(ref cat) = mode.cat_background {
                draw_sprite(cat, 0.0, 0.0);
            }

            // 3. Отрисовка лица
            if let Some(face) = avatar.face_images.get(current_face) {
                draw_sprite(face, 0.0, 0.0);
            }

            // 4. Отрисовка рук (пока только поднятые)
            if let Some(ref hand) = mode.left_hand {
                draw_sprite(&hand.up_image, 0.0, 0.0);
            }

            if let Some(ref hand) = mode.right_hand {
                draw_sprite(&hand.up_image, 0.0, 0.0);
            }
        }

        // Отладочный вывод (реже)
        use std::sync::atomic::{AtomicUsize, Ordering};
        static FRAME_COUNT: AtomicUsize = AtomicUsize::new(0);
        let frame = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
        if frame % 300 == 0 {
            println!("✓ Rendered frame {}", frame);
        }
    }
}

impl KeyClickSource for AvatarSource {
    fn key_click(&mut self, _event: obs_sys::obs_key_event, pressed: bool) {
        let Some(ref avatar) = self.avatar else {
            return;
        };

        // Преобразуем код клавиши в строку (упрощенная версия)
        // TODO: Реализовать полный маппинг vkey -> string
        let key_str = "unknown".to_string(); // Placeholder

        if pressed {
            // Добавляем в набор нажатых клавиш
            self.pressed_keys.insert(key_str.clone());

            // Проверяем, есть ли это выражение лица
            if let Some(_face_img) = avatar.get_face_by_key(&key_str) {
                self.current_face = key_str.clone();
                // TODO: Обновить текстуру лица
            }

            // Проверяем, есть ли это клавиша в текущем режиме
            if let Some(mode) = avatar.get_mode(&self.current_mode) {
                if let Some(_key_img) = mode.key_images.get(&key_str) {
                    // TODO: Показать анимацию нажатия клавиши
                    // TODO: Анимировать руки
                }
            }
        } else {
            // Убираем из набора нажатых клавиш
            self.pressed_keys.remove(&key_str);
        }
    }
}

impl MouseClickSource for AvatarSource {
    fn mouse_click(
        &mut self,
        _event: obs_sys::obs_mouse_event,
        button: MouseButton,
        pressed: bool,
        _click_count: u8,
    ) {
        if !pressed {
            return;
        }

        // TODO: Добавить логику реакции на клики мыши
        match button {
            MouseButton::Left => {
                // Например, показать указывающий жест
                // self.point_gesture();
            }
            MouseButton::Right => {
                // Другая реакция
            }
            _ => {}
        }
    }
}

impl MouseMoveSource for AvatarSource {
    fn mouse_move(&mut self, event: obs_sys::obs_mouse_event, _leave: bool) {
        // TODO: Добавить логику отслеживания мыши глазами аватара
        // let mouse_x = event.x;
        // let mouse_y = event.y;

        // self.look_at(mouse_x, mouse_y);
    }
}

// impl FilterAudioSource для обработки аудио входа
// Если вы хотите, чтобы это был фильтр, а не источник
// Раскомментируйте этот блок и измените get_type() на SourceType::Filter

/*
impl FilterAudioSource for AvatarSource {
    fn filter_audio(&mut self, audio: &mut AudioDataContext) {
        // Вычисляем уровень аудио для определения речи
        if let Some(channel_data) = audio.get_channel_as_mut_slice(0) {
            let mut sum = 0.0;
            for sample in channel_data.iter() {
                sum += sample.abs();
            }

            self.audio_level = sum / channel_data.len() as f32;
        }
    }
}
*/

// Plugin Module
struct AvatarModule {
    context: ModuleRef,
}

impl Module for AvatarModule {
    fn new(context: ModuleRef) -> Self {
        Self { context }
    }

    fn get_ctx(&self) -> &ModuleRef {
        &self.context
    }

    fn load(&mut self, load_context: &mut LoadContext) -> bool {
        let source = load_context
            .create_source_builder::<AvatarSource>()
            .enable_get_name()
            .enable_get_width()
            .enable_get_height()
            .enable_get_properties()
            .enable_update()
            .enable_video_tick()
            .enable_video_render()
            .enable_key_click()
            .enable_mouse_click()
            .enable_mouse_move()
            // TODO: Uncomment when FilterAudioSource is implemented
            // .enable_filter_audio()
            .build();

        load_context.register_source(source);

        true
    }

    fn description() -> ObsString {
        obs_string!(
            "A virtual avatar with animated PNG parts that respond to keyboard, mouse, and audio input events."
        )
    }

    fn name() -> ObsString {
        obs_string!("Avatar Plugin")
    }

    fn author() -> ObsString {
        obs_string!("TakiMoysha")
    }
}

obs_register_module!(AvatarModule);
