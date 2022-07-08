use winit::window::Window;
use wgpu::*;
use wgpu::util::*;

mod dither;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PushConstants {
    dimensions: [f32; 2],
    field_of_view: f32,
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, 1.0] },
    Vertex { position: [-1.0, -1.0] },
    Vertex { position: [1.0, -1.0] },
    Vertex { position: [1.0, 1.0] },
    Vertex { position: [-1.0, 1.0] },
];

pub struct Graphics  {
    instance: Instance,
    window: Window,
    surface: Surface,
    adapter: Adapter,
    surface_format: TextureFormat,
    device: Device,
    queue: Queue,
    shader: ShaderModule,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    surface_stale: bool,
    desired_size: winit::dpi::PhysicalSize<u32>,
    dither_bind_group: BindGroup,
}

impl Graphics {
    pub async fn setup(window: Window) -> Self {
        // TODO: I don't think there's any reason we can't support ALL, but with ALL it defaults to OpenGL
        //   on my machine for some resason. We should support ALL, so long as the PRIMARY backends
        //   are used by default.
        let instance = Instance::new(Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance.request_adapter(
            &RequestAdapterOptionsBase {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface)
            }
        ).await.expect("Failed to get wgpu adapter.");
        let format = surface.get_supported_formats(&adapter)[0];
        let (device, queue) = adapter.request_device(&DeviceDescriptor {
            label: None,
            features: Features::PUSH_CONSTANTS,
            limits: Limits {
                max_push_constant_size: 128,
                .. Limits::default()
            }
        }, None).await.expect("Failed to get wgpu device.");
        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
        let dither_texture = device.create_texture_with_data(
            &queue,
            &TextureDescriptor {
                size: Extent3d {
                    width: u8::MAX as u32,
                    height: u8::MAX as u32,
                    depth_or_array_layers: 1
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                label: Some("dither_texture")
            },
            bytemuck::cast_slice(&*dither::bayer_texture())
        );
        let dither_texture_view = dither_texture.create_view(&TextureViewDescriptor::default());
        let dither_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                }
            ],
            label: Some("dither_bind_group_layout")
        });
        let dither_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &dither_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&dither_texture_view)
                }
             ],
            label: Some("dither_bind_group")
         });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&dither_bind_group_layout],
                push_constant_ranges: &[
                    PushConstantRange {
                        stages: ShaderStages::FRAGMENT,
                        range: 0..12,
                    }
                ]
            })),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x3]
                    }
                ]
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[
                    Some(ColorTargetState {
                        format,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL
                    })
                ]
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            },
            multiview: None
        });
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(VERTICES),
            usage: BufferUsages::VERTEX
        });
        let desired_size = window.inner_size();
        Self {
            instance,
            window,
            surface,
            surface_format: format,
            adapter,
            device,
            queue,
            shader,
            pipeline,
            vertex_buffer,
            surface_stale: true,
            desired_size,
            dither_bind_group
        }
    }

    fn reconfigure_surface(&self, size: winit::dpi::PhysicalSize<u32>) {
        log::debug!("Reconfiguring wgpu surface.");
        self.surface.configure(&self.device, &SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Mailbox
        });
    }

    fn reconfigure_surface_if_stale(&mut self) {
        log::info!("reconfigure");
        if self.surface_stale {
            self.reconfigure_surface(self.desired_size);
            self.surface_stale = false;
        }
    }

    pub fn window_resized(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.desired_size = new_size;
        self.surface_stale = true;
    }

    pub fn draw(&mut self) {
        log::info!("redraw");
        self.reconfigure_surface_if_stale();
        let frame = self.surface.get_current_texture().expect("Failed to get surface texture");
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0
                            }),
                            store: true
                        }
                    })
                ],
                depth_stencil_attachment: None
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::bytes_of(&PushConstants {
                dimensions: [self.desired_size.width as f32, self.desired_size.height as f32],
                field_of_view: std::f32::consts::PI,
            }));
            render_pass.set_bind_group(0, &self.dither_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..VERTICES.len() as u32, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
