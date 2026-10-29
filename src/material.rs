#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Material {
    Empty,
    Sand,
    Water,
    Gas,
    Wall,
    Glass,
    Shards,
}

impl Material {
    pub fn density(self) -> u8 {
        match self {
            Material::Empty => 0,
            Material::Gas => 1,
            Material::Water => 3,
            Material::Glass => 4,
            Material::Shards => 4,
            Material::Sand => 5,
            Material::Wall => 255,
        }
    }

    pub fn color(self) -> [u8; 4] {
        match self {
            Material::Empty => [10, 10, 15, 255],
            Material::Sand => [194, 178, 128, 255],
            Material::Water => [40, 90, 200, 255],
            Material::Gas => [170, 220, 170, 255],
            Material::Wall => [100, 100, 100, 255],
            Material::Glass => [120, 200, 220, 255],
            Material::Shards => [200, 235, 240, 255],
        }
    }

    /// Walls and empty space never move.
    pub fn is_movable(self) -> bool {
        !matches!(self, Material::Empty | Material::Wall)
    }

    /// 1.0 = normal gravity, negative = buoyant (rises).
    pub fn gravity_scale(self) -> f32 {
        match self {
            Material::Gas => -0.4,
            Material::Empty | Material::Wall => 0.0,
            _ => 1.0,
        }
    }

    /// How far a resting particle spreads sideways (liquids and gases).
    pub fn dispersion(self) -> i32 {
        match self {
            Material::Water => 4,
            Material::Gas => 2,
            _ => 0,
        }
    }

    /// If Some((speed, becomes)): hitting something at that speed or
    /// faster turns this material into `becomes`.
    pub fn shatter_threshold(self) -> Option<(f32, Material)> {
        match self {
            Material::Glass => Some((2.5, Material::Shards)),
            _ => None,
        }
    }
}