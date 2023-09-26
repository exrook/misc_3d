use bytemuck::{Pod, Zeroable};
use shader::{prelude::*, Loadable, PipelineCache};
use std::sync::{Arc, Mutex};
use wgpu_shader_boilerplate as shader;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window as WinitWindow,
};

pub struct WindowData {
    read_bind_group: wgpu::BindGroup,
}

shader::shader_file!(SHADER "window.wgsl");

impl WindowData {
    pub fn new(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> Self {
        let blank_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("display"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        Self::with_texture(device, &blank_texture)
    }
    pub fn with_texture(device: &wgpu::Device, texture: &wgpu::Texture) -> Self {
        let texture_view = texture.create_view(&Default::default());
        let make_descriptor = |layout: &_| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                }],
            })
        };

        let read_bind_group =
            Self::with_layout::<Read, _, _>(device, |layout| make_descriptor(layout));
        Self { read_bind_group }
    }
    pub fn replace_texture(&mut self, device: &wgpu::Device, new_texture: &wgpu::Texture) {
        *self = Self::with_texture(device, new_texture);
    }
}
impl BindLayout for WindowData {}
impl<T: AccessType> BindLayoutFor<T> for WindowData {
    fn with_layout_impl<F: FnOnce(&wgpu::BindGroupLayoutDescriptor) -> U, U>(f: F) -> U {
        f(&wgpu::BindGroupLayoutDescriptor {
            label: Some("window render group"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        })
    }
}

impl Bindable for WindowData {}
impl BindableFor<Read> for WindowData {
    fn group_impl(&self) -> &wgpu::BindGroup {
        &self.read_bind_group
    }
}

pub struct Pipelines {
    draw_window: wgpu::RenderPipeline,
}

impl Loadable<(&wgpu::PipelineLayout, wgpu::TextureFormat)> for Pipelines {
    fn load(
        device: &wgpu::Device,
        shader: wgpu::ShaderModule,
        (pipeline_layout, swapchain_format): (&wgpu::PipelineLayout, wgpu::TextureFormat),
    ) -> Self {
        Self {
            draw_window: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(swapchain_format.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            }),
        }
    }
}

pub struct WindowResources {
    pipeline_layout: wgpu::PipelineLayout,
    pipeline_cache: PipelineCache<Pipelines>,

    pub swapchain_format: wgpu::TextureFormat,

    config: wgpu::SurfaceConfiguration,

    pub mode: i32,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct DispatchData {
    mode: i32,
}

impl WindowResources {
    pub fn setup(
        size: winit::dpi::PhysicalSize<u32>,
        surface: &wgpu::Surface,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> Self {
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let pipeline_layout = WindowData::with_layout::<Read, _, _>(device, |window_layout| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[window_layout],
                push_constant_ranges: &[wgpu::PushConstantRange {
                    stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    range: 0..(std::mem::size_of::<DispatchData>() as u32),
                }],
            })
        });

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        let pipeline_cache = SHADER.cache();

        surface.configure(&device, &config);

        Self {
            pipeline_layout,
            pipeline_cache,
            swapchain_format,

            config,

            mode: 0,
        }
    }
    pub fn reconfigure(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        surface: &wgpu::Surface,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        // Reconfigure the surface with the new size
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        surface.configure(&device, &self.config);
        // On macos the window needs to be redrawn manually after resizing
    }
    pub fn make_renderpass<'s: 'pass, 'pass>(
        &'s self,
        encoder: &'pass mut wgpu::CommandEncoder,
        surface_view: &'s wgpu::TextureView,
    ) -> wgpu::RenderPass<'pass> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        data: &WindowData,
        surface_view: &wgpu::TextureView,
    ) {
        let Pipelines { draw_window, .. } = &*self
            .pipeline_cache
            .load_auto(device, (&self.pipeline_layout, self.swapchain_format));

        let mut rpass = self.make_renderpass(encoder, surface_view);

        rpass.set_pipeline(draw_window);
        rpass.set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::bytes_of(&DispatchData { mode: self.mode }),
        );
        rpass.set_bind_group(0, data.group::<Read>(), &[]);
        rpass.draw(0..4, 0..1);
    }
}

pub struct Window {
    pub data: WindowData,
    pub resources: WindowResources,
    pub surface: wgpu::Surface,
    pub window: WinitWindow,
}

impl Window {
    pub fn create_with<
        F: Fn(&wgpu::Instance, &wgpu::Surface) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue),
    >(
        event_loop: &mut EventLoop<()>,
        f: F,
    ) -> (
        Self,
        (wgpu::Instance, wgpu::Adapter, wgpu::Device, wgpu::Queue),
    ) {
        let window = WinitWindow::new(event_loop).unwrap();

        let size = window.inner_size();

        let instance = wgpu::Instance::default();

        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        let (adapter, device, queue) = f(&instance, &surface);

        let resources = WindowResources::setup(size, &surface, &adapter, &device, &queue);
        let data = WindowData::new(&device, size);

        (
            Self {
                data,
                resources,
                surface,
                window,
            },
            (instance, adapter, device, queue),
        )
    }
    pub fn tick(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resolution: &Mutex<(u32, u32)>,
        output: &Mutex<Option<Arc<wgpu::Texture>>>,
        event: Event<()>,
    ) -> Option<ControlFlow> {
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                if size.width > 0 && size.height > 0 {
                    *resolution.lock().unwrap() = (size.width, size.height);
                    self.resources
                        .reconfigure(size, &self.surface, &device, &queue);
                    self.window.request_redraw();
                }
            }
            Event::MainEventsCleared | Event::RedrawRequested(_) => {
                match self.surface.get_current_texture() {
                    Ok(frame) => {
                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        if let Some(new_texture) = output.lock().unwrap().take() {
                            self.data.replace_texture(&device, &*new_texture);
                        }

                        let mut encoder = device.create_command_encoder(&Default::default());
                        self.resources.draw(device, &mut encoder, &self.data, &view);
                        queue.submit(Some(encoder.finish()));

                        frame.present();
                    }
                    Err(e) => {
                        println!("Failed to acquire next swap chain texture {}", e);
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => return Some(ControlFlow::Exit),
            _ => {}
        }
        None
    }
}
