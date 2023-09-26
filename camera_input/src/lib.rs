use camera::Control;
use winit_input_helper::WinitInputHelper;

pub fn process_input(input: &WinitInputHelper, last_control: &Control) -> Control {
    use winit::event::VirtualKeyCode as Key;
    let mut wish_dir = glam::Vec3::ZERO;
    let mut val = 1.0;
    let mut control_input = Control::zero();
    if input.held_shift() {
        val *= 10.0;
    }
    if input.key_held(Key::W) {
        wish_dir.x += val;
    }
    if input.key_held(Key::S) {
        wish_dir.x += -val;
    }
    if input.key_held(Key::A) {
        wish_dir.y += -val;
    }
    if input.key_held(Key::D) {
        wish_dir.y += val;
    }
    if input.key_held(Key::Q) {
        wish_dir.z += val;
    }
    if input.key_held(Key::E) {
        wish_dir.z += -val;
    }
    if input.mouse_held(0) {
        let sens = 0.01;
        let movement = input.mouse_diff();
        control_input.rotation.x += movement.0 * sens * 0.5;
        control_input.rotation.y = (control_input.rotation.y + movement.1 * sens).clamp(-1.0, 1.0);
    }
    control_input.position += (last_control.rot() * wish_dir).as_dvec3() * 0.01;
    return control_input;
}
