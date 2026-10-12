//! SolidCraft-owned portable B-rep data, independent of a modelling backend.
//! Millimetres, radians, right-handed Z-up. See docs/native-brep.md for semantics.
//! Validation checks structure and resource budgets, not geometric equivalence or trust.
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const MAX_ITEMS: usize = 100_000;
const MAX_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brep {
    pub version: u32,
    pub shells: Vec<Shell>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shell {
    pub vertices: Vec<[f64; 3]>,
    pub edges: Vec<Edge>,
    pub faces: Vec<Face>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub vertices: [u32; 2],
    pub curve: Curve,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Face {
    pub loops: Vec<Vec<EdgeUse>>,
    pub surface: Surface,
    pub forward: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeUse {
    pub edge: u32,
    pub forward: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Curve {
    Line { start: [f64; 3], end: [f64; 3] },
    Polyline { points: Vec<[f64; 3]> },
    Spline { knots: Vec<f64>, points: Vec<[f64; 3]> },
    RationalSpline { knots: Vec<f64>, homogeneous: Vec<[f64; 4]> },
    Intersection { leader: Box<Curve>, surfaces: [Box<Surface>; 2], tolerance: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Surface {
    Plane { origin: [f64; 3], u_axis: [f64; 3], v_axis: [f64; 3] },
    Spline { u_knots: Vec<f64>, v_knots: Vec<f64>, points: Vec<Vec<[f64; 3]>> },
    RationalSpline { u_knots: Vec<f64>, v_knots: Vec<f64>, homogeneous: Vec<Vec<[f64; 4]>> },
    Revolution { profile: Box<Curve>, origin: [f64; 3], axis: [f64; 3], columns: [[f64; 4]; 4], forward: bool },
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("portable B-rep: {0}")]
pub struct Invalid(pub &'static str);

type Result<T> = std::result::Result<T, Invalid>;

fn require(condition: bool, message: &'static str) -> Result<()> {
    if condition { Ok(()) } else { Err(Invalid(message)) }
}

struct Budget(usize);
impl Budget {
    fn add(&mut self, n: usize) -> Result<()> {
        self.0 = self.0.checked_add(n).ok_or(Invalid("item budget"))?;
        require(self.0 <= MAX_ITEMS, "item budget")
    }
}
fn numbers(values: impl IntoIterator<Item = f64>) -> Result<()> {
    require(values.into_iter().all(|v| v.is_finite() && v.abs() <= 1e12), "non-finite or excessive coordinate")
}
fn points<const N: usize>(values: &[[f64; N]], budget: &mut Budget) -> Result<()> {
    budget.add(values.len())?;
    numbers(values.iter().flatten().copied())
}
fn knots(values: &[f64], count: usize, budget: &mut Budget) -> Result<()> {
    budget.add(values.len())?;
    numbers(values.iter().copied())?;
    let degree = values.len().checked_sub(count.saturating_add(1)).ok_or(Invalid("spline degree"))?;
    require((2..=4096).contains(&count) && (1..=16).contains(&degree) && degree < count, "spline degree or size")?;
    require(values.windows(2).all(|w| w.first() <= w.get(1)), "unordered knots")?;
    require(values.get(degree) < values.get(count), "empty spline domain")?;
    // No knot may have multiplicity greater than degree + 1.
    let mut run = 1;
    for pair in values.windows(2) {
        run = if pair.first() == pair.get(1) { run + 1 } else { 1 };
        require(run <= degree + 1, "knot multiplicity")?;
    }
    Ok(())
}
fn weights(values: &[[f64; 4]]) -> Result<()> {
    require(values.iter().all(|p| p.get(3).is_some_and(|w| *w >= 1e-12)), "non-positive rational weight")
}
fn grid<const N: usize>(u: &[f64], v: &[f64], rows: &[Vec<[f64; N]>], budget: &mut Budget) -> Result<()> {
    let columns = rows.first().map_or(0, Vec::len);
    knots(u, rows.len(), budget)?;
    knots(v, columns, budget)?;
    for row in rows {
        require(row.len() == columns, "ragged surface")?;
        points(row, budget)?;
    }
    Ok(())
}
fn vector(p: [f64; 3]) -> crate::Vec3 {
    crate::Vec3::new(p[0], p[1], p[2])
}
fn curve(c: &Curve, depth: usize, budget: &mut Budget) -> Result<()> {
    require(depth <= MAX_DEPTH, "geometry depth")?;
    budget.add(1)?;
    match c {
        Curve::Line { start, end } => {
            points(&[*start, *end], budget)?;
            require(start != end, "collapsed line")
        }
        Curve::Polyline { points: p } => {
            require(p.len() >= 2, "empty polyline")?;
            points(p, budget)?;
            require(p.windows(2).all(|p| p.first() != p.get(1)), "collapsed polyline segment")
        }
        Curve::Spline { knots: k, points: p } => {
            knots(k, p.len(), budget)?;
            points(p, budget)
        }
        Curve::RationalSpline { knots: k, homogeneous: p } => {
            knots(k, p.len(), budget)?;
            points(p, budget)?;
            weights(p)
        }
        Curve::Intersection { leader, surfaces, tolerance } => {
            require(matches!(leader.as_ref(), Curve::Polyline { .. } | Curve::Spline { .. }), "intersection leader")?;
            require((1e-12..=1.0).contains(tolerance), "intersection tolerance")?;
            curve(leader, depth + 1, budget)?;
            for s in surfaces {
                surface(s, depth + 1, budget)?;
            }
            Ok(())
        }
    }
}
fn surface(s: &Surface, depth: usize, budget: &mut Budget) -> Result<()> {
    require(depth <= MAX_DEPTH, "geometry depth")?;
    budget.add(1)?;
    match s {
        Surface::Plane { origin, u_axis, v_axis } => {
            points(&[*origin, *u_axis, *v_axis], budget)?;
            require(vector(*u_axis).cross(vector(*v_axis)).len() > 1e-12, "degenerate plane")
        }
        Surface::Spline { u_knots, v_knots, points: p } => grid(u_knots, v_knots, p, budget),
        Surface::RationalSpline { u_knots, v_knots, homogeneous: p } => {
            grid(u_knots, v_knots, p, budget)?;
            for row in p {
                weights(row)?;
            }
            Ok(())
        }
        Surface::Revolution { profile, origin, axis, columns, .. } => {
            points(&[*origin, *axis], budget)?;
            require((vector(*axis).len() - 1.0).abs() <= 1e-8, "revolution axis is not unit length")?;
            points(columns, budget)?;
            require(
                columns.iter().take(3).all(|c| c.get(3) == Some(&0.0)) && columns.get(3).and_then(|c| c.get(3)) == Some(&1.0),
                "non-affine transform",
            )?;
            let xyz = |i: usize| columns.get(i).map(|c| crate::Vec3::new(c[0], c[1], c[2])).ok_or(Invalid("transform"));
            require(xyz(0)?.dot(xyz(1)?.cross(xyz(2)?)).abs() > 1e-12, "singular transform")?;
            curve(profile, depth + 1, budget)
        }
    }
}

impl Brep {
    /// Validate portable structure without calling a kernel or evaluating geometry.
    /// Does not certify watertightness, surface coincidence, or authoritative state.
    pub fn validate(&self) -> Result<()> {
        require(self.version == VERSION, "unsupported version")?;
        require(!self.shells.is_empty() && self.shells.len() <= 1024, "shell count")?;
        let mut budget = Budget(0);
        for shell in &self.shells {
            points(&shell.vertices, &mut budget)?;
            budget.add(shell.edges.len())?;
            budget.add(shell.faces.len())?;
            require(!shell.faces.is_empty(), "empty shell")?;
            for edge in &shell.edges {
                require(edge.vertices.iter().all(|&v| (v as usize) < shell.vertices.len()), "edge vertices")?;
                curve(&edge.curve, 0, &mut budget)?;
            }
            for face in &shell.faces {
                require(!face.loops.is_empty(), "face loops")?;
                surface(&face.surface, 0, &mut budget)?;
                for wire in &face.loops {
                    require(!wire.is_empty(), "empty wire")?;
                    budget.add(wire.len())?;
                    let mut first = None;
                    let mut last = None;
                    for use_ in wire {
                        let edge = shell.edges.get(use_.edge as usize).ok_or(Invalid("edge reference"))?;
                        let [a, b] = edge.vertices;
                        let (start, end) = if use_.forward { (a, b) } else { (b, a) };
                        require(last.is_none_or(|v| v == start), "disconnected wire")?;
                        first.get_or_insert(start);
                        last = Some(end);
                    }
                    require(first == last, "open wire")?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn triangle() -> Brep {
        let vertices = vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let edges = (0..3)
            .map(|i| Edge { vertices: [i as u32, ((i + 1) % 3) as u32], curve: Curve::Line { start: vertices[i], end: vertices[(i + 1) % 3] } })
            .collect();
        Brep {
            version: VERSION,
            shells: vec![Shell {
                vertices,
                edges,
                faces: vec![Face {
                    loops: vec![(0..3).map(|edge| EdgeUse { edge, forward: true }).collect()],
                    surface: Surface::Plane { origin: [0.; 3], u_axis: [1., 0., 0.], v_axis: [0., 1., 0.] },
                    forward: true,
                }],
            }],
        }
    }
    #[test]
    fn portable_open_face_round_trip_without_a_kernel() {
        let body = triangle();
        body.validate().unwrap();
        let bytes = serde_json::to_vec(&body).unwrap();
        let restored: Brep = serde_json::from_slice(&bytes).unwrap();
        restored.validate().unwrap();
        assert_eq!(body, restored);
        assert!(!String::from_utf8(bytes).unwrap().contains("truck"));
    }
    #[test]
    fn topology_references_and_wire_orientation_are_checked() {
        let mut b = triangle();
        b.shells[0].faces[0].loops[0][0].edge = u32::MAX;
        assert!(b.validate().is_err());
        let mut b = triangle();
        b.shells[0].faces[0].loops[0][1].forward = false;
        assert!(b.validate().is_err());
        let mut b = triangle();
        b.shells[0].edges[0].vertices[0] = u32::MAX;
        assert!(b.validate().is_err());
        let mut b = triangle();
        b.version += 1;
        assert!(b.validate().is_err());
        let mut b = triangle();
        b.shells[0].vertices[0][0] = f64::NAN;
        assert!(b.validate().is_err());
    }
    #[test]
    fn closed_edge_can_share_its_endpoint_vertex() {
        let mut body = triangle();
        body.shells[0].vertices.truncate(1);
        body.shells[0].edges =
            vec![Edge { vertices: [0, 0], curve: Curve::Polyline { points: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 0.]] } }];
        body.shells[0].faces[0].loops = vec![vec![EdgeUse { edge: 0, forward: true }]];
        body.validate().unwrap();
    }

    #[test]
    fn spline_domains_weights_and_nesting_are_bounded() {
        let mut budget = Budget(0);
        assert!(knots(&[0., 0., 1., 1.], 2, &mut budget).is_ok());
        for bad in [vec![0., 0., 0., 0.], vec![0., 1., 0., 1.], vec![0., 0., f64::INFINITY, 1.]] {
            assert!(knots(&bad, 2, &mut Budget(0)).is_err());
        }
        let c = Curve::RationalSpline { knots: vec![0., 0., 1., 1.], homogeneous: vec![[0., 0., 0., 0.], [1., 0., 0., 1.]] };
        assert!(curve(&c, 0, &mut Budget(0)).is_err());
        let p = Surface::Plane { origin: [0.; 3], u_axis: [1., 0., 0.], v_axis: [2., 0., 0.] };
        assert!(surface(&p, 0, &mut Budget(0)).is_err());
        assert!(curve(&Curve::Line { start: [0.; 3], end: [1.; 3] }, MAX_DEPTH + 1, &mut Budget(0)).is_err());
        assert!(Budget(MAX_ITEMS).add(1).is_err());
    }
}
