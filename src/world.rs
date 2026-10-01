use crate::animation::Animation;
use crate::controls::Controls;
use crate::scene::Scene;

/// One explorable place: its blocks and lights, what moves in it, which
/// material is its portal, and where you appear when you arrive by portal.
pub struct World {
    pub scene: Scene,
    pub animation: Animation,
    pub portal_material: usize,
    /// Fresh camera controls for arriving here through the portal.
    pub arrival: fn() -> Controls,
}
