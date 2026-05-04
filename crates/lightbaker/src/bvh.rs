use glam::Vec3;

use crate::scene::BakeTriangle;

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub tmax: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub t: f32,
    pub triangle: u32,
}

#[derive(Debug, Clone, Copy)]
struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }
    fn expand_point(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }
    fn expand(&mut self, other: Aabb) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }
    fn for_tri(t: &BakeTriangle) -> Self {
        let mut a = Aabb::empty();
        a.expand_point(t.v0);
        a.expand_point(t.v1);
        a.expand_point(t.v2);
        a
    }
    fn centroid(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    fn intersect(&self, r: &Ray) -> Option<f32> {
        let inv = 1.0 / r.direction;
        let t0 = (self.min - r.origin) * inv;
        let t1 = (self.max - r.origin) * inv;
        let lo = t0.min(t1);
        let hi = t0.max(t1);
        let tmin = lo.x.max(lo.y).max(lo.z).max(0.0);
        let tmax = hi.x.min(hi.y).min(hi.z).min(r.tmax);
        if tmin <= tmax {
            Some(tmin)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
struct Node {
    bounds: Aabb,
    left: u32,
    right: u32,
    first: u32,
    count: u32,
}

pub struct Bvh {
    nodes: Vec<Node>,
    indices: Vec<u32>,
    triangles: Vec<BakeTriangle>,
}

impl Bvh {
    pub fn build(triangles: Vec<BakeTriangle>) -> Self {
        let n = triangles.len() as u32;
        let indices: Vec<u32> = (0..n).collect();
        let mut nodes = Vec::with_capacity((triangles.len() * 2).max(1));
        nodes.push(Node {
            bounds: Aabb::empty(),
            left: u32::MAX,
            right: u32::MAX,
            first: 0,
            count: n,
        });
        let mut bvh = Self {
            nodes,
            indices,
            triangles,
        };
        if n > 0 {
            bvh.build_node(0);
        }
        bvh
    }

    fn build_node(&mut self, idx: u32) {
        let (first, count) = {
            let n = &mut self.nodes[idx as usize];
            n.bounds = Aabb::empty();
            for i in n.first..n.first + n.count {
                let t = &self.triangles[self.indices[i as usize] as usize];
                n.bounds.expand(Aabb::for_tri(t));
            }
            (n.first, n.count)
        };

        if count <= 4 {
            return;
        }

        let bounds = self.nodes[idx as usize].bounds;
        let extent = bounds.max - bounds.min;
        let axis = if extent.x > extent.y && extent.x > extent.z {
            0
        } else if extent.y > extent.z {
            1
        } else {
            2
        };
        let mid = bounds.centroid()[axis];

        let mut i = first;
        let mut j = first + count;
        while i < j {
            let tri = &self.triangles[self.indices[i as usize] as usize];
            let c = Aabb::for_tri(tri).centroid()[axis];
            if c < mid {
                i += 1;
            } else {
                j -= 1;
                self.indices.swap(i as usize, j as usize);
            }
        }
        let left_count = i - first;
        if left_count == 0 || left_count == count {
            return;
        }

        let left_idx = self.nodes.len() as u32;
        self.nodes.push(Node {
            bounds: Aabb::empty(),
            left: u32::MAX,
            right: u32::MAX,
            first,
            count: left_count,
        });
        let right_idx = self.nodes.len() as u32;
        self.nodes.push(Node {
            bounds: Aabb::empty(),
            left: u32::MAX,
            right: u32::MAX,
            first: i,
            count: count - left_count,
        });
        self.nodes[idx as usize].left = left_idx;
        self.nodes[idx as usize].right = right_idx;
        self.nodes[idx as usize].count = 0;
        self.build_node(left_idx);
        self.build_node(right_idx);
    }

    pub fn closest_hit(&self, ray: &Ray) -> Option<Hit> {
        if self.triangles.is_empty() {
            return None;
        }
        let mut closest: Option<Hit> = None;
        let mut stack = [0u32; 64];
        let mut sp = 1;
        stack[0] = 0;
        let mut tmax = ray.tmax;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            let r2 = Ray { tmax, ..*ray };
            if node.bounds.intersect(&r2).is_none() {
                continue;
            }
            if node.count > 0 {
                for i in node.first..node.first + node.count {
                    let t_idx = self.indices[i as usize];
                    let tri = &self.triangles[t_idx as usize];
                    if let Some(t) = intersect_triangle(ray, tri) {
                        if t > 1e-5 && t < tmax {
                            tmax = t;
                            closest = Some(Hit { t, triangle: t_idx });
                        }
                    }
                }
            } else {
                if sp + 2 <= stack.len() {
                    stack[sp] = node.left;
                    stack[sp + 1] = node.right;
                    sp += 2;
                }
            }
        }
        closest
    }

    pub fn occluded(&self, ray: &Ray) -> bool {
        self.closest_hit(ray).is_some()
    }

    pub fn triangle(&self, i: u32) -> &BakeTriangle {
        &self.triangles[i as usize]
    }

    pub fn triangles(&self) -> &[BakeTriangle] {
        &self.triangles
    }
}

pub fn intersect_triangle(ray: &Ray, tri: &BakeTriangle) -> Option<f32> {
    let edge1 = tri.v1 - tri.v0;
    let edge2 = tri.v2 - tri.v0;
    let h = ray.direction.cross(edge2);
    let a = edge1.dot(h);
    if a.abs() < 1e-9 {
        return None;
    }
    let f = 1.0 / a;
    let s = ray.origin - tri.v0;
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(edge1);
    let v = f * ray.direction.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = f * edge2.dot(q);
    if t > 0.0 {
        Some(t)
    } else {
        None
    }
}
