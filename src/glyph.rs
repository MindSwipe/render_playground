use std::sync::Arc;

use rend3::{
    graph::{RenderGraph, RenderTargetHandle},
    types::TextureFormat,
};
use wgpu::{Device, Queue};
use wgpu_glyph::{ab_glyph::FontArc, GlyphBrushBuilder, Section};

pub use wgpu_glyph;

pub struct TextRenderRoutine {
    pub glyph_brush: wgpu_glyph::GlyphBrush<()>,
    pub viewport_size: glam::UVec2,
    pub queue: Arc<Queue>,
}

impl TextRenderRoutine {
    pub fn from_font(
        font: FontArc,
        device: &Device,
        format: TextureFormat,
        queue: Arc<Queue>,
        size: glam::UVec2,
    ) -> Self {
        let glyph_brush = GlyphBrushBuilder::using_font(font).build(device, format);

        Self {
            glyph_brush,
            viewport_size: size,
            queue,
        }
    }

    pub fn add_to_graph<'node>(
        &'node mut self,
        graph: &mut RenderGraph<'node>,
        output: RenderTargetHandle,
        sections: Vec<Section<'node>>,
    ) {
        let mut builder = graph.add_node("wgpu_glyph");

        let output_handle = builder.add_render_target_output(output);
        let pt_handle = builder.passthrough_ref_mut(self);

        builder.build(
            move |pt, renderer, encoder_or_pass, _temps, _ready, graph_data| {
                let this = pt.get_mut(pt_handle);
                let encoder = encoder_or_pass.get_encoder();

                for section in sections {
                    this.glyph_brush.queue(section);
                }

                let output = graph_data.get_render_target(output_handle);

                let draw_op = this.glyph_brush.draw_queued(
                    &renderer.device,
                    &mut this.queue,
                    encoder,
                    output,
                    this.viewport_size.x,
                    this.viewport_size.y,
                );

                match draw_op {
                    Ok(()) => {}
                    Err(e) => eprintln!("[ERROR]: Rendering text failed: {}", e),
                }
            },
        );
    }
}
