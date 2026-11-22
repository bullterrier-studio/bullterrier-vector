//! The painter: what the drawing rules of `raster.rs` ask of a rasteriser, so that the rules are written once for every
//! rasteriser that paints them. The one painter so far is [`Cpu`] (`vello_cpu`), the only user of that crate; the
//! types the calls take (kurbo paths, peniko colours, gradients and blends, a `Mask`, a `Filter`) are the vocabulary
//! of `vello_common`, which `vello_cpu` and `vello_gpu` share, so another painter is an adapter of four calls.
//!
//! Every call is scoped: a clip, a layer or a move holds for the painting of its closure only, so a picture cannot be
//! left inside a layer it opened.

use kurbo::{Affine, BezPath, Vec2};
use vello_cpu::filter_effects::Filter;
use vello_cpu::peniko::{BlendMode, Fill, ImageAlphaType};
use vello_cpu::{
    Level, Mask, PaintType, Pixmap, RasterizerSettings, RenderContext, RenderMode, RenderSettings, Resources,
};

/// Paints paths into a picture of a fixed size, in device pixels, filled by the nonzero rule.
pub(super) trait Painter {
    /// Fills `path` with `paint`, composited source-over onto what is painted so far.
    fn fill(&mut self, path: &BezPath, paint: impl Into<PaintType>);

    /// Paints `draw` with all of it cut to the inside of `path`, without a layer of its own.
    fn clipped(&mut self, path: &BezPath, draw: impl FnOnce(&mut Self));

    /// Paints `draw` into a new transparent layer, then composites the layer as `layer` says.
    fn layered(&mut self, layer: Layer, draw: impl FnOnce(&mut Self));

    /// Paints `draw` with its paths and paints moved by `offset`.
    fn moved(&mut self, offset: Vec2, draw: impl FnOnce(&mut Self));
}

/// How a layer is composited onto what is below it: kept inside `clip`, filtered by `filter`, faded by `mask`'s
/// alpha and by `opacity`, then blended by `blend`. [`Layer::default`] composites it as it is.
pub(super) struct Layer<'a> {
    pub(super) clip: Option<&'a BezPath>,
    pub(super) blend: BlendMode,
    pub(super) opacity: f32,
    /// A mask the size of the picture; the rasteriser leaves out one of any other size.
    pub(super) mask: Option<Mask>,
    pub(super) filter: Option<Filter>,
}

impl Default for Layer<'_> {
    fn default() -> Self {
        Self {
            clip: None,
            blend: BlendMode::default(),
            opacity: 1.0,
            mask: None,
            filter: None,
        }
    }
}

/// The painter of `vello_cpu`, on one thread.
pub(super) struct Cpu(RenderContext);

impl Cpu {
    /// A transparent picture of `width` by `height` pixels.
    pub(super) fn new((width, height): (u16, u16)) -> Self {
        let mut context = RenderContext::new_with(
            width,
            height,
            RenderSettings {
                level: Level::new(),
                num_threads: 0,
            },
        );
        context.set_fill_rule(Fill::NonZero);
        Self(context)
    }

    /// The picture as premultiplied RGBA bytes, rows from the top.
    pub(super) fn into_premultiplied(self) -> Vec<u8> {
        self.into_pixmap().take_rgba8(ImageAlphaType::AlphaPremultiplied)
    }

    /// The picture as a mask: each pixel's luminance by its alpha.
    pub(super) fn into_mask(self) -> Mask {
        Mask::new_luminance(&self.into_pixmap())
    }

    fn into_pixmap(mut self) -> Pixmap {
        self.0.flush();
        let mut pixmap = Pixmap::new(self.0.width(), self.0.height());
        self.0.render_with(
            &mut pixmap,
            &mut Resources::new(),
            RasterizerSettings {
                render_mode: RenderMode::OptimizeSpeed,
                ..RasterizerSettings::default()
            },
        );
        pixmap
    }
}

impl Painter for Cpu {
    fn fill(&mut self, path: &BezPath, paint: impl Into<PaintType>) {
        self.0.set_paint(paint);
        self.0.fill_path(path);
    }

    fn clipped(&mut self, path: &BezPath, draw: impl FnOnce(&mut Self)) {
        self.0.push_clip_path(path);
        draw(self);
        self.0.pop_clip();
    }

    fn layered(&mut self, layer: Layer, draw: impl FnOnce(&mut Self)) {
        self.0.push_layer(
            layer.clip,
            Some(layer.blend),
            Some(layer.opacity),
            layer.mask,
            layer.filter,
        );
        draw(self);
        self.0.pop_layer();
    }

    fn moved(&mut self, offset: Vec2, draw: impl FnOnce(&mut Self)) {
        let before = *self.0.transform();
        self.0.set_transform(before * Affine::translate(offset));
        draw(self);
        self.0.set_transform(before);
    }
}
