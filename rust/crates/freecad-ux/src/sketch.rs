//! Sketch smart binding — mirrors SketchWorkflow + SnapManager.
//! Handles face projection and mid/endpoint candidates.

/// 2D projected edge (already in sketch plane)
#[derive(Debug, Clone)]
pub struct EdgeProj {
    /// Global edge name like "Edge3"
    pub name: String,
    /// 2D endpoints in sketch coords
    pub start: [f64; 2],
    pub end: [f64; 2],
    /// If arc: center+radius+angles, else None for line
    pub arc: Option<ArcData>,
}

#[derive(Debug, Clone, Copy)]
pub struct ArcData {
    pub center: [f64; 2],
    pub radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}

/// Candidate for snapping (midpoint, endpoint, center)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapCandidate {
    pub pos: [f64; 2],
    pub kind: SnapKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    Endpoint,
    Midpoint,
    Center,
}

/// A face's projected edges ready for external import
#[derive(Debug, Clone)]
pub struct FaceProj {
    pub face_name: String, // "Face3"
    pub edges: Vec<EdgeProj>,
}

impl FaceProj {
    /// Build snap candidates for this face (endpoints + midpoints)
    pub fn snap_candidates(&self) -> Vec<SnapCandidate> {
        let mut out = Vec::new();
        for e in &self.edges {
            out.push(SnapCandidate {
                pos: e.start,
                kind: SnapKind::Endpoint,
            });
            out.push(SnapCandidate {
                pos: e.end,
                kind: SnapKind::Endpoint,
            });
            let mid = if let Some(arc) = e.arc {
                // arc middle via bisector — preserve sweep via rem_euclid for reflex/crossing-zero arcs
                let sweep =
                    (arc.end_angle - arc.start_angle).rem_euclid(2.0 * std::f64::consts::PI);
                let mid_ang = arc.start_angle + sweep * 0.5;
                [
                    arc.center[0] + arc.radius * mid_ang.cos(),
                    arc.center[1] + arc.radius * mid_ang.sin(),
                ]
            } else {
                [(e.start[0] + e.end[0]) * 0.5, (e.start[1] + e.end[1]) * 0.5]
            };
            out.push(SnapCandidate {
                pos: mid,
                kind: SnapKind::Midpoint,
            });
            if let Some(arc) = e.arc {
                out.push(SnapCandidate {
                    pos: arc.center,
                    kind: SnapKind::Center,
                });
            }
        }
        out
    }
}

/// Threshold for mid snap: 5% of length (line) or 10% of sweep (arc)
/// For lines: 5% of length. For arcs: use sweep*0.10 via snap_to_arc_middle.
pub fn mid_snap_threshold(len: f64) -> f64 {
    if !len.is_finite() || len <= 0.0 {
        return 0.0;
    }
    len * 0.05
}

/// Decide which external edges to import for a new sketch on face.
/// Fusion-like: import all boundary edges (no filtering). Caller passes cap
/// (80 for auto-import policy in C++ tryAutoImportFaceEdges, 200 for manual bulk).
pub fn face_edges_to_external(face: &FaceProj, max_edges: usize) -> Vec<String> {
    face.edges
        .iter()
        .take(max_edges)
        .map(|e| e.name.clone())
        .collect()
}

/// Smart external policy — mirrors C++ tryAutoImportFaceEdges cap (80 edges)
/// Returns false for >80 edges to avoid STEP blowups (separate from 200 hard cap).
pub fn should_auto_import(face: &FaceProj) -> bool {
    face.edges.len() <= 80
}

/// Nearest snap candidate for a cursor pos (Fusion-style magnetic)
pub fn nearest_snap(face: &FaceProj, cursor: [f64; 2], max_dist: f64) -> Option<SnapCandidate> {
    let mut best: Option<(f64, SnapCandidate)> = None;
    for c in face.snap_candidates() {
        let d = ((c.pos[0] - cursor[0]).powi(2) + (c.pos[1] - cursor[1]).powi(2)).sqrt();
        if d <= max_dist {
            match best {
                None => best = Some((d, c)),
                Some((bd, _)) if d < bd => best = Some((d, c)),
                _ => {}
            }
        }
    }
    best.map(|(_, c)| c)
}

