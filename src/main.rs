mod glyph;

use std::sync::Arc;

use rend3::{graph::RenderGraph, util::output::OutputFrame};
use rend3_routine::pbr::PbrMaterial;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    env_logger::init();

    let el = EventLoop::new();
    let window = {
        WindowBuilder::new()
            .with_title("Rendering playground")
            .build(&el)
            .expect("Could not build window")
    };

    let window_size = window.inner_size();

    let iad = pollster::block_on(rend3::create_iad(None, None, None, None)).unwrap();

    let surface = Arc::new(unsafe { iad.instance.create_surface(&window) });
    let format = surface
        .get_preferred_format(&iad.adapter)
        .expect("Could not get preferred format");

    rend3::configure_surface(
        &surface,
        &iad.device,
        format,
        glam::UVec2::new(window_size.width, window_size.height),
        rend3::types::PresentMode::Mailbox,
    );

    let renderer = rend3::Renderer::new(
        iad.clone(),
        rend3::types::Handedness::Left,
        Some(window_size.width as f32 / window_size.height as f32),
    )
    .unwrap();

    let rgraph = rend3_routine::base::BaseRenderGraph::new(&renderer);

    let mut data_core = renderer.data_core.lock();
    let pbr_routine =
        rend3_routine::pbr::PbrRoutine::new(&renderer, &mut data_core, &rgraph.interfaces);
    drop(data_core);

    let tonemapping_routine =
        rend3_routine::tonemapping::TonemappingRoutine::new(&renderer, &rgraph.interfaces, format);

    let font = glyph::wgpu_glyph::ab_glyph::FontArc::try_from_slice(include_bytes!(
        "../assets/Inconsolata-Regular.ttf"
    ))
    .unwrap();

    let mut glyph_routine = glyph::TextRenderRoutine::from_font(
        font,
        &iad.device,
        format,
        renderer.queue.clone(),
        glam::UVec2::new(window_size.width, window_size.height),
    );

    let mesh = create_mesh();
    let mesh_handle = renderer.add_mesh(mesh);

    let material = PbrMaterial {
        albedo: rend3_routine::pbr::AlbedoComponent::Value(glam::Vec4::new(0.0, 0.5, 0.5, 1.0)),
        ..rend3_routine::pbr::PbrMaterial::default()
    };
    let material_handle = renderer.add_material(material);

    let object = rend3::types::Object {
        mesh_kind: rend3::types::ObjectMeshKind::Static(mesh_handle),
        material: material_handle,
        transform: glam::Mat4::IDENTITY,
    };

    let _object_handle = renderer.add_object(object);

    let view_location = glam::Vec3::new(3.0, 3.0, -5.0);
    let view = glam::Mat4::from_euler(glam::EulerRot::XYZ, -0.55, 0.5, 0.0);
    let view = view * glam::Mat4::from_translation(-view_location);

    renderer.set_camera_data(rend3::types::Camera {
        projection: rend3::types::CameraProjection::Perspective {
            vfov: 60.0,
            near: 0.1,
        },
        view,
    });

    let _directional_handle = renderer.add_directional_light(rend3::types::DirectionalLight {
        color: glam::Vec3::ONE,
        intensity: 10.0,
        // Direction will be normalized
        direction: glam::Vec3::new(-1.0, -4.0, 2.0),
        distance: 400.0,
    });

    let mut resolution = glam::UVec2::new(window_size.width, window_size.height);
    let mut frame_count = 0;

    el.run(move |event, _, ctrl| {
        let string_to_print = &format!("Hello World: {}", frame_count);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *ctrl = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                resolution = glam::UVec2::new(physical_size.width, physical_size.height);

                rend3::configure_surface(
                    &surface,
                    &iad.device,
                    format,
                    glam::UVec2::new(resolution.x, resolution.y),
                    rend3::types::PresentMode::Mailbox,
                );

                renderer.set_aspect_ratio(resolution.x as f32 / resolution.y as f32);
            }
            Event::MainEventsCleared => {
                frame_count += 1;

                let frame = OutputFrame::Surface {
                    surface: Arc::clone(&surface),
                };

                let (cmd_bufs, ready) = renderer.ready();

                let mut graph = RenderGraph::new();

                rgraph.add_to_graph(
                    &mut graph,
                    &ready,
                    &pbr_routine,
                    None,
                    &tonemapping_routine,
                    resolution,
                    rend3::types::SampleCount::One,
                    glam::Vec4::ZERO,
                );

                let surface = graph.add_surface_texture();

                let sections = vec![wgpu_glyph::Section {
                    screen_position: (30.0, 30.0),
                    bounds: (resolution.x as f32, resolution.y as f32),
                    text: vec![wgpu_glyph::Text::new(&string_to_print)
                        .with_color([1.0, 1.0, 1.0, 1.0])
                        .with_scale(40.0)],
                    ..wgpu_glyph::Section::default()
                }];

                glyph_routine.add_to_graph(&mut graph, surface, sections);

                graph.execute(&renderer, frame, cmd_bufs, &ready);
            }
            _ => {}
        }
    })
}

fn vertex(pos: [f32; 3]) -> glam::Vec3 {
    glam::Vec3::from(pos)
}

fn create_mesh() -> rend3::types::Mesh {
    let vertex_positions = [
        // far side (0.0, 0.0, 1.0)
        vertex([-1.0, -1.0, 1.0]),
        vertex([1.0, -1.0, 1.0]),
        vertex([1.0, 1.0, 1.0]),
        vertex([-1.0, 1.0, 1.0]),
        // near side (0.0, 0.0, -1.0)
        vertex([-1.0, 1.0, -1.0]),
        vertex([1.0, 1.0, -1.0]),
        vertex([1.0, -1.0, -1.0]),
        vertex([-1.0, -1.0, -1.0]),
        // right side (1.0, 0.0, 0.0)
        vertex([1.0, -1.0, -1.0]),
        vertex([1.0, 1.0, -1.0]),
        vertex([1.0, 1.0, 1.0]),
        vertex([1.0, -1.0, 1.0]),
        // left side (-1.0, 0.0, 0.0)
        vertex([-1.0, -1.0, 1.0]),
        vertex([-1.0, 1.0, 1.0]),
        vertex([-1.0, 1.0, -1.0]),
        vertex([-1.0, -1.0, -1.0]),
        // top (0.0, 1.0, 0.0)
        vertex([1.0, 1.0, -1.0]),
        vertex([-1.0, 1.0, -1.0]),
        vertex([-1.0, 1.0, 1.0]),
        vertex([1.0, 1.0, 1.0]),
        // bottom (0.0, -1.0, 0.0)
        vertex([1.0, -1.0, 1.0]),
        vertex([-1.0, -1.0, 1.0]),
        vertex([-1.0, -1.0, -1.0]),
        vertex([1.0, -1.0, -1.0]),
    ];

    let index_data: &[u32] = &[
        0, 1, 2, 2, 3, 0, // far
        4, 5, 6, 6, 7, 4, // near
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // top
        20, 21, 22, 22, 23, 20, // bottom
    ];

    rend3::types::MeshBuilder::new(vertex_positions.to_vec(), rend3::types::Handedness::Left)
        .with_indices(index_data.to_vec())
        .build()
        .unwrap()
}
