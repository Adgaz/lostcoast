use glam::Vec3;

#[derive(Debug, Clone, Copy, Default)]
pub struct AmbientCube {
    pub axes: [[f32; 3]; 6],
}

impl AmbientCube {
    pub fn from_axes(axes: [[f32; 3]; 6]) -> Self {
        Self { axes }
    }

    pub fn sample(&self, n: Vec3) -> Vec3 {
        let nsq = n * n;
        let xi = if n.x >= 0.0 { 0 } else { 1 };
        let yi = if n.y >= 0.0 { 2 } else { 3 };
        let zi = if n.z >= 0.0 { 4 } else { 5 };
        Vec3::from_array(self.axes[xi]) * nsq.x
            + Vec3::from_array(self.axes[yi]) * nsq.y
            + Vec3::from_array(self.axes[zi]) * nsq.z
    }
}

#[derive(Debug, Clone)]
pub struct ProbeVolume {
    pub origin: Vec3,
    pub spacing: Vec3,
    pub dims: [u32; 3],
    pub probes: Vec<AmbientCube>,
}

impl ProbeVolume {
    pub fn index(&self, i: u32, j: u32, k: u32) -> usize {
        let [x, y, _z] = self.dims;
        (k * y * x + j * x + i) as usize
    }

    pub fn sample_trilinear(&self, p: Vec3, n: Vec3) -> Vec3 {
        let local = (p - self.origin) / self.spacing;
        let lx = local.x.clamp(0.0, (self.dims[0] - 1) as f32);
        let ly = local.y.clamp(0.0, (self.dims[1] - 1) as f32);
        let lz = local.z.clamp(0.0, (self.dims[2] - 1) as f32);
        let i0 = lx.floor() as u32;
        let j0 = ly.floor() as u32;
        let k0 = lz.floor() as u32;
        let i1 = (i0 + 1).min(self.dims[0] - 1);
        let j1 = (j0 + 1).min(self.dims[1] - 1);
        let k1 = (k0 + 1).min(self.dims[2] - 1);
        let fx = lx - i0 as f32;
        let fy = ly - j0 as f32;
        let fz = lz - k0 as f32;
        let mut acc = Vec3::ZERO;
        for (ki, kw) in [(k0, 1.0 - fz), (k1, fz)] {
            for (ji, jw) in [(j0, 1.0 - fy), (j1, fy)] {
                for (ii, iw) in [(i0, 1.0 - fx), (i1, fx)] {
                    let w = iw * jw * kw;
                    if w == 0.0 {
                        continue;
                    }
                    let probe = &self.probes[self.index(ii, ji, ki)];
                    acc += probe.sample(n) * w;
                }
            }
        }
        acc
    }
}
