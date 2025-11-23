# Fork repository

- init submodules (obs for obs-sys)
- update obs `git submodule update --remote`
- update rust-bindings

- **`module`**: defining and registering OBS modules (plugins).
- **`source`**: traits and types for creating sources, filters, and transitions.
- **`properties`**: API for defining user-configurable properties for sources.
- **`data`**: wrapper around `obs_data_t` for handling settings and configuration data.
- **`string`**: utilities for handling OBS-specific strings (`ObsString`).
- **`log`**: logging utilities to print to the OBS log.

> [!note] If you create separate threads, added stop signal in `unload` method.

| Trait                 | Description                                   | Builder Method             |
| :-------------------- | :-------------------------------------------- | :------------------------- |
| `GetNameSource`       | Returns the display name of the source.       | `.enable_get_name()`       |
| `GetWidthSource`      | Returns the width of the source.              | `.enable_get_width()`      |
| `GetHeightSource`     | Returns the height of the source.             | `.enable_get_height()`     |
| `VideoRenderSource`   | Handles video rendering.                      | `.enable_video_render()`   |
| `AudioRenderSource`   | Handles audio rendering.                      | `.enable_audio_render()`   |
| `UpdateSource`        | Called when settings are updated.             | `.enable_update()`         |
| `GetPropertiesSource` | Defines properties (settings) for the source. | `.enable_get_properties()` |
| `GetDefaultsSource`   | Sets default values for settings.             | `.enable_get_defaults()`   |
| `VideoTickSource`     | Called every video frame.                     | `.enable_video_tick()`     |
| `ActivateSource`      | Called when the source becomes active.        | `.enable_activate()`       |
| `DeactivateSource`    | Called when the source becomes inactive.      | `.enable_deactivate()`     |
| `MouseClickSource`    | Handles mouse clicks.                         | `.enable_mouse_click()`    |
| `MouseMoveSource`     | Handles mouse movement.                       | `.enable_mouse_move()`     |
| `MouseWheelSource`    | Handles mouse wheel events.                   | `.enable_mouse_wheel()`    |
| `KeyClickSource`      | Handles keyboard events.                      | `.enable_key_click()`      |
| `FocusSource`         | Handles focus events.                         | `.enable_focus()`          |
| `FilterVideoSource`   | For filters: process video data.              | `.enable_filter_video()`   |
| `FilterAudioSource`   | For filters: process audio data.              | `.enable_filter_audio()`   |

### Property Types (`GetPropertiesSource`)

- **`NumberProp`**: Integer or Float. Can be configured as a slider.
- **`BoolProp`**: Checkbox.
- **`TextProp`**: Text input (Default, Password, Multiline).
- **`ColorProp`**: Color picker.
- **`PathProp`**: File or directory picker.
- **`ListProp`**: Dropdown list (via `props.add_list`).
- **`FontProp`**: Font selection.
- **`EditableListProp`**: Editable list of strings or files.

---

# Rust OBS Wrapper

[![Build Status](https://travis-ci.org/bennetthardwick/rust-obs-plugins.svg?branch=master)](https://travis-ci.org/bennetthardwick/rust-obs-plugins)
[![Wrapper Docs](https://docs.rs/obs-wrapper/badge.svg)](https://docs.rs/obs-wrapper)

A safe wrapper around the OBS API, useful for creating OBS sources, filters and effects. The wrapper is quite incomplete and will most likely see dramatic API changes in the future.

This repo also includes plugins creating using the wrapper in the `/plugins` folder.

## Plugins

| Folder                   | Description                                                      |
| ------------------------ | ---------------------------------------------------------------- |
| /scroll-focus-filter     | an OBS filter that will zoom into the currently focused X window |
| /rnnoise-denoiser-filter | an OBS filter for removing background noise from your Mic        |

## Usage

In your `Cargo.toml` file add the following section, substituting `<module-name>` for the name of
the module:

```toml
[dependencies]
obs-wrapper = "0.4"

[lib]
name = "<module-name>"
crate-type = ["cdylib"]
```

The process for creating a plugin is:

1. Create a struct that implements Module
1. Create a struct that will store the plugin state
1. Implement the required traits for the module
1. Enable the traits which have been enabled in the module `load` method

```rust
use obs_wrapper::{
    // Everything required for modules
    prelude::*,
    // Everything required for creating a source
    source::*,
    // Macro for registering modules
    obs_register_module,
    // Macro for creating strings
    obs_string,
};

// The module that will handle creating the source.
struct TestModule {
    context: ModuleRef
}

// The source that will be shown inside OBS.
struct TestSource;

// Implement the Sourceable trait for TestSource, this is required for each source.
// It allows you to specify the source ID and type.
impl Sourceable for TestSource {
    fn get_id() -> ObsString {
        obs_string!("test_source")
    }

    fn get_type() -> SourceType {
        SourceType::Filter
    }

    fn create(create: &mut CreatableSourceContext<Self>, source: SourceContext) -> Self {
        Self
    }
}

// Allow OBS to show a name for the source
impl GetNameSource for TestSource {
    fn get_name() -> ObsString {
        obs_string!("Test Source")
    }
}

// Implement the Module trait for TestModule. This will handle the creation of the source and
// has some methods for telling OBS a bit about itself.
pub trait Module {
    fn new(ctx: ModuleRef) -> Self;
    fn get_ctx(&self) -> &ModuleRef;
    fn unload(&mut self) {}
    fn post_load(&mut self) {}
    // about plugin
    fn description() -> ObsString;
    fn name() -> ObsString;
    fn author() -> ObsString;

    // Load the module - create all sources, returning true if all went well.
    fn load(&mut self, load_context: &mut LoadContext) -> bool {
        // Create the source
        let source = load_context
            .create_source_builder::<TestSource>()
            // Since GetNameSource is implemented, this method needs to be called to
            // enable it.
            .enable_get_name()
            .build();

        // Tell OBS about the source so that it will show it.
        load_context.register_source(source);

        // Nothing could have gone wrong, so return true.
        true
    }
}

obs_register_module!(TestModule);
```

### Installation

1. Run `cargo build --release`
2. Copy `/target/release/<module-name>.so` to your OBS plugins folder (`/usr/lib/obs-plugins/`)
3. The plugin should be available for use from inside OBS

## License

Like [obs-studio](https://github.com/obsproject/obs-studio), `obs-wrapper` is licensed under GNU General Public License v2.0.

See [LICENSE](./LICENSE) for details.
