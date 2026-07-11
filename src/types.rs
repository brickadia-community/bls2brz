pub use brdb::Direction;

pub type BrickMapping = Vec<BrickDesc>;

/// RGBA color used throughout the mappings. brdb's own `Color` is RGB-only; the
/// alpha channel is preserved here so `lib.rs` can translate it into a material
/// choice (opaque vs. translucent plastic) when building the final brick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl From<Color> for brdb::Color {
    fn from(c: Color) -> Self {
        brdb::Color::new(c.r, c.g, c.b)
    }
}

#[derive(Debug, Clone)]
pub struct BrickDesc {
    pub asset: &'static str,
    pub size: (u32, u32, u32),
    pub offset: (i32, i32, i32),
    pub rotation_offset: u8,
    pub color_override: Option<Color>,
    pub direction_override: Option<Direction>,
    pub non_priority: bool,
    pub microwedge_rotate: bool,
    pub inverted_modter_rotate: bool,
    pub inverted_wedge_rotate: bool,
    pub modter: bool,
    pub rotate_by_direction: bool,
    pub nocollide: bool,
}

impl BrickDesc {
    pub const fn new(asset: &'static str) -> Self {
        Self {
            asset,
            size: (0, 0, 0),
            offset: (0, 0, 0),
            rotation_offset: 1,
            color_override: None,
            direction_override: None,
            non_priority: false,
            microwedge_rotate: false,
            inverted_modter_rotate: false,
            inverted_wedge_rotate: false,
            modter: false,
            rotate_by_direction: false,
            nocollide: false,
        }
    }

    pub fn size(mut self, size: (u32, u32, u32)) -> Self {
        self.size = size;
        self
    }

    pub fn offset(mut self, offset: (i32, i32, i32)) -> Self {
        self.offset = offset;
        self
    }

    pub fn rotation_offset(mut self, rotation: u8) -> Self {
        self.rotation_offset = rotation;
        self
    }

    pub fn color_override(mut self, color_override: Color) -> Self {
        self.color_override = Some(color_override);
        self
    }

    pub fn direction_override(mut self, direction_override: Direction) -> Self {
        self.direction_override = Some(direction_override);
        self
    }

    pub fn non_priority(mut self, non_priority: bool) -> Self {
        self.non_priority = non_priority;
        self
    }

    pub fn microwedge_rotate(mut self, microwedge_rotate: bool) -> Self {
        self.microwedge_rotate = microwedge_rotate;
        self
    }

    pub fn inverted_modter_rotate(mut self, inverted_modter_rotate: bool) -> Self {
        self.inverted_modter_rotate = inverted_modter_rotate;
        self
    }

    pub fn inverted_wedge_rotate(mut self, inverted_wedge_rotate: bool) -> Self {
        self.inverted_wedge_rotate = inverted_wedge_rotate;
        self
    }

    pub fn modter(mut self, modter: bool) -> Self {
        self.modter = modter;
        self
    }

    pub fn rotate_by_direction(mut self) -> Self {
        self.rotate_by_direction = true;
        self
    }

    pub fn nocollide(mut self) -> Self {
        self.nocollide = true;
        self
    }
}

impl From<BrickDesc> for BrickMapping {
    fn from(desc: BrickDesc) -> Self {
        vec![desc]
    }
}
