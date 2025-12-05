# Avatar Plugin

Плагин рендерит простой аватар в OBS. Перехватывает input с клавиатуры и мыши и анимирования аватара.

Для тестов есть `examples`:

- `cargo run --release --package avatar-plugin --example wayland_keyboard_capture --features wayland`: проверка захвата клавиш.

> [!warn] Для wayland читает напрямую с утройств.

## Plugin Settings

### Avatar Structure

Для загрузки аватар используется `loader.rs`.

```
bongo_cat/
├── face/
│   ├── config.json       # Конфигурация выражений лица
│   ├── 0.png            # Нейтральное лицо
│   ├── 1.png            # Счастливое лицо
│   ├── 2.png            # Грустное лицо
│   └── 3.png            # Удивленное лицо
│
└── mode/
    ├── config.json       # Список доступных режимов
    │
    ├── keyboard/         # Режим "Keyboard"
    │   ├── config.json
    │   ├── bg.png
    │   ├── catbg.png
    │   ├── keyboard/     # Клавиши
    │   │   ├── 0.png
    │   │   ├── 1.png
    │   │   └── ...
    │   ├── lefthand/     # Левая рука
    │   │   ├── leftup.png
    │   │   ├── 0.png
    │   │   └── ...
    │   └── righthand/    # Правая рука
    │       ├── rightup.png
    │       ├── 0.png
    │       └── ...
    │
    └── standard/         # Другие режимы
        └── ...
```

## Events and Reactions

## Implementation

### Deformation (moc3 file, live2d cubism)

Live2D Cubims SDK - проприетарная либа, и референс-проекте использует ее, под rust нету порта.
`.moc3` - скомпилированный файл модели Live2D Cubism 3. Банрный формат, содержит mesh данные (deformation), параметры, деформеры, части модели.
В reference-проекте загружается через sdk, прогидывает параметры (input) и отрисовыает через нее.

> [!note] сейчас я пишу для голого png.
> треубется добавить физику и деформацию

Для динамической руки нужна *rigid deformation*, нативно это можно сделать через `atan2` для вычисления угла поворота, scaling точка крепления на аватар.

### Windows

- Windows Hooks API

### X11

- XInput2 и XRecord

### Wayland

Перехват input реализован на основе evdev, то есть прямого доступк к устройствам.
Далее планируется переход на udev и polkit.

Сейчас сканирует все клавиатурные устройства в `/dev/input/event*`, определяет по клавишам `A`, `Z`, `Enter`.
Обработка нажатий работает через Non-Blocking Polling. Для каждого события вызывается callback.

> [!note] Повышенные привилегии для работы
> сейчас требуются права для перехвата: `sudo usermod -a -G input $USER ; newgrp input`
> либо запускать от root

Работает в одном потокe c неблокирующим вводом/выводом. Для каждого дескриптора устанавливается `O_NONBLOCK`.
`InputCapture:pool` вызывается каждый кадр из `video_tick`, опрашивает все устройства, вызывает `device.fetch_events()`.

```rust
let fd = device.as_raw_fd();
let flags = libc::fcntl(fd, libc::F_GETFL);
libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
```

## Examples

- [[./examples/wayland_keyboard_capture.rs]] - демпонстрация захвата input за wayland
- [[./examples/avatar_render.rs]] - демпонстрация рендера аватара c деформацией и анимациями

## References

- [Input capture; MacOS, Windows, x11 / rdev](https://github.com/Narsil/rdev)
- [Input capture; MacOS, Windows, x11 / device_query](https://github.com/ostrosco/device_query)
- [Bongo.cat](https://github.com/Externalizable/bongo.cat/blob/master)
- [Bongobs-Cat-Plugin](https://github.com/a1928370421/Bongobs-Cat-Plugin)
- [evdev documentation](https://docs.rs/evdev/)
- [bevy_spritesheet_animation](https://docs.rs/bevy_spritesheet_animation/)
- [skeletal_animation](https://github.com/PistonDevelopers/skeletal_animation)
- [BongoCat / Desktop App - Tauri and Live2D.js](https://github.com/ayangweb/BongoCat/tree/master)
- [Awesome Bongo Cats](https://github.com/g0l4/BongoCat-Models)
- [wayland bongocat widget](https://github.com/saatvik333/wayland-bongocat)

## Bongobs-Cat-Plugin

Использует Live2D Cubism SDK - проприетарная либа, (`cubism-rs`, `spine-rs`, `inochi2d`). OpenGL. Mesh Deformation через контрольные точки.
Нужена Mesh деформация.

```cpp
// Hook.cpp - под windows

void Hook::Start() {
    th = new std::thread(&Hook::Run, this);
    th->detach();  // ← Detached thread!
}

void Hook::Run() {
    // Зарегистрировать hooks
    // Message loop
    while ((bRet = GetMessage(&msg, 0, 0, 0)) != 0) {
        if (isExist) break;  // simple flag
        DispatchMessage(&msg);
    }
}

// Callback вызывается из Windows message loop
LRESULT Hook::KeyboardHookProc(...) {
    eventManager->KeyEventDown(key);  // update state
    return CallNextHookEx(...);
}
```

1. **Detached thread** - не нужно join
2. **Message loop** - естественно завершается
3. **Простое состояние** - массив bool для клавиш
4. **Polling из модели** - `GetKeySignal(key)`

## License

GPL-3.0 (as used in rust-obs-plugins)
