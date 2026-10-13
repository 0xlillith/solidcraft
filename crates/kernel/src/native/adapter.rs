//! The truck adapter. Persisted data belongs to solidcraft_geom::brep, never truck.
use super::{Result, invalid};
use mt::{InnerSpace, Invertible, Transformed};
use solidcraft_geom::brep as b;
use truck_modeling as mt;
use truck_topology::compress as t;

type Packed = t::CompressedSolid<mt::Point3, mt::Curve, mt::Surface>;
fn point(p: mt::Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}
fn vector(p: mt::Vector3) -> [f64; 3] {
    [p.x, p.y, p.z]
}
fn vector4(p: mt::Vector4) -> [f64; 4] {
    [p.x, p.y, p.z, p.w]
}
fn p([x, y, z]: [f64; 3]) -> mt::Point3 {
    mt::Point3::new(x, y, z)
}
fn v([x, y, z]: [f64; 3]) -> mt::Vector3 {
    mt::Vector3::new(x, y, z)
}
fn v4([x, y, z, w]: [f64; 4]) -> mt::Vector4 {
    mt::Vector4::new(x, y, z, w)
}
fn index(i: usize) -> Result<u32> {
    u32::try_from(i).map_err(invalid)
}
fn depth(d: usize) -> Result<()> {
    if d > 16 { Err(invalid("geometry nesting")) } else { Ok(()) }
}

