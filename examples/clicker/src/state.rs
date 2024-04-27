use std::{cell::OnceCell, iter, mem, sync::Arc};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalPosition, event::{ElementState, MouseButton, Touch, TouchPhase, WindowEvent}, window::Window};

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


#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct General {
    resolution: [u32; 2],
    resized: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(super) struct Widget {
    /// Xmin, Xmax ,Ymin, Ymax
    pub(super) limits: [f32; 4],
    /// The type of widget, with parameters.
    /// Interpreted in the shaders.
    pub(super) ty: [u32; 4],
    // /// How resizing should be handled in the compute shader.
    // /// Interpreted in the shaders.
    // resize_type: u32,
    // /// Info for resizing.
    // /// Interpreted in the shaders.
    // resize_params: [f32; 2],
}

impl Widget {
    /// A new widget.
    /// - `limits` are (Xmin, Xmax ,Ymin, Ymax), are both the quad limits and the values used to draw in the fragment shader.
    /// - `wt` is the type of widget.
    pub(super) fn new(limits: [f32; 4], wt: WidgetType) -> Self  {
        Self {
            limits,
            ty: [wt.ty(), 0, 0, 0],
        }
    }
}

pub(super) enum WidgetType {
    EllipticButton,
}
impl WidgetType {
    pub(super) fn ty(&self) -> u32 {
        match self {
            Self::EllipticButton => 0,
        }
    }
}

pub(super) struct State<'window> {
    instance: wgpu::Instance,
    pub(super) window: Arc<Window>,
    surface: wgpu::Surface<'window>,
    pub(super) size: winit::dpi::PhysicalSize<u32>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    gen_buffer: wgpu::Buffer,
    gen_bind_group: wgpu::BindGroup,

    id_buffer: wgpu::Buffer,
    id_buffer_len: u64,
    id_max_buffer_len: u64,
    id_bind_group_layout: wgpu::BindGroupLayout,
    id_bind_group: wgpu::BindGroup,

    #[allow(unused)]
    widgets_buffer: wgpu::Buffer,
    #[allow(unused)]
    widgets_bind_group: wgpu::BindGroup,
    
    pipeline: wgpu::RenderPipeline,

    resized: bool,

    cursor_pos: PhysicalPosition<f64>,
}

impl<'window> State<'window> {
    pub(super) async fn new(window: Arc<Window>, widgets: Vec<Widget>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();
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
                    required_limits: wgpu::Limits::downlevel_defaults(),
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


        let mut vertices: Vec<Vertex> = vec![];
        let mut indices: Vec<u16> = vec![];
        for (i, w) in widgets.iter().enumerate() {
            vertices.push(Vertex { position: [w.limits[0], w.limits[2]], id: i as u32 });
            vertices.push(Vertex { position: [w.limits[0], w.limits[3]], id: i as u32 });
            vertices.push(Vertex { position: [w.limits[1], w.limits[3]], id: i as u32 });
            vertices.push(Vertex { position: [w.limits[1], w.limits[2]], id: i as u32 });
            indices.push(4*i as u16);
            indices.push(4*i as u16+1);
            indices.push(4*i as u16+2);
            indices.push(4*i as u16+0);
            indices.push(4*i as u16+2);
            indices.push(4*i as u16+3);
        }


        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices.as_slice()),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices: u32 = indices.len() as u32;
        println!("{}", num_indices);


