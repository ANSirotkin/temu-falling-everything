/// A material's identity. All of its properties live in the table below.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Material {
    Empty,
    Sand,
    Water,
    Gas,
    Wall,
    Glass,
    Shards,
    Vessel,
}

/// What happens to a particle that hits something fast enough.
#[derive(Clone, Copy, Debug)]
pub struct Shatter {
    pub speed: f32,
    pub becomes: Material,
}

/// Everything the simulation needs to know about one material.
#[derive(Clone, Copy, Debug)]
pub struct MaterialDef {
    pub id: Material, // must match this row's position (checked by a test)
    pub name: &'static str,
    pub color: [u8; 4],
    pub density: u8,    
    pub movable: bool,
    pub gravity_scale: f32, // 1.0 normal, negative = rises, 0.0 = ignores gravity
    pub dispersion: i32,    // how far it spreads sideways at rest
    pub shatter: Option<Shatter>,
}

/// Defaults: a plain powder in loud magenta, so a forgotten field is obvious.
const DEFAULT: MaterialDef = MaterialDef {
    id: Material::Empty,
    name: "?",
    color: [255, 0, 255, 255],
    density: 5,
    movable: true,
    gravity_scale: 1.0,
    dispersion: 0,
    shatter: None,
};

/// One row per material, in the SAME ORDER as the enum.
static MATERIALS: [MaterialDef; 8] = [
    MaterialDef {
        id: Material::Empty,
        name: "Empty",
        color: [10, 10, 15, 255],
        density: 0,
        movable: false,
        gravity_scale: 0.0,
        ..DEFAULT
    },
    MaterialDef {
        id: Material::Sand,
        name: "Sand",
        color: [194, 178, 128, 255],
        density: 5,
        ..DEFAULT
    },
    MaterialDef {
        id: Material::Water,
        name: "Water",
        color: [40, 90, 200, 255],
        density: 3,
        dispersion: 4,
        ..DEFAULT
    },
    MaterialDef {
        id: Material::Gas,
        name: "Gas",
        color: [170, 220, 170, 255],
        density: 1,
        gravity_scale: -0.4,
        dispersion: 2,
        ..DEFAULT
    },
    MaterialDef {
        id: Material::Wall,
        name: "Wall",
        color: [100, 100, 100, 255],
        density: 255,
        movable: false,
        gravity_scale: 0.0,
        ..DEFAULT
    },
    MaterialDef {
        id: Material::Glass,
        name: "Glass",
        color: [120, 200, 220, 255],
        density: 4,
        shatter: Some(Shatter {
            speed: 2.5,
            becomes: Material::Shards,
        }),
        ..DEFAULT
    },
    MaterialDef {
        id: Material::Shards,
        name: "Shards",
        color: [200, 235, 240, 255],
        density: 4,
        ..DEFAULT
    },

    MaterialDef {
        id: Material::Vessel,
        name: "Vessel",
        color: [170, 205, 215, 255],
        density: 255,
        movable: false,
        gravity_scale: 0.0,
        ..DEFAULT
    },
];

impl Material {
    /// Materials the player can paint with (number keys 1, 2, 3...).
    pub const PLACEABLE: [Material; 5] = [
        Material::Sand,
        Material::Water,
        Material::Gas,
        Material::Wall,
        Material::Glass,
    ];

    pub fn def(self) -> &'static MaterialDef {
        &MATERIALS[self as usize]
    }

    // Thin wrappers, so the rest of the code keeps working unchanged.
    pub fn name(self) -> &'static str {
        self.def().name
    }

    pub fn color(self) -> [u8; 4] {
        self.def().color
    }

    pub fn density(self) -> u8 {
        self.def().density
    }

    pub fn is_movable(self) -> bool {
        self.def().movable
    }

    pub fn gravity_scale(self) -> f32 {
        self.def().gravity_scale
    }

    pub fn dispersion(self) -> i32 {
        self.def().dispersion
    }

    pub fn shatter_threshold(self) -> Option<(f32, Material)> {
        self.def().shatter.map(|s| (s.speed, s.becomes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_rows_match_enum_order() {
        for (i, def) in MATERIALS.iter().enumerate() {
            assert_eq!(def.id as usize, i, "row {} ({}) is out of order", i, def.name);
        }
    }

    #[test]
    fn immovable_materials_ignore_gravity() {
        for def in MATERIALS.iter().filter(|d| !d.movable) {
            assert_eq!(def.gravity_scale, 0.0, "{} is immovable", def.name);
        }
    }
}