// Truck evaluates the full knot interval. The portable domain is the standard
// active interval, so only coincident (clamped) domains can cross this adapter.
fn clamped(knots: &[f64], count: usize) -> Result<()> {
    let degree = knots.len().checked_sub(count + 1).ok_or_else(|| invalid("spline degree"))?;
    if knots.first() != knots.get(degree) || knots.last() != knots.get(count) {
        return Err(invalid("unclamped spline domain unsupported by this adapter"));
    }
    Ok(())
}
fn compatible_curve(c: &b::Curve) -> Result<()> {
    match c {
        b::Curve::Spline { knots, points } => clamped(knots, points.len()),
        b::Curve::RationalSpline { knots, homogeneous } => clamped(knots, homogeneous.len()),
        b::Curve::Intersection { leader, surfaces, .. } => {
            compatible_curve(leader)?;
            for s in surfaces {
                compatible_surface(s)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
fn compatible_surface(s: &b::Surface) -> Result<()> {
    match s {
        b::Surface::Plane { origin, u_axis, v_axis } => {
            let plane = mt::Plane::new(p(*origin), p(*origin) + v(*u_axis), p(*origin) + v(*v_axis));
            if vector(plane.u_axis()) != *u_axis || vector(plane.v_axis()) != *v_axis {
                return Err(invalid("plane axes cannot be reconstructed exactly"));
            }
            Ok(())
        }
        b::Surface::Spline { u_knots, v_knots, points } => {
            clamped(u_knots, points.len())?;
            clamped(v_knots, points.first().map_or(0, Vec::len))
        }
        b::Surface::RationalSpline { u_knots, v_knots, homogeneous } => {
            clamped(u_knots, homogeneous.len())?;
            clamped(v_knots, homogeneous.first().map_or(0, Vec::len))
        }
        b::Surface::Revolution { axis, profile, .. } => {
            if vector(v(*axis).normalize()) != *axis {
                return Err(invalid("revolution axis cannot be reconstructed exactly"));
            }
            compatible_curve(profile)
        }
    }
}
fn compatible(body: &b::Brep) -> Result<()> {
    for shell in &body.shells {
        for edge in &shell.edges {
            compatible_curve(&edge.curve)?;
        }
        for face in &shell.faces {
            compatible_surface(&face.surface)?;
        }
    }
    Ok(())
}

pub(super) fn encode(solid: &mt::Solid) -> Result<b::Brep> {
    let mut shells = Vec::new();
    for shell in solid.compress().boundaries {
        let vertices = shell.vertices.into_iter().map(point).collect();
        let edges = shell
            .edges
            .into_iter()
            .map(|e| Ok(b::Edge { vertices: [index(e.vertices.0)?, index(e.vertices.1)?], curve: save_curve(&e.curve, 0)? }))
            .collect::<Result<_>>()?;
        let faces = shell
            .faces
            .into_iter()
            .map(|f| {
                Ok(b::Face {
                    loops: f
                        .boundaries
                        .into_iter()
                        .map(|w| w.into_iter().map(|e| Ok(b::EdgeUse { edge: index(e.index)?, forward: e.orientation })).collect())
                        .collect::<Result<_>>()?,
                    surface: save_surface(&f.surface, 0)?,
                    forward: f.orientation,
                })
            })
            .collect::<Result<_>>()?;
        shells.push(b::Shell { vertices, edges, faces });
    }
    let result = b::Brep { version: b::VERSION, shells };
    result.validate().map_err(invalid)?;
    compatible(&result)?;
    Ok(result)
}
pub(super) fn decode(body: b::Brep) -> Result<mt::Solid> {
    body.validate().map_err(invalid)?;
    compatible(&body)?;
    if body.shells.iter().flat_map(|s| &s.edges).any(|e| e.vertices[0] == e.vertices[1]) {
        return Err(invalid("closed single-vertex edges are unsupported by this adapter"));
    }
    let packed = Packed {
        boundaries: body
            .shells
            .into_iter()
            .map(|s| {
                Ok(t::CompressedShell {
                    vertices: s.vertices.into_iter().map(p).collect(),
                    edges: s
                        .edges
                        .into_iter()
                        .map(|e| Ok(t::CompressedEdge { vertices: (e.vertices[0] as usize, e.vertices[1] as usize), curve: load_curve(e.curve)? }))
                        .collect::<Result<_>>()?,
                    faces: s
                        .faces
                        .into_iter()
                        .map(|f| {
                            Ok(t::CompressedFace {
                                boundaries: f
                                    .loops
                                    .into_iter()
                                    .map(|w| {
                                        w.into_iter().map(|e| t::CompressedEdgeIndex { index: e.edge as usize, orientation: e.forward }).collect()
                                    })
                                    .collect(),
                                orientation: f.forward,
                                surface: load_surface(f.surface)?,
                            })
                        })
                        .collect::<Result<_>>()?,
                })
            })
            .collect::<Result<_>>()?,
    };
    let shells = packed.boundaries.into_iter().map(mt::Shell::extract).collect::<std::result::Result<Vec<_>, _>>().map_err(invalid)?;
    if shells.iter().any(|s| s.shell_condition() == mt::ShellCondition::Irregular) {
        return Err(invalid("non-manifold shell"));
    }
    Ok(mt::Solid::new_unchecked(shells))
}
fn save_curve(c: &mt::Curve, d: usize) -> Result<b::Curve> {
    depth(d)?;
    Ok(match c {
        mt::Curve::Line(c) => b::Curve::Line { start: point(c.0), end: point(c.1) },
        mt::Curve::BSplineCurve(c) => {
            b::Curve::Spline { knots: c.knot_vec().iter().copied().collect(), points: c.control_points().iter().copied().map(point).collect() }
        }
        mt::Curve::NurbsCurve(c) => b::Curve::RationalSpline {
            knots: c.knot_vec().iter().copied().collect(),
            homogeneous: c.control_points().iter().copied().map(vector4).collect(),
        },
        mt::Curve::IntersectionCurve(c) => b::Curve::Intersection {
            leader: Box::new(match c.leader() {
                mt::Leader::Polyline(c) => b::Curve::Polyline { points: c.iter().copied().map(point).collect() },
                mt::Leader::BSpline(c) => b::Curve::Spline {
                    knots: c.knot_vec().iter().copied().collect(),
                    points: c.control_points().iter().copied().map(point).collect(),
                },
            }),
            surfaces: [Box::new(save_surface(c.surface0(), d + 1)?), Box::new(save_surface(c.surface1(), d + 1)?)],
            tolerance: c.tolerance(),
        },
    })
}
fn save_surface(s: &mt::Surface, d: usize) -> Result<b::Surface> {
    depth(d)?;
    Ok(match s {
        mt::Surface::Plane(s) => b::Surface::Plane { origin: point(s.origin()), u_axis: vector(s.u_axis()), v_axis: vector(s.v_axis()) },
        mt::Surface::BSplineSurface(s) => b::Surface::Spline {
            u_knots: s.uknot_vec().iter().copied().collect(),
            v_knots: s.vknot_vec().iter().copied().collect(),
            points: s.control_points().iter().map(|r| r.iter().copied().map(point).collect()).collect(),
        },
        mt::Surface::NurbsSurface(s) => b::Surface::RationalSpline {
            u_knots: s.uknot_vec().iter().copied().collect(),
            v_knots: s.vknot_vec().iter().copied().collect(),
            homogeneous: s.control_points().iter().map(|r| r.iter().copied().map(vector4).collect()).collect(),
        },
        mt::Surface::RevolutedCurve(s) => {
            let m = s.transform();
            b::Surface::Revolution {
                profile: Box::new(save_curve(s.entity().entity_curve(), d + 1)?),
                origin: point(s.entity().origin()),
                axis: vector(s.entity().axis()),
                columns: [vector4(m.x), vector4(m.y), vector4(m.z), vector4(m.w)],
                forward: s.orientation(),
            }
        }
    })
}
fn load_curve(c: b::Curve) -> Result<mt::Curve> {
    Ok(match c {
        b::Curve::Line { start, end } => mt::Curve::Line(mt::Line(p(start), p(end))),
        b::Curve::Spline { knots, points } => {
            mt::Curve::BSplineCurve(mt::BSplineCurve::try_new(knots.into(), points.into_iter().map(p).collect()).map_err(invalid)?)
        }
        b::Curve::RationalSpline { knots, homogeneous } => mt::Curve::NurbsCurve(mt::NurbsCurve::new(
            mt::BSplineCurve::try_new(knots.into(), homogeneous.into_iter().map(v4).collect()).map_err(invalid)?,
        )),
        b::Curve::Polyline { .. } => return Err(invalid("polyline supported only as intersection leader")),
        b::Curve::Intersection { leader, surfaces, tolerance } => {
            let leader = match *leader {
                b::Curve::Polyline { points } => mt::Leader::Polyline(points.into_iter().map(p).collect()),
                b::Curve::Spline { knots, points } => {
                    mt::Leader::BSpline(mt::BSplineCurve::try_new(knots.into(), points.into_iter().map(p).collect()).map_err(invalid)?)
                }
                _ => return Err(invalid("unsupported intersection leader")),
            };
            let [a, b] = surfaces;
            mt::Curve::IntersectionCurve(mt::IntersectionCurve::new_unchecked(
                Box::new(load_surface(*a)?),
                Box::new(load_surface(*b)?),
                leader,
                tolerance,
            ))
        }
    })
}
fn load_surface(s: b::Surface) -> Result<mt::Surface> {
    Ok(match s {
        b::Surface::Plane { origin, u_axis, v_axis } => mt::Surface::Plane(mt::Plane::new(p(origin), p(origin) + v(u_axis), p(origin) + v(v_axis))),
        b::Surface::Spline { u_knots, v_knots, points } => mt::Surface::BSplineSurface(
            mt::BSplineSurface::try_new((u_knots.into(), v_knots.into()), points.into_iter().map(|r| r.into_iter().map(p).collect()).collect())
                .map_err(invalid)?,
        ),
        b::Surface::RationalSpline { u_knots, v_knots, homogeneous } => mt::Surface::NurbsSurface(mt::NurbsSurface::new(
            mt::BSplineSurface::try_new((u_knots.into(), v_knots.into()), homogeneous.into_iter().map(|r| r.into_iter().map(v4).collect()).collect())
                .map_err(invalid)?,
        )),
        b::Surface::Revolution { profile, origin, axis, columns, forward } => {
            let [a, b, c, d] = columns;
            let mut s = mt::Processor::new(mt::RevolutedCurve::by_revolution(load_curve(*profile)?, p(origin), v(axis)));
            s.transform_by(mt::Matrix4::from_cols(v4(a), v4(b), v4(c), v4(d)));
            if !forward {
                s.invert();
            }
            mt::Surface::RevolutedCurve(s)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_domain_or_axis_changes_instead_of_approximating() {
        let curve = b::Curve::Spline { knots: vec![0., 1., 2., 3.], points: vec![[0.; 3], [1.; 3]] };
        assert!(compatible_curve(&curve).is_err());
        let plane = b::Surface::Plane { origin: [1e12; 3], u_axis: [1e-6, 0., 0.], v_axis: [0., 1., 0.] };
        assert!(compatible_surface(&plane).is_err());
        assert!(clamped(&[0., 0., 1., 1.], 2).is_ok());
    }
}
