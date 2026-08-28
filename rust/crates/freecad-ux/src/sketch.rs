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
                // arc middle via bisector
                let mid_ang = arc.start_angle + (arc.end_angle - arc.start_angle) * 0.5;
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
pub fn mid_snap_threshold(len: f64) -> f64 {
    len * 0.05
}

/// Decide which external edges to import for a new sketch on face.
/// Fusion-like: import all boundary edges (no filtering), but cap at 200 to avoid STEP blowups.
pub fn face_edges_to_external(face: &FaceProj, max_edges: usize) -> Vec<String> {
    face.edges
        .iter()
        .take(max_edges)
        .map(|e| e.name.clone())
        .collect()
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
}
