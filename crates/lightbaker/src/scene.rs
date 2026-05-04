use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct BakeTriangle {
    pub v0: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
    pub mesh: u32,
}

impl BakeTriangle {
    pub fn normal(&self) -> Vec3 {
        (self.v1 - self.v0).cross(self.v2 - self.v0).normalize()
    }
    pub fn area(&self) -> f32 {
        (self.v1 - self.v0).cross(self.v2 - self.v0).length() * 0.5
    }
    pub fn centroid(&self) -> Vec3 {
        (self.v0 + self.v1 + self.v2) / 3.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct BakeMesh {
    pub albedo: Vec3,
    pub emissive: Vec3,
    pub triangles: Vec<BakeTriangle>,
}

#[derive(Debug, Clone, Default)]
pub struct BakeScene {
    pub meshes: Vec<BakeMesh>,
}

impl BakeScene {
    pub fn flatten(&self) -> Vec<BakeTriangle> {
        self.meshes
            .iter()
            .enumerate()
            .flat_map(|(idx, m)| {
                m.triangles.iter().map(move |t| BakeTriangle {
                    mesh: idx as u32,
                    ..*t
                })
            })
            .collect()
    }

    pub fn closed_unit_cube(rho: f32) -> Self {
        let face = |verts: [Vec3; 4], albedo: Vec3, emissive: Vec3| BakeMesh {
            albedo,
            emissive,
            triangles: vec![
                BakeTriangle {
                    v0: verts[0],
                    v1: verts[1],
                    v2: verts[2],
                    mesh: 0,
                },
                BakeTriangle {
                    v0: verts[0],
                    v1: verts[2],
                    v2: verts[3],
                    mesh: 0,
                },
            ],
        };
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(1.0, 1.0, 0.0);
        let d = Vec3::new(0.0, 1.0, 0.0);
        let e = Vec3::new(0.0, 0.0, 1.0);
        let f = Vec3::new(1.0, 0.0, 1.0);
        let g = Vec3::new(1.0, 1.0, 1.0);
        let h = Vec3::new(0.0, 1.0, 1.0);
        let albedo = Vec3::splat(rho);
        BakeScene {
            meshes: vec![
                face([a, b, c, d], albedo, Vec3::splat(1.0)),
                face([h, g, f, e], albedo, Vec3::ZERO),
                face([a, e, f, b], albedo, Vec3::ZERO),
                face([d, c, g, h], albedo, Vec3::ZERO),
                face([a, d, h, e], albedo, Vec3::ZERO),
                face([b, f, g, c], albedo, Vec3::ZERO),
            ],
        }
    }

    pub fn flat_plane_overhead() -> Self {
        let a = Vec3::new(-1.0, 0.0, -1.0);
        let b = Vec3::new(1.0, 0.0, -1.0);
        let c = Vec3::new(1.0, 0.0, 1.0);
        let d = Vec3::new(-1.0, 0.0, 1.0);
        BakeScene {
            meshes: vec![BakeMesh {
                albedo: Vec3::ONE,
                emissive: Vec3::ZERO,
                triangles: vec![
                    BakeTriangle {
                        v0: a,
                        v1: b,
                        v2: c,
                        mesh: 0,
                    },
                    BakeTriangle {
                        v0: a,
                        v1: c,
                        v2: d,
                        mesh: 0,
                    },
                ],
            }],
        }
    }

    pub fn cornell_one_light() -> Self {
        let mut s = BakeScene::default();
        let push = |s: &mut BakeScene, v: [Vec3; 4], albedo: Vec3, emissive: Vec3| {
            s.meshes.push(BakeMesh {
                albedo,
                emissive,
                triangles: vec![
                    BakeTriangle {
                        v0: v[0],
                        v1: v[1],
                        v2: v[2],
                        mesh: 0,
                    },
                    BakeTriangle {
                        v0: v[0],
                        v1: v[2],
                        v2: v[3],
                        mesh: 0,
                    },
                ],
            });
        };
        let a = Vec3::new(-1.0, 0.0, -1.0);
        let b = Vec3::new(1.0, 0.0, -1.0);
        let c = Vec3::new(1.0, 0.0, 1.0);
        let d = Vec3::new(-1.0, 0.0, 1.0);
        let e = Vec3::new(-1.0, 2.0, -1.0);
        let f = Vec3::new(1.0, 2.0, -1.0);
        let g = Vec3::new(1.0, 2.0, 1.0);
        let h = Vec3::new(-1.0, 2.0, 1.0);
        push(&mut s, [a, b, c, d], Vec3::splat(0.7), Vec3::ZERO);
        push(&mut s, [e, h, g, f], Vec3::splat(0.7), Vec3::ZERO);
        push(&mut s, [a, e, f, b], Vec3::splat(0.7), Vec3::ZERO);
        push(&mut s, [a, d, h, e], Vec3::new(0.6, 0.1, 0.1), Vec3::ZERO);
        push(&mut s, [b, f, g, c], Vec3::new(0.1, 0.6, 0.1), Vec3::ZERO);
        s
    }
}
