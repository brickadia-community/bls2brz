//! Native Brickadia spawn-marker components used by converted marker bricks.

use brdb::assets::LiteralComponent;
use brdb::AsBrdbValue;

/// Let players spawn on this brick using Brickadia's normal spawn-point
/// behavior, including gravity alignment.
pub fn spawn_point_component() -> LiteralComponent {
    LiteralComponent::new("Component_SpawnPoint").with_data([
        (
            "bRotatePlayerGravityOnSpawn",
            Box::new(true) as Box<dyn AsBrdbValue>,
        ),
        ("bEnable", Box::new(true)),
    ])
}

/// Make this brick a Brickadia checkpoint with the normal gravity alignment.
pub fn checkpoint_component() -> LiteralComponent {
    LiteralComponent::new("Component_CheckPoint")
        .with_data([(
            "bRotatePlayerGravityOnSpawn",
            Box::new(true) as Box<dyn AsBrdbValue>,
        )])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_native_marker_component_types() {
        assert_eq!(spawn_point_component().component_name.as_ref(), "Component_SpawnPoint");
        assert_eq!(checkpoint_component().component_name.as_ref(), "Component_CheckPoint");
    }
}
