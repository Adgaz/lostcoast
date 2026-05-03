use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

pub fn unit_cube() -> Mesh {
    type Quad = ([f32; 3], [f32; 3], [f32; 3], [f32; 3]);
    let faces: [(Quad, [f32; 3]); 6] = [
        (
            (
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ),
            [0.0, 0.0, 1.0],
        ),
        (
            (
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ),
            [0.0, 0.0, -1.0],
        ),
        (
            (
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
            ),
            [1.0, 0.0, 0.0],
        ),
        (
            (
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ),
            [-1.0, 0.0, 0.0],
        ),
        (
            (
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
            ),
            [0.0, 1.0, 0.0],
        ),
        (
            (
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
            ),
            [0.0, -1.0, 0.0],
        ),
    ];

    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (i, ((a, b, c, d), n)) in faces.into_iter().enumerate() {
        let base = (i * 4) as u16;
        for (pos, uv) in [a, b, c, d].into_iter().zip(uvs) {
            vertices.push(Vertex { pos, normal: n, uv });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh { vertices, indices }
}
