use super::*;
use crate::{ConstraintKind, solve};

fn square(size: f64) -> Vec<LinkGeom> {
    let pts = [Vec2::new(-size, -size), Vec2::new(size, -size), Vec2::new(size, size), Vec2::new(-size, size)];
    (0..4).map(|i| LinkGeom::Line(pts[i], pts[(i + 1) % 4])).collect()
}

fn face() -> LinkSource {
    LinkSource::Face { body: "Case".into(), at: Vec3::ZERO }
}

fn ring(outer_first: bool, outer: f64, inner: f64) -> Vec<LinkGeom> {
    let (first, second) = if outer_first { (outer, inner) } else { (inner, outer) };
    square(first).into_iter().chain(square(second)).collect()
}

#[test]
fn saved_face_link_keeps_inner_ids_when_boundary_order_changes() {
    let mut sk = Sketch::new();
    // An older saved face may have enumerated its inner boundary first.
    let link = sk.add_link(LinkKind::Project, face(), &ring(false, 10.0, 8.0)).unwrap();
    let inner = sk.link_curves(&link)[0];
    let corner = match sk.curves[inner].kind {
        CurveKind::Line { a, .. } => a,
        _ => panic!(),
    };
    let free = sk.add_point(Vec2::new(-8.0, -8.0), None).unwrap();
    sk.add_constraint(ConstraintKind::Coincident { p: free, q: corner }, None).unwrap();
    let serialized = serde_json::to_string(&sk).unwrap();
    let mut sk: Sketch = serde_json::from_str(&serialized).unwrap();
    let before = sk.clone();
    assert!(!sk.update_link(&link, &ring(true, 10.0, 8.0)).unwrap());
    assert_eq!(sk, before, "canonical outer-first enumeration must not reinterpret saved ids");
    assert!(solve(&mut sk).ok());
    assert_eq!(sk.points[free].pos, Vec2::new(-8.0, -8.0));
}

#[test]
fn resized_and_reordered_face_keeps_constraints_on_the_same_boundary() {
    let mut sk = Sketch::new();
    let link = sk.add_link(LinkKind::Project, face(), &ring(false, 10.0, 8.0)).unwrap();
    let points = sk.link_points(&link);
    let original: Vec<Vec2> = points.iter().map(|p| sk.points[*p].pos).collect();
    let ids: Vec<String> = sk.link_curves(&link).iter().map(|c| sk.curves[*c].id.clone()).collect();
    let free = sk.add_point(Vec2::new(-8.0, -8.0), None).unwrap();
    sk.add_constraint(ConstraintKind::Coincident { p: free, q: points[0] }, None).unwrap();
    let fresh: Vec<LinkGeom> = ring(true, 10.0, 8.0)
        .into_iter()
        .map(|g| match g {
            LinkGeom::Line(a, b) => LinkGeom::Line(Vec2::new(b.x * 2.0, b.y * 3.0), Vec2::new(a.x * 2.0, a.y * 3.0)),
            _ => panic!(),
        })
        .collect();
    assert!(!sk.update_link(&link, &fresh).unwrap());
    for (p, old) in points.iter().zip(original) {
        assert_eq!(sk.points[*p].pos, Vec2::new(old.x * 2.0, old.y * 3.0));
    }
    assert_eq!(ids, sk.link_curves(&link).iter().map(|c| sk.curves[*c].id.clone()).collect::<Vec<_>>());
    assert!(solve(&mut sk).ok());
    assert!(sk.points[free].pos.dist(Vec2::new(-16.0, -24.0)) < 1e-7);
}

#[test]
fn ambiguous_face_refresh_preserves_last_geometry_and_marks_link_lost() {
    let mut sk = Sketch::new();
    let link = sk.add_link(LinkKind::Project, face(), &ring(true, 10.0, 9.9)).unwrap();
    let before = sk.clone();
    assert!(sk.update_link(&link, &ring(false, 10.0, 2.0)).is_err());
    assert_eq!(sk.points, before.points);
    assert_eq!(sk.curves, before.curves);
    assert!(sk.link(&link).unwrap().lost);
}

#[test]
fn large_face_permutation_is_bounded_and_preserves_geometry() {
    let mut sk = Sketch::new();
    let geom: Vec<LinkGeom> = (1..=129).flat_map(|i| square(i as f64)).collect();
    let link = sk.add_link(LinkKind::Project, face(), &geom).unwrap();
    assert!(!sk.update_link(&link, &geom).unwrap(), "ordered unchanged geometry needs no permutation scan");
    let before = sk.clone();
    let reversed: Vec<LinkGeom> = (1..=129).rev().flat_map(|i| square(i as f64)).collect();
    assert!(sk.update_link(&link, &reversed).is_err());
    assert_eq!(sk.points, before.points);
    assert_eq!(sk.curves, before.curves);
}

