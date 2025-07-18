use std::{f32::consts::PI, sync::Arc};

use wgpu::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
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
    num_instances: u32,
    base_storage_buffer: Buffer,
    extra_storage_buffer: Buffer,
    extra_storage_values: Vec<Extra>,
    scaling_units: Vec<f32>,
    bind_group: BindGroup,
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

        let num_instances = 100;

        let base_unit_size = std::mem::size_of::<Base>();
        let base_storage_buffer_size = base_unit_size * num_instances as usize;
        let base_storage_buffer = device.create_buffer(&BufferDescriptor {
            label: Some(&format!("base uniform buffer for obj")),
            size: base_storage_buffer_size as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let extra_unit_size = std::mem::size_of::<Extra>();
        let extra_storage_buffer_size = extra_unit_size * num_instances as usize;
        let extra_storage_buffer = device.create_buffer(&BufferDescriptor {
            label: Some(&format!("extra uniform buffer for obj")),
            size: extra_storage_buffer_size as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut bases = Vec::with_capacity(num_instances as usize);
        let mut scaling_units = Vec::with_capacity(num_instances as usize);
        for i in 0..num_instances as usize {
            let base = Base {
                color: [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0],
                offset: [rand(-0.9, 0.9), rand(-0.9, 0.9)],
            };
            bases.push(base);
            scaling_units.push(rand(0.2, 0.5));
        }
        queue.write_buffer(&base_storage_buffer, 0, bytemuck::cast_slice(&bases));

        let extra_storage_values = Vec::with_capacity(num_instances as usize);

        let vertices = create_circle_vertices(0.5, 24, 0.25, 0.0, PI * 2.0);
        let vertex_storage_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("vertex storage buffer"),
            size: (std::mem::size_of::<Vertex>() * vertices.len()) as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_storage_buffer, 0, bytemuck::cast_slice(&vertices));

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("bind group")),
            layout: &render_pipeline.get_bind_group_layout(0),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: base_storage_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: extra_storage_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: vertex_storage_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            device,
            queue,
            surface,
            format,
            render_pipeline,
            num_instances,
            base_storage_buffer,
            extra_storage_buffer,
            extra_storage_values,
            scaling_units,
            bind_group,
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
        let extras: Vec<_> = self
            .scaling_units
            .iter()
            .map(|scale| Extra {
                scale: [scale / aspect as f32, *scale],
            })
            .collect();
        self.queue
            .write_buffer(&self.extra_storage_buffer, 0, bytemuck::cast_slice(&extras));
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..24 * 3 * 2, 0..self.num_instances);

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

fn rand(min: f32, max: f32) -> f32 {
    min + rand::random_range(0.0..1.0) * (max - min)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    fn new(x: f32, y: f32) -> Self {
        Self { position: [x, y] }
    }
}

fn create_circle_vertices(
    radius: f32,
    num_subdivisions: usize,
    inner_radius: f32,
    start_angle: f32,
    end_angle: f32,
) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    for i in 0..num_subdivisions {
        let angle1 = start_angle + i as f32 * (end_angle - start_angle) / num_subdivisions as f32;
        let angle2 =
            start_angle + (i as f32 + 1.0) * (end_angle - start_angle) / num_subdivisions as f32;

        let c1 = angle1.cos();
        let s1 = angle1.sin();
        let c2 = angle2.cos();
        let s2 = angle2.sin();

        vertices.push(Vertex::new(c1 * radius, s1 * radius));
        vertices.push(Vertex::new(c2 * radius, s2 * radius));
        vertices.push(Vertex::new(c1 * inner_radius, s1 * inner_radius));
        vertices.push(Vertex::new(c1 * inner_radius, s1 * inner_radius));
        vertices.push(Vertex::new(c2 * radius, s2 * radius));
        vertices.push(Vertex::new(c2 * inner_radius, s2 * inner_radius));
    }

    vertices
}
