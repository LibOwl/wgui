use std::{iter, mem};

use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, BindGroup, BufferUsages, Sampler, Texture, TextureView};
use winit::{window::Window, event::WindowEvent};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct General {
    resolution: [u32; 2],
    resized: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    id: u32,
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        // vertex a
        position: [-0.5, -0.5],
        id: 1,
    },
    Vertex {
        // vertex b
        position: [0.5, -0.5],
        id: 1,
    },
    Vertex {
        // vertex d
        position: [-0.5, 0.5],
        id: 1,
    },
    Vertex {
        // vertex c
        position: [0.5, 0.5],
        id: 1,
    },
];
const INDICES: &[u16] = &[
    0, 1, 2, 
    3, 1, 2, 
];

pub(super) struct State<'window> {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'window>,
    pub(super) size: winit::dpi::PhysicalSize<u32>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    gen_bind_group: BindGroup,
    id_bind_group: BindGroup,
    // widgets_bind_group: BindGroup,
    pipeline: wgpu::RenderPipeline,

}

impl<'window> State<'window> {
    pub(super) async fn new(window: &'window Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter:false
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode:surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1
        };
        surface.configure(&device, &config);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });


        let gen_info = General {
            resolution: [size.width, size.height],
            resized: [1, 0],
        };
        let gen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gen Uniform Buffer"),
            contents: bytemuck::cast_slice(&[gen_info]),
            usage: BufferUsages::UNIFORM,
        });
        let gen_bind_group_layout = 
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("gen_bind_group_layout"),
        });
        let gen_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &gen_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(gen_buffer.as_entire_buffer_binding()),
                    },
                ],
                label: Some("gen_bind_group"),
            }
        );


        let id_info = (0..size.width * size.height).map(|_| 0u32).collect::<Vec<u32>>();
        let id_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ID Storage Buffer"),
            contents: bytemuck::cast_slice(id_info.as_slice()),
            usage: BufferUsages::STORAGE,
        });
        let id_bind_group_layout = 
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("id_bind_group_layout"),
        });
        let id_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &id_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(id_buffer.as_entire_buffer_binding()),
                    },
                ],
                label: Some("id_bind_group"),
            }
        );


// TEXTURE
        // let (_id_texture, id_view, id_sampler) = create_texture(
        //     &device,
        //     &config,
        //     "ID Texture"
        // );
        // let id_bind_group_layout =
        // device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //     entries: &[
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::StorageTexture {
        //                 access: wgpu::StorageTextureAccess::WriteOnly,
        //                 format: wgpu::TextureFormat::R32Uint,
        //                 view_dimension: wgpu::TextureViewDimension::D2,
        //             },
        //             count: None,
        //         },
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 1,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
                    
                    
        //             ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //             count: None,
        //         },
        //     ],
        //     label: Some("id_bind_group_layout"),
        // });
        // let id_bind_group = device.create_bind_group(
        //     &wgpu::BindGroupDescriptor {
        //         layout: &id_bind_group_layout,
        //         entries: &[
        //             wgpu::BindGroupEntry {
        //                 binding: 0,
        //                 resource: wgpu::BindingResource::TextureView(&id_view),
        //             },
        //             wgpu::BindGroupEntry {
        //                 binding: 1,
        //                 resource: wgpu::BindingResource::Sampler(&id_sampler),
        //             }
        //         ],
        //         label: Some("id_bind_group"),
        //     }
        // );


        // let widgets_info = General {
        //     resolution: [size.width, size.height]
        // };
        // let widgets_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Widgets Buffer"),
        //     contents: bytemuck::cast_slice(&[widgets_info]),
        //     usage: BufferUsages::UNIFORM,
        // });
        // let widgets_bind_group_layout = 
        // device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //     entries: &[
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Buffer {
        //                 ty: wgpu::BufferBindingType::Uniform,
        //                 has_dynamic_offset: false,
        //                 min_binding_size: None,
        //             },
        //             count: None,
        //         },
        //     ],
        //     label: Some("widgets_bind_group_layout"),
        // });
        // let widgets_bind_group = device.create_bind_group(
        //     &wgpu::BindGroupDescriptor {
        //         layout: &widgets_bind_group_layout,
        //         entries: &[
        //             wgpu::BindGroupEntry {
        //                 binding: 0,
        //                 resource: wgpu::BindingResource::Buffer(widgets_buffer.as_entire_buffer_binding()),
        //             },
        //         ],
        //         label: Some("widgets_bind_group"),
        //     }
        // );


        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &gen_bind_group_layout,
                // &widgets_bind_group_layout,
                &id_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            instance,
            surface,
            size,
            device,
            queue,
            config,

            vertex_buffer,
            index_buffer,
            gen_bind_group,
            id_bind_group,
            // widgets_bind_group,
            pipeline,
        }
    }

    pub(super) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // Recreate Surface
        log::info!("Surface resize {new_size:?}");
        self.instance.poll_all(true);
        self.size = new_size;
        self.config.width = new_size.width.max(1);
        self.config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.config);

        // Recreate Id Buffer
        let id_info = (0..self.size.width * self.size.height).map(|_| 0u32).collect::<Vec<u32>>();
        let id_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ID Storage Buffer"),
            contents: bytemuck::cast_slice(id_info.as_slice()),
            usage: BufferUsages::STORAGE,
        });
        let id_bind_group_layout = 
        self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("id_bind_group_layout"),
        });
        self.id_bind_group = self.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &id_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(id_buffer.as_entire_buffer_binding()),
                    },
                ],
                label: Some("id_bind_group"),
            }
        );
    }

    #[allow(unused)]
    pub(super) fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub(super) fn update(&mut self) {
        // self.queue.write_buffer(&self.gen_buffer, 0, bytemuck::cast_slice(&[self.gen_info]));

    }

    pub(super) fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        //let output = self.surface.get_current_frame()?.output;
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2,
                            g: 0.247,
                            b: 0.314,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.gen_bind_group, &[]);
            render_pass.set_bind_group(1, &self.id_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[allow(unused)]
fn create_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, label: &str) -> (Texture, TextureView, Sampler) {
    let size = wgpu::Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let desc = wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(
        &wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }
    );

    ( texture, view, sampler )
}