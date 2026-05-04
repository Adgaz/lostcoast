#[derive(Debug, Clone, Copy)]
pub struct LightmapChart {
    pub mesh_index: u32,
    pub origin: [u32; 2],
    pub size: [u32; 2],
}

pub struct LightmapAtlas {
    pub width: u32,
    pub height: u32,
    pub charts: Vec<LightmapChart>,
}

pub fn pack_grid(mesh_count: u32, chart_size: u32, gutter: u32) -> LightmapAtlas {
    let cell = chart_size + gutter * 2;
    let cols = (mesh_count as f32).sqrt().ceil().max(1.0) as u32;
    let rows = mesh_count.div_ceil(cols);
    let width = cols * cell;
    let height = rows * cell;
    let mut charts = Vec::with_capacity(mesh_count as usize);
    for i in 0..mesh_count {
        let cx = i % cols;
        let cy = i / cols;
        charts.push(LightmapChart {
            mesh_index: i,
            origin: [cx * cell + gutter, cy * cell + gutter],
            size: [chart_size, chart_size],
        });
    }
    LightmapAtlas {
        width,
        height,
        charts,
    }
}
