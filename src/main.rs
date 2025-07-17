use phex::App;

use winit::event_loop::{EventLoop, ControlFlow};

fn main() {
    env_logger::init();

    let mut event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
