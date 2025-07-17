use std::sync::Arc;

use wgpu::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop, ActiveEventLoop},
    window::{Window, WindowId},
};


fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}


#[derive(Default)]
struct App {
    render_context: Option<RenderContext>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("resumed");

        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        let mut render_context = RenderContext::new(Arc::new(window));
        render_context.configure_surface();
        self.render_context = Some(render_context);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(render_context) = self.render_context.as_mut() else {
            return;
        };

        if window_id != render_context.window.id() {
            return;
        }

        log::info!("window_event: {:?}", event);

        match event {
            WindowEvent::RedrawRequested => {
                render_context.render();
                render_context.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

struct RenderContext {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    format: TextureFormat,
    render_pipeline: RenderPipeline,
    objects: Vec<Object>,
    window: Arc<Window>,
}

impl RenderContext {
    fn new(window: Arc<Window>) -> Self {
        use pollster::FutureExt;

        let instance = Instance::new(&InstanceDescriptor::default());
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .block_on()
            .unwrap();
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .block_on()
            .unwrap();

        let format = surface.get_capabilities(&adapter).formats[0];

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: None,
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let num_objects = 100;
        let mut objects = Vec::with_capacity(num_objects);

        let base_uniform_buffer_size = std::mem::size_of::<Base>().max(32);
        let extra_uniform_buffer_size = std::mem::size_of::<Extra>().max(32);

        for i in 0..num_objects {
            let base_uniform_buffer = device.create_buffer(&BufferDescriptor {
                label: Some(&format!("base uniform buffer for obj {}", i)),
                size: base_uniform_buffer_size as BufferAddress,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let base = Base {
                color: [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0],
                offset: [rand(-0.9, 0.9), rand(-0.9, 0.9)],
            };
            queue.write_buffer(&base_uniform_buffer, 0, bytemuck::cast_slice(&[base]));

            let extra_uniform_buffer = device.create_buffer(&BufferDescriptor {
                label: Some(&format!("extra uniform buffer for obj {}", i)),
                size: extra_uniform_buffer_size as BufferAddress,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some(&format!("bind group for obj {}", i)),
                layout: &render_pipeline.get_bind_group_layout(0),
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: base_uniform_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: extra_uniform_buffer.as_entire_binding(),
                    },
                ],
            });

            objects.push(Object {
                scale: rand(0.2, 0.5),
                extra_uniform_buffer,
                bind_group,
            });
        }

        Self {
            device,
            queue,
            surface,
            format,
            render_pipeline,
            objects,
            window,
        }
    }

    fn configure_surface(&mut self) {
        let size = self.window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: self.format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![self.format.add_srgb_suffix()],
        };
        self.surface.configure(&self.device, &config);
    }

    fn render(&mut self) {
        let frame = self.surface.get_current_texture().unwrap();
        let view = frame.texture.create_view(&TextureViewDescriptor {
            format: Some(self.format.add_srgb_suffix()),
            ..Default::default()
        });


        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.019,
                        g: 0.019,
                        b: 0.019,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);


        let size = self.window.inner_size();
        let aspect = size.width / size.height;

        for object in &mut self.objects {
            let uniform = Extra {
                scale: [object.scale / aspect as f32, object.scale],
            };
            self.queue
                .write_buffer(&object.extra_uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
            render_pass.set_bind_group(0, &object.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        drop(render_pass);

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        frame.present();
    }
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct Base {
    color: [f32; 4],
    offset: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct Extra {
    scale: [f32; 2],
}

struct Object {
    scale: f32,
    extra_uniform_buffer: Buffer,
    bind_group: BindGroup,
}

fn rand(min: f32, max: f32) -> f32 {
    min + rand::random_range(0.0..1.0) * (max - min)
}
