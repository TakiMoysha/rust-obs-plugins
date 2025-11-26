#

## Guidelines

- use `obs_wrapper::log` for logging, example: `obs_wrapper::log::info!("Audio level: {}", self.audio_level);`

## Reference

Как образец используется проект на C, находится в директории references.
Он не входит в git репозиторий и загружается отдельно.

### Обзор технологических решений

- для манипулированием аватара используется Live2D Cubism SDK.

#### Live2D Cubism SDK

Каждый кадр параметры в `Model.cpp` обновляются  в методе `model::Update`

1. Motion (Animation):
    SDK Class: `CubismMotionManager`
    Functionality: Plays predefined .motion3.json files.
    Usage: `_motionManager->UpdateMotion(_model, deltaTimeSeconds)` applies the base animation to the model's parameters.

2. Physics (Secondary Motion):
    SDK Class: CubismPhysics
    Functionality: Calculates physics for hair, clothes, etc., based on the current model state.
    Usage: `_physics->Evaluate(_model, deltaTimeSeconds)` adds physics effects on top of the animation.

3. Pose (Part Switching):
    SDK Class: CubismPose
    Functionality: Manages part opacity to switch parts (e.g., changing arms).
    Usage: `_pose->UpdateParameters(_model, deltaTimeSeconds)`.

4. Procedural Animation:
    Eye Blink: CubismEyeBlink automatically handles blinking parameters.
    Breath: CubismBreath applies a sine wave to specific parameters (like ParamBreath, ParamAngleX) to simulate breathing.

5. Manual Input (Mouse/Tracking):
    Mechanism: Directly modifying parameter values using `_model->AddParameterValue(id, value)`.
    Reference Logic:
    Head Tracking: Maps mouse position to `ParamAngleX`, `ParamAngleY`, `ParamAngleZ`, `ParamBodyAngleX`, `ParamEyeBallX`, `ParamEyeBallY`.
    Mouse Interaction: Maps mouse position to custom parameters `ParamMouseX`, `ParamMouseY`.
    Clicks: Maps mouse clicks to `ParamMouseLeftDown`, `ParamMouseRightDown`.
