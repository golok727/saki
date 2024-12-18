// TODO: add bytemuck_feature
#[derive(Clone, Copy, Eq, PartialEq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Color {
    pub const TRANSPARENT: Self = Self::from_rgba(0x00000000);
    pub const WHITE: Self = Self::from_rgb(0xFFFFFF);
    pub const BLACK: Self = Self::from_rgb(0x000000);

    pub const THAMAR_BLACK: Self = Self::from_rgb(0x181818);

    pub const BROWN: Self = Self::from_rgb(0xA52A2A);
    pub const TORCH_RED: Self = Self::from_rgb(0xff2233);
    pub const DARK_RED: Self = Self::from_rgb(0x8B0000);
    pub const RED: Self = Self::from_rgb(0xFF0000);
    pub const LIGHT_RED: Self = Self::from_rgb(0xFF8080);

    pub const YELLOW: Self = Self::from_rgb(0xFFFF00);
    pub const ORANGE: Self = Self::from_rgb(0xFFA500);
    pub const LIGHT_YELLOW: Self = Self::from_rgb(0xFFFFE0);
    pub const KHAKI: Self = Self::from_rgb(0xF0E68C);

    pub const DARK_GREEN: Self = Self::from_rgb(0x006400);
    pub const GREEN: Self = Self::from_rgb(0x00FF00);
    pub const LIGHT_GREEN: Self = Self::from_rgb(0x90EE90);

    pub const DARK_BLUE: Self = Self::from_rgb(0x00008B);
    pub const BLUE: Self = Self::from_rgb(0x0000FF);
    pub const LIGHT_BLUE: Self = Self::from_rgb(0xADD8E6);

    pub const GOLD: Self = Self::from_rgb(0xFFD700);

    pub const DARK_GRAY: Self = Self::from_rgb(0x606060);
    pub const GRAY: Self = Self::from_rgb(0xA0A0A0);
    pub const LIGHT_GRAY: Self = Self::from_rgb(0xDCDCDC);

    #[inline]
    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }

    // Without alpha use 0xRRGGBB
    #[inline]
    pub const fn from_rgb(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xff) as u8,
            g: ((hex >> 8) & 0xff) as u8,
            b: ((hex) & 0xff) as u8,
            a: 255,
        }
    }

    #[inline]
    pub const fn from_rgb_additive(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 24) & 0xff) as u8,
            g: ((rgb >> 16) & 0xff) as u8,
            b: ((rgb >> 8) & 0xff) as u8,
            a: 0,
        }
    }

    /// With premultiplied alpha
    #[inline]
    pub const fn from_rgba(rgba: u32) -> Self {
        Self {
            r: ((rgba >> 24) & 0xff) as u8,
            g: ((rgba >> 16) & 0xff) as u8,
            b: ((rgba >> 8) & 0xff) as u8,
            a: (rgba & 0xff) as u8,
        }
    }
}

impl From<u32> for Color {
    fn from(color: u32) -> Self {
        Self {
            r: ((color >> 24) & 0xff) as u8,
            g: ((color >> 16) & 0xff) as u8,
            b: ((color >> 8) & 0xff) as u8,
            a: (color & 0xff) as u8,
        }
    }
}

impl From<[u8; 4]> for Color {
    fn from(color: [u8; 4]) -> Self {
        Self {
            r: color[0],
            g: color[1],
            b: color[2],
            a: color[3],
        }
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    fn from(color: (u8, u8, u8, u8)) -> Self {
        Self {
            r: color.0,
            g: color.1,
            b: color.2,
            a: color.3,
        }
    }
}

impl From<Color> for (u8, u8, u8, u8) {
    fn from(color: Color) -> Self {
        (color.r, color.g, color.b, color.a)
    }
}

impl From<Color> for [u8; 4] {
    fn from(color: Color) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

impl From<Color> for u32 {
    fn from(color: Color) -> Self {
        ((color.r as u32) << 24)
            | ((color.g as u32) << 16)
            | ((color.b as u32) << 8)
            | color.a as u32
    }
}

impl std::ops::Index<usize> for Color {
    type Output = u8;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.r,
            1 => &self.g,
            2 => &self.b,
            3 => &self.a,
            _ => panic!("Color: index out of bounds expected 0 <= index <= 3"),
        }
    }
}

impl std::ops::IndexMut<usize> for Color {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut u8 {
        match index {
            0 => &mut self.r,
            1 => &mut self.g,
            2 => &mut self.b,
            3 => &mut self.a,
            _ => panic!("Color: index out of bounds expected 0 <= index <= 3"),
        }
    }
}

#[derive(PartialEq, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<Rgba> for u32 {
    fn from(rgba: Rgba) -> Self {
        let r = (rgba.r * 255.0) as u32;
        let g = (rgba.g * 255.0) as u32;
        let b = (rgba.b * 255.0) as u32;
        let a = (rgba.a * 255.0) as u32;
        (r << 24) | (g << 16) | (b << 8) | a
    }
}

impl From<u32> for Rgba {
    #[inline]
    fn from(value: u32) -> Self {
        Self::from_rgba(value)
    }
}

impl From<Color> for Rgba {
    fn from(color: Color) -> Self {
        let r = (color.r as f32) / 255.0;
        let g = (color.g as f32) / 255.0;
        let b = (color.b as f32) / 255.0;
        let a = (color.a as f32) / 255.0;
        Self { r, g, b, a }
    }
}

impl Rgba {
    pub const TRANSPARENT: Self = Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const WHITE: Self = Rgba {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    pub fn from_rgb(hex: u32) -> Rgba {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self { r, g, b, a: 1.0 }
    }

    pub fn from_rgba(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let b = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let a = (hex & 0xFF) as f32 / 255.0;
        Self { r, g, b, a }
    }

    pub fn blend(&self, other: Rgba) -> Self {
        if other.a >= 1.0 {
            other
        } else if other.a <= 0.0 {
            return *self;
        } else {
            return Rgba {
                r: (self.r * (1.0 - other.a)) + (other.r * other.a),
                g: (self.g * (1.0 - other.a)) + (other.g * other.a),
                b: (self.b * (1.0 - other.a)) + (other.b * other.a),
                a: self.a,
            };
        }
    }
}

impl std::fmt::Debug for Rgba {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "rgba({:#010x})", u32::from(*self))
    }
}

impl std::fmt::Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "color({:#010x})", u32::from(*self))
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        let r = (color.r as f64) / 255.0;
        let g = (color.g as f64) / 255.0;
        let b = (color.b as f64) / 255.0;
        let a = (color.a as f64) / 255.0;
        Self { r, g, b, a }
    }
}