#[test]
fn geometric_refresh_updates_conic_and_ellipse_scalar_values() {
    let mut sk = Sketch::new();
    let geom = |rho, minor| {
        vec![
            LinkGeom::Conic { a: Vec2::new(1.0, 1.0), apex: Vec2::new(2.0, 3.0), b: Vec2::new(4.0, 1.0), rho },
            LinkGeom::Ellipse { c: Vec2::new(10.0, 10.0), major: Vec2::new(14.0, 10.0), minor },
        ]
    };
    let link = sk.add_link(LinkKind::Project, LinkSource::Origin, &geom(0.4, 2.0)).unwrap();
    assert!(!sk.update_link(&link, &geom(0.6, 3.0)).unwrap());
    assert!(matches!(sk.curves[0].kind, CurveKind::Conic { rho, .. } if rho == 0.6));
    assert!(matches!(sk.curves[1].kind, CurveKind::Ellipse { r, .. } if r == 3.0));
}

#[test]
fn many_curves_sharing_few_points_do_not_enter_permutation_matching() {
    let mut sk = Sketch::new();
    let geom = vec![LinkGeom::Line(Vec2::new(1.0, 1.0), Vec2::new(2.0, 2.0)); 513];
    let link = sk.add_link(LinkKind::Project, face(), &geom).unwrap();
    assert_eq!(sk.link_points(&link).len(), 2);
    let before = sk.clone();
    let shifted = vec![LinkGeom::Line(Vec2::new(3.0, 3.0), Vec2::new(4.0, 4.0)); 513];
    assert!(sk.update_link(&link, &shifted).is_err());
    assert_eq!(sk.points, before.points);
    assert_eq!(sk.curves, before.curves);
    assert!(sk.link(&link).unwrap().lost);
}

#[test]
fn concentric_face_resize_keeps_inner_outer_circle_identity() {
    let mut sk = Sketch::new();
    let link = sk.add_link(LinkKind::Project, face(), &[LinkGeom::Circle(Vec2::ZERO, 8.0), LinkGeom::Circle(Vec2::ZERO, 10.0)]).unwrap();
    let ids: Vec<String> = sk.link_curves(&link).iter().map(|i| sk.curves[*i].id.clone()).collect();
    assert!(!sk.update_link(&link, &[LinkGeom::Circle(Vec2::ZERO, 20.0), LinkGeom::Circle(Vec2::ZERO, 16.0)]).unwrap());
    assert!(matches!(sk.curves[0].kind, CurveKind::Circle { r, .. } if r == 16.0));
    assert!(matches!(sk.curves[1].kind, CurveKind::Circle { r, .. } if r == 20.0));
    assert_eq!(ids, sk.link_curves(&link).iter().map(|i| sk.curves[*i].id.clone()).collect::<Vec<_>>());
    // Coincident circles do not have a unique boundary identity.
    let before = sk.clone();
    assert!(sk.update_link(&link, &[LinkGeom::Circle(Vec2::ZERO, 18.0), LinkGeom::Circle(Vec2::ZERO, 18.0)]).is_err());
    assert_eq!(sk.points, before.points);
    assert_eq!(sk.curves, before.curves);
}

#[test]
fn resized_dense_rim_matches_arc_centres_by_role_before_proximity() {
    let mut sk = Sketch::new();
    let geometry = |half_width: f64, x: f64| {
        let mut out = square(half_width);
        let c = Vec2::new(x, -16.5);
        let left = c - Vec2::X * 3.0;
        let right = c + Vec2::X * 3.0;
        out.push(LinkGeom::Arc { c, a: right, b: left });
        out.push(LinkGeom::Arc { c, a: left, b: right });
        out
    };
    let link = sk.add_link(LinkKind::Project, face(), &geometry(38.0, -26.5)).unwrap();
    let arcs: Vec<usize> = sk.link_curves(&link).into_iter().filter(|i| matches!(sk.curves[*i].kind, CurveKind::Arc { .. })).collect();
    let ids: Vec<String> = arcs.iter().map(|i| sk.curves[*i].id.clone()).collect();
    // The old centre is spatially closer in normalized space to a fresh endpoint.
    // Both semicircles must still keep the same shared centre and logical endpoints.
    assert!(!sk.update_link(&link, &geometry(43.0, -31.5)).unwrap());
    for i in &arcs {
        let CurveKind::Arc { c, a, b } = sk.curves[*i].kind else { panic!() };
        assert_eq!(sk.points[c].pos, Vec2::new(-31.5, -16.5));
        assert!((sk.points[a].pos.dist(sk.points[c].pos) - 3.0).abs() < 1e-9);
        assert!((sk.points[b].pos.dist(sk.points[c].pos) - 3.0).abs() < 1e-9);
    }
    assert_eq!(ids, arcs.iter().map(|i| sk.curves[*i].id.clone()).collect::<Vec<_>>());
}