        let gen_info = General {
            resolution: [size.width, size.height],
            resized: [1, 0],
        };
        let gen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gen Uniform Buffer"),
            contents: bytemuck::cast_slice(&[gen_info]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
        let id_buffer_data: &[u8] = bytemuck::cast_slice(id_info.as_slice());
        let id_buffer_len: u64 = id_buffer_data.len() as u64;
        let id_max_buffer_len: u64 = id_buffer_len;
        let id_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ID Storage Buffer"),
            contents: id_buffer_data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
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


        let widgets_info: Vec<Widget> = widgets;
        let widgets_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Widgets Storage Buffer"),
            contents: bytemuck::cast_slice(widgets_info.as_slice()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let widgets_bind_group_layout = 
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
            label: Some("widgets_bind_group_layout"),
        });
        let widgets_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &widgets_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(widgets_buffer.as_entire_buffer_binding()),
                    },
                ],
                label: Some("widgets_bind_group"),
            }
        );


        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("w__vertex.wgsl").into()),
        });
        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("w__fragment.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &gen_bind_group_layout,
                &widgets_bind_group_layout,
                &id_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
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

        let resized: bool = true;

        window.set_cursor_position(PhysicalPosition::new(500.0f64, 500.0f64)).unwrap();
        let cursor_pos: PhysicalPosition<f64> = PhysicalPosition::new(500.0f64, 500.0f64);

        Self {
            instance,
            window,
            surface,
            size,
            device,
            queue,
            config,

            vertex_buffer,
            index_buffer,
            num_indices,

            gen_buffer,
            gen_bind_group,

            id_buffer,
            id_buffer_len,
            id_max_buffer_len,
            id_bind_group_layout,
            id_bind_group,
            
            widgets_buffer,
            widgets_bind_group,

            pipeline,

            resized,

            cursor_pos,
        }
    }

    pub(super) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // Recreate Surface
        self.instance.poll_all(true);
        self.size = new_size;
        self.config.width = new_size.width.max(1);
        self.config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.config);

        // Modify Gen buffer accordingly
        let gen_info = General {
            resolution: [self.size.width, self.size.height],
            resized: [1, 0]
        };
        self.queue.write_buffer(&self.gen_buffer, 0, bytemuck::cast_slice(&[gen_info]));
        
        // Clear Id Buffer, and resize it if the new one is bigger than the current one
        self.id_buffer_len = 4 * self.size.width as u64 * self.size.height as u64;

        if self.id_buffer_len > self.device.limits().max_buffer_size {
            panic!("The wanted buffer is too large!");
        }

        if self.id_buffer_len > self.id_max_buffer_len {
            let id_info = (0..self.size.width * self.size.height).map(|_| 0u32).collect::<Vec<u32>>();
            let id_buffer_data: &[u8] = bytemuck::cast_slice(id_info.as_slice());

            assert!(self.id_buffer_len == id_buffer_data.len() as u64); // DEBUG
            self.id_max_buffer_len = id_buffer_data.len() as u64;

            self.id_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ID Storage Buffer"),
                contents: id_buffer_data,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            });
            self.id_bind_group = self.device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &self.id_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(self.id_buffer.as_entire_buffer_binding()),
                        },
                    ],
                    label: Some("id_bind_group"),
                }
            );
        }
        else {
            let data = (0..self.id_buffer_len).map(|_| 0u8).collect::<Vec<u8>>();
            self.queue.write_buffer(&self.id_buffer, 0, data.as_slice());
        }

        self.resized = true;
    }

    #[allow(unused)]
    pub(super) fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { device_id, position } => {
                self.cursor_pos = *position;
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.click();
            }
            WindowEvent::Touch(Touch {
                phase: TouchPhase::Started,
                location,
                ..
            })=> {
                self.cursor_pos = *location;

                self.click();
            }
            _ => {  }
        }
        false
    }

    fn click(&mut self) {
        let id_buffer_index = 4 * (self.cursor_pos.y as u64 * self.size.width as u64 + self.cursor_pos.x as u64);

        let mapped_id_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readable ID Buffer"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            encoder.copy_buffer_to_buffer(
                &self.id_buffer, 
                id_buffer_index as u64, 
                &mapped_id_buffer, 
                0, 4
            );
        }
        self.queue.submit(iter::once(encoder.finish()));


        mapped_id_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |result| {
                OnceCell::new().set(result).unwrap();
            });

        self.device.poll(wgpu::Maintain::Wait);

        let slice: &[u8] = &mut mapped_id_buffer.slice(..).get_mapped_range();
        println!("{:?}", bytemuck::cast_slice::<u8, u32>(slice));
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
            render_pass.set_bind_group(1, &self.widgets_bind_group, &[]);
            render_pass.set_bind_group(2, &self.id_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        if self.resized {
            let gen_info = General {
                resolution: [self.size.width, self.size.height],
                resized: [0, 0]
            };
            self.queue.write_buffer(&self.gen_buffer, 0, bytemuck::cast_slice(&[gen_info]));
            self.resized = false;
        }

        Ok(())
    }

    #[allow(unused)]
    pub(super) fn id_buffer_len(&self) -> u64 {
        self.id_buffer_len
    }

    #[allow(unused)]
    pub(super) fn mapped_id_buffer(&mut self) -> (wgpu::Buffer, u32, u32) {
        let mapped_id_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readable ID Buffer"),
            size: self.id_buffer_len,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            encoder.copy_buffer_to_buffer(
                &self.id_buffer, 
                0, 
                &mapped_id_buffer, 
                0, self.id_buffer_len
            );
        }
        self.queue.submit(iter::once(encoder.finish()));


        mapped_id_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |result| {
                OnceCell::new().set(result).unwrap();
            });

        self.device.poll(wgpu::Maintain::Wait);

        (mapped_id_buffer, self.size.width, self.size.height)
    }
}