//! Cube map math: which face of a big cube around the world a direction points
//! at, and where on that face.
//!
//! Shared by the renderer (direction → face, u, v) and the sky-painting tool
//! (face, u, v → direction), so both always agree. It uses plain numbers
//! instead of Vec3 so the tool can include this file on its own.
//!
//! Faces, in this order: +X, -X, +Y (up), -Y (down), +Z, -Z.
//! On each face, u goes left → right and v goes top → bottom as seen from
//! INSIDE the cube looking at that face, with "up" = +Y on the four side
//! faces. So a painter can open a face image and it looks the right way up.

pub const FACE_NAMES: [&str; 6] = ["sky_px", "sky_nx", "sky_py", "sky_ny", "sky_pz", "sky_nz"];

/// Which face a direction points at, and the (u, v) on it, each 0..1.
///
/// The face is decided by the direction's biggest component: (0.2, 0.9, -0.3)
/// points mostly up, so it hits the +Y face. Dividing the other two components
/// by that biggest one gives a position from -1 to +1 across the face, which
/// we then map to 0..1.
pub fn face_and_uv(x: f32, y: f32, z: f32) -> (usize, f32, f32) {
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    // (face, sideways coordinate, downward coordinate, biggest component)
    let (face, across, down, major) = if ax >= ay && ax >= az {
        if x > 0.0 { (0, z, -y, ax) } else { (1, -z, -y, ax) }
    } else if ay >= az {
        if y > 0.0 { (2, x, z, ay) } else { (3, x, -z, ay) }
    } else if z > 0.0 {
        (4, -x, -y, az)
    } else {
        (5, x, -y, az)
    };
    (face, (across / major + 1.0) * 0.5, (down / major + 1.0) * 0.5)
}

/// The reverse: the direction (not unit length) through point (u, v) of `face`.
/// Used by the sky-painting tool and the tests; the renderer only needs face_and_uv.
#[allow(dead_code)]
pub fn direction_for(face: usize, u: f32, v: f32) -> (f32, f32, f32) {
    let (across, down) = (2.0 * u - 1.0, 2.0 * v - 1.0);
    match face {
        0 => (1.0, -down, across),
        1 => (-1.0, -down, -across),
        2 => (across, 1.0, down),
        3 => (across, -1.0, -down),
        4 => (-across, -down, 1.0),
        _ => (across, -down, -1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_axis_hits_the_center_of_its_face() {
        let axes = [(1.0, 0.0, 0.0), (-1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, -1.0, 0.0), (0.0, 0.0, 1.0), (0.0, 0.0, -1.0)];
        for (expected_face, (x, y, z)) in axes.into_iter().enumerate() {
            let (face, u, v) = face_and_uv(x, y, z);
            assert_eq!(face, expected_face);
            assert!((u - 0.5).abs() < 1e-6 && (v - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn face_to_direction_and_back_round_trips_on_every_face() {
        for face in 0..6 {
            for &(u, v) in &[(0.1, 0.2), (0.5, 0.5), (0.9, 0.3), (0.25, 0.95)] {
                let (x, y, z) = direction_for(face, u, v);
                let (back_face, back_u, back_v) = face_and_uv(x, y, z);
                assert_eq!(back_face, face);
                assert!((back_u - u).abs() < 1e-5 && (back_v - v).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn side_faces_have_the_sky_at_the_top() {
        // A direction slightly ABOVE the horizon must land in the top half (v < 0.5).
        for (x, z) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let (_, _, v) = face_and_uv(x, 0.3, z);
            assert!(v < 0.5);
        }
    }

    #[test]
    fn neighboring_faces_meet_at_the_same_edge() {
        // The right edge of the -Z face (u = 1) and the left edge of the +X face
        // (u = 0) must be the same line in the world: no gap, no jump.
        let (x1, y1, z1) = direction_for(5, 1.0, 0.3);
        let (x2, y2, z2) = direction_for(0, 0.0, 0.3);
        assert!((x1 - x2).abs() < 1e-6 && (y1 - y2).abs() < 1e-6 && (z1 - z2).abs() < 1e-6);
    }
}