/// Ghost snap — find edge name to lazily import when cursor snaps to face
/// without prior external. Returns edge name if candidate belongs to that edge.
/// Reuses snap_candidates() mapping to avoid duplicating midpoint math.
pub fn ghost_edge_for_snap(face: &FaceProj, cursor: [f64; 2], max_dist: f64) -> Option<String> {
    let snap = nearest_snap(face, cursor, max_dist)?;
    // Reuse canonical candidates with edge index to avoid trig duplication.
    let candidates = face.snap_candidates();
    // snap_candidates order per edge: start Endpoint, end Endpoint, Midpoint, [Center]
    let mut idx = 0usize;
    for e in &face.edges {
        let per_edge = if e.arc.is_some() { 4 } else { 3 };
        for k in 0..per_edge {
            let c = candidates[idx + k];
            if c.kind == snap.kind
                && (c.pos[0] - snap.pos[0]).abs() < 1e-6
                && (c.pos[1] - snap.pos[1]).abs() < 1e-6
            {
                return Some(e.name.clone());
            }
        }
        idx += per_edge;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_face() -> FaceProj {
        FaceProj {
            face_name: "Face3".into(),
            edges: vec![
                EdgeProj {
                    name: "Edge1".into(),
                    start: [0.0, 0.0],
                    end: [10.0, 0.0],
                    arc: None,
                },
                EdgeProj {
                    name: "Edge2".into(),
                    start: [10.0, 0.0],
                    end: [10.0, 10.0],
                    arc: None,
                },
                EdgeProj {
                    name: "Edge3".into(),
                    start: [10.0, 10.0],
                    end: [0.0, 10.0],
                    arc: None,
                },
                EdgeProj {
                    name: "Edge4".into(),
                    start: [0.0, 10.0],
                    end: [0.0, 0.0],
                    arc: None,
                },
            ],
        }
    }

    #[test]
    fn snap_candidates_rect() {
        let face = rect_face();
        let c = face.snap_candidates();
        // 4 edges * (2 endpoints +1 mid) =12
        assert_eq!(c.len(), 12);
        // first edge mid should be 5,0
        assert!(
            c.iter()
                .any(|sc| sc.pos == [5.0, 0.0] && sc.kind == SnapKind::Midpoint)
        );
    }

    #[test]
    fn face_edges_to_external_caps() {
        let face = rect_face();
        let names = face_edges_to_external(&face, 2);
        assert_eq!(names, vec!["Edge1", "Edge2"]);
        let all = face_edges_to_external(&face, 100);
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn mid_threshold() {
        assert_eq!(mid_snap_threshold(10.0), 0.5);
        assert_eq!(mid_snap_threshold(0.0), 0.0);
    }

    #[test]
    fn arc_face_mid() {
        let face = FaceProj {
            face_name: "Face1".into(),
            edges: vec![EdgeProj {
                name: "Edge1".into(),
                start: [10.0, 0.0],
                end: [0.0, 10.0],
                arc: Some(ArcData {
                    center: [0.0, 0.0],
                    radius: 10.0,
                    start_angle: 0.0,
                    end_angle: std::f64::consts::FRAC_PI_2,
                }),
            }],
        };
        let c = face.snap_candidates();
        // 2 endpoints + mid + center =4
        assert_eq!(c.len(), 4);
        assert!(c.iter().any(|sc| sc.kind == SnapKind::Center));
    }

    #[test]
    fn should_auto_import_cap() {
        let mut face = rect_face();
        assert!(should_auto_import(&face));
        face.edges.resize(81, face.edges[0].clone());
        assert!(!should_auto_import(&face));
    }

    #[test]
    fn nearest_snap_picks_mid() {
        let face = rect_face();
        // cursor near (5,0) midpoint
        let n = nearest_snap(&face, [5.1, 0.1], 1.0).unwrap();
        assert_eq!(n.kind, SnapKind::Midpoint);
        assert_eq!(n.pos, [5.0, 0.0]);
        assert!(nearest_snap(&face, [50.0, 50.0], 1.0).is_none());
    }

    #[test]
    fn ghost_edge_resolves() {
        let face = rect_face();
        let e = ghost_edge_for_snap(&face, [5.1, 0.1], 1.0).unwrap();
        assert_eq!(e, "Edge1");
        assert!(ghost_edge_for_snap(&face, [50.0, 50.0], 1.0).is_none());
    }
}
