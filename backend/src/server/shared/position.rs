//! Generic position handling for ordered entities.
//!
//! Provides traits and utilities for entities that have a position field
//! for ordering within a parent collection (e.g., interfaces within a host,
//! services within a host, bindings within a group).
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::server::shared::position::{Positioned, validate_input_positions};
//!
//! impl Positioned for Interface {
//!     fn position(&self) -> i32 { self.base.position }
//!     fn set_position(&mut self, p: i32) { self.base.position = p; }
//!     fn id(&self) -> Uuid { self.id }
//!     fn entity_name() -> &'static str { "interface" }
//! }
//!
//! // Validate positions in a create request
//! validate_input_positions(&interface_inputs, "interface")?;
//! ```

use crate::server::shared::types::api::ApiError;
use uuid::Uuid;

// =============================================================================
// TRAITS
// =============================================================================

/// Trait for entities that have a position for ordering.
/// Implement this for any entity that needs position-based ordering within a parent.
pub trait Positioned {
    /// Get the entity's current position
    fn position(&self) -> i32;

    /// Set the entity's position
    fn set_position(&mut self, position: i32);

    /// Get the entity's unique ID (for conflict detection during updates)
    fn id(&self) -> Uuid;

    /// Human-readable name for error messages (e.g., "interface", "service")
    fn entity_name() -> &'static str;
}

/// Trait for input types (create/update requests) that carry an optional position field.
/// Used for ServiceInput, InterfaceInput in host create/update requests.
pub trait PositionedInput {
    /// Get the position from the input (None if omitted)
    fn position(&self) -> Option<i32>;

    /// Set the position on the input (used during resolution)
    fn set_position(&mut self, position: i32);

    /// Get the entity ID (used to look up existing entities)
    fn id(&self) -> Uuid;
}

// =============================================================================
// VALIDATION FUNCTIONS
// =============================================================================

/// Validates that positions are sequential (0, 1, 2, ..., n-1) with no gaps or duplicates.
///
/// Positions can be provided in any order; they will be sorted before validation.
///
/// # Arguments
/// * `positions` - Slice of position values to validate
/// * `entity_name` - Name for error messages (e.g., "interface", "service")
///
/// # Returns
/// * `Ok(())` if positions are valid (sequential from 0, no duplicates)
/// * `Err(ApiError)` with descriptive message if invalid
///
/// # Examples
/// ```rust,ignore
/// // Valid: sequential from 0
/// validate_sequential_positions(&[0, 1, 2], "interface")?; // Ok
/// validate_sequential_positions(&[2, 0, 1], "interface")?; // Ok (unsorted input)
/// validate_sequential_positions(&[], "interface")?;        // Ok (empty)
///
/// // Invalid
/// validate_sequential_positions(&[0, 1, 1], "interface")?; // Err: duplicate
/// validate_sequential_positions(&[0, 2, 3], "interface")?; // Err: gap (missing 1)
/// validate_sequential_positions(&[1, 2, 3], "interface")?; // Err: doesn't start at 0
/// ```
pub fn validate_sequential_positions(positions: &[i32], entity_name: &str) -> Result<(), ApiError> {
    if positions.is_empty() {
        return Ok(());
    }

    let mut sorted = positions.to_vec();
    sorted.sort();

    // Check for duplicates
    for window in sorted.windows(2) {
        if window[0] == window[1] {
            return Err(ApiError::bad_request(&format!(
                "Duplicate {} position: {}. Each {} must have a unique position.",
                entity_name, window[0], entity_name
            )));
        }
    }

    // Check sequential starting from 0
    for (expected, actual) in sorted.iter().enumerate() {
        if *actual != expected as i32 {
            return Err(ApiError::bad_request(&format!(
                "{} positions must be sequential starting from 0. \
                 Expected position {} but found {}. Positions should be: 0, 1, 2, ..., {}",
                capitalize(entity_name),
                expected,
                actual,
                positions.len() - 1
            )));
        }
    }

    Ok(())
}

/// Validates positions from a collection of positioned inputs where all positions are specified.
///
/// Convenience wrapper around `validate_sequential_positions` that extracts
/// positions from input types implementing `PositionedInput`.
/// All inputs must have Some(position); use `resolve_and_validate_input_positions` for optional positions.
pub fn validate_input_positions<T: PositionedInput>(
    inputs: &[T],
    entity_name: &str,
) -> Result<(), ApiError> {
    let positions: Vec<i32> = inputs.iter().filter_map(|i| i.position()).collect();
    validate_sequential_positions(&positions, entity_name)
}

/// Resolves optional positions on inputs, then validates.
///
/// Handles three cases:
/// - All positions are `None`: Auto-assign positions (existing items keep their positions,
///   new items are appended to the end)
/// - All positions are `Some`: Validate that they are sequential (0, 1, 2, ...)
/// - Mixed: Returns an error (must be all specified or all omitted)
///
/// # Arguments
/// * `inputs` - Mutable slice of inputs to resolve positions on
/// * `existing` - Slice of existing entities (used to preserve positions for updates)
/// * `entity_name` - Name for error messages (e.g., "interface", "service")
///
/// # Returns
/// * `Ok(())` if positions were resolved/validated successfully
/// * `Err(ApiError)` if validation fails
pub fn resolve_and_validate_input_positions<T: PositionedInput, E: Positioned>(
    inputs: &mut [T],
    existing: &[E],
    entity_name: &str,
) -> Result<(), ApiError> {
    use std::collections::HashMap;

    if inputs.is_empty() {
        return Ok(());
    }

    // Count how many have explicit positions
    let specified_count = inputs.iter().filter(|i| i.position().is_some()).count();

    // Check for mixed case (some specified, some not)
    if specified_count > 0 && specified_count < inputs.len() {
        return Err(ApiError::bad_request(&format!(
            "{} positions must be all specified or all omitted. \
             Found {} with positions and {} without.",
            capitalize(entity_name),
            specified_count,
            inputs.len() - specified_count
        )));
    }

    if specified_count == inputs.len() {
        // All specified - validate sequential
        let positions: Vec<i32> = inputs.iter().filter_map(|i| i.position()).collect();
        return validate_sequential_positions(&positions, entity_name);
    }

    // All omitted - resolve automatically
    // Build lookup of existing entity positions by ID
    let existing_by_id: HashMap<Uuid, i32> =
        existing.iter().map(|e| (e.id(), e.position())).collect();

    // Next position for new items starts after all existing items
    let mut next_pos = existing.len() as i32;

    for input in inputs.iter_mut() {
        if let Some(&pos) = existing_by_id.get(&input.id()) {
            // Existing item: preserve its current position
            input.set_position(pos);
        } else {
            // New item: append to end
            input.set_position(next_pos);
            next_pos += 1;
        }
    }

    Ok(())
}

/// Validates positions from a collection of positioned entities.
///
/// Convenience wrapper around `validate_sequential_positions` that extracts
/// positions from entity types implementing `Positioned`.
pub fn validate_entity_positions<T: Positioned>(entities: &[T]) -> Result<(), ApiError> {
    let positions: Vec<i32> = entities.iter().map(|e| e.position()).collect();
    validate_sequential_positions(&positions, T::entity_name())
}

/// Validates a single position is within valid range.
///
/// # Arguments
/// * `new_position` - The proposed new position
/// * `count` - Total number of entities (valid range is 0 to count-1)
/// * `entity_name` - Name for error messages
pub fn validate_position_range(
    new_position: i32,
    count: usize,
    entity_name: &str,
) -> Result<(), ApiError> {
    if count == 0 {
        return Err(ApiError::bad_request(&format!(
            "Cannot set {} position: no {}s exist.",
            entity_name, entity_name
        )));
    }

    let max_position = (count as i32) - 1;

    if new_position < 0 || new_position > max_position {
        return Err(ApiError::bad_request(&format!(
            "{} position {} is out of range. Valid positions are 0 to {}.",
            capitalize(entity_name),
            new_position,
            max_position
        )));
    }

    Ok(())
}

/// Validates that a position doesn't conflict with existing entities (excluding self).
///
/// Use this when updating an entity's position to ensure no other entity
/// already occupies that position.
///
/// # Arguments
/// * `new_position` - The proposed position
/// * `exclude_id` - ID of entity being updated (excluded from conflict check)
/// * `existing` - Slice of existing positioned entities
pub fn validate_no_position_conflict<T: Positioned>(
    new_position: i32,
    exclude_id: Option<Uuid>,
    existing: &[T],
) -> Result<(), ApiError> {
    let conflict = existing
        .iter()
        .any(|e| e.position() == new_position && Some(e.id()) != exclude_id);

    if conflict {
        return Err(ApiError::bad_request(&format!(
            "{} position {} is already used by another {}.",
            capitalize(T::entity_name()),
            new_position,
            T::entity_name()
        )));
    }

    Ok(())
}

// =============================================================================
// POSITION OPERATIONS
// =============================================================================

/// Renumbers entities to ensure sequential positions (0, 1, 2, ...).
///
/// Entities are sorted by their current position, then assigned new sequential
/// positions starting from 0. Useful after deletions to close gaps.
///
/// # Returns
/// * `true` if any positions were changed
/// * `false` if positions were already sequential (no changes needed)
pub fn renumber_positions<T: Positioned>(entities: &mut [T]) -> bool {
    if entities.is_empty() {
        return false;
    }

    // Sort by current position first
    entities.sort_by_key(|e| e.position());

    let mut changed = false;
    for (i, entity) in entities.iter_mut().enumerate() {
        let expected = i as i32;
        if entity.position() != expected {
            entity.set_position(expected);
            changed = true;
        }
    }

    changed
}

/// Gets the next available position for a new entity.
///
/// Returns the count of existing entities, which is the next sequential position.
pub fn next_position<T>(existing: &[T]) -> i32 {
    existing.len() as i32
}

/// Reorders entities by moving one from `from_position` to `to_position`.
///
/// All entities between the two positions are shifted accordingly:
/// - Moving down (from < to): entities in (from, to] shift up by 1
/// - Moving up (from > to): entities in [to, from) shift down by 1
///
/// # Arguments
/// * `entities` - Mutable slice of entities to reorder
/// * `from_position` - Current position of the entity to move
/// * `to_position` - Target position for the entity
///
/// # Returns
/// * `true` if reorder was performed
/// * `false` if positions are equal or out of bounds
pub fn reorder_positions<T: Positioned>(
    entities: &mut [T],
    from_position: i32,
    to_position: i32,
) -> bool {
    if from_position == to_position {
        return false;
    }

    let count = entities.len() as i32;
    if from_position < 0 || from_position >= count || to_position < 0 || to_position >= count {
        return false;
    }

    // Sort by position to ensure consistent ordering
    entities.sort_by_key(|e| e.position());

    if from_position < to_position {
        // Moving down: entity at from goes to to, items between shift up
        for entity in entities.iter_mut() {
            let pos = entity.position();
            if pos == from_position {
                entity.set_position(to_position);
            } else if pos > from_position && pos <= to_position {
                entity.set_position(pos - 1);
            }
        }
    } else {
        // Moving up: entity at from goes to to, items between shift down
        for entity in entities.iter_mut() {
            let pos = entity.position();
            if pos == from_position {
                entity.set_position(to_position);
            } else if pos >= to_position && pos < from_position {
                entity.set_position(pos + 1);
            }
        }
    }

    true
}

// =============================================================================
// HELPERS
// =============================================================================

/// Capitalizes the first letter of a string.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestEntity {
        id: Uuid,
        position: i32,
    }

    impl TestEntity {
        fn new(position: i32) -> Self {
            Self {
                id: Uuid::new_v4(),
                position,
            }
        }
    }

    impl Positioned for TestEntity {
        fn position(&self) -> i32 {
            self.position
        }
        fn set_position(&mut self, p: i32) {
            self.position = p;
        }
        fn id(&self) -> Uuid {
            self.id
        }
        fn entity_name() -> &'static str {
            "test entity"
        }
    }

    #[derive(Clone, Debug)]
    struct TestInput {
        id: Uuid,
        position: Option<i32>,
    }

    impl TestInput {
        fn new(position: Option<i32>) -> Self {
            Self {
                id: Uuid::new_v4(),
                position,
            }
        }

        fn with_id(id: Uuid, position: Option<i32>) -> Self {
            Self { id, position }
        }
    }

    impl PositionedInput for TestInput {
        fn position(&self) -> Option<i32> {
            self.position
        }

        fn set_position(&mut self, position: i32) {
            self.position = Some(position);
        }

        fn id(&self) -> Uuid {
            self.id
        }
    }

    // =========================================================================
    // validate_sequential_positions tests
    // =========================================================================

    #[test]
    fn test_validate_sequential_empty() {
        assert!(validate_sequential_positions(&[], "test").is_ok());
    }

    #[test]
    fn test_validate_sequential_single() {
        assert!(validate_sequential_positions(&[0], "test").is_ok());
    }

    #[test]
    fn test_validate_sequential_ordered() {
        assert!(validate_sequential_positions(&[0, 1, 2], "test").is_ok());
        assert!(validate_sequential_positions(&[0, 1, 2, 3, 4], "test").is_ok());
    }

    #[test]
    fn test_validate_sequential_unordered_valid() {
        assert!(validate_sequential_positions(&[2, 0, 1], "test").is_ok());
        assert!(validate_sequential_positions(&[3, 1, 0, 2], "test").is_ok());
    }

    #[test]
    fn test_validate_sequential_duplicates() {
        let result = validate_sequential_positions(&[0, 1, 1], "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Duplicate"));
    }

    #[test]
    fn test_validate_sequential_gap() {
        let result = validate_sequential_positions(&[0, 2, 3], "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("sequential"));
    }

    #[test]
    fn test_validate_sequential_not_starting_at_zero() {
        let result = validate_sequential_positions(&[1, 2, 3], "test");
        assert!(result.is_err());
    }

    // =========================================================================
    // validate_input_positions tests
    // =========================================================================

    #[test]
    fn test_validate_input_positions() {
        let inputs = vec![
            TestInput::new(Some(0)),
            TestInput::new(Some(1)),
            TestInput::new(Some(2)),
        ];
        assert!(validate_input_positions(&inputs, "test").is_ok());
    }

    // =========================================================================
    // resolve_and_validate_input_positions tests
    // =========================================================================

    #[test]
    fn test_resolve_all_omitted_empty() {
        let mut inputs: Vec<TestInput> = vec![];
        let existing: Vec<TestEntity> = vec![];
        assert!(resolve_and_validate_input_positions(&mut inputs, &existing, "test").is_ok());
    }

    #[test]
    fn test_resolve_all_omitted_create() {
        // All positions omitted on create - should assign 0, 1, 2 in input order
        let mut inputs = vec![
            TestInput::new(None),
            TestInput::new(None),
            TestInput::new(None),
        ];
        let existing: Vec<TestEntity> = vec![];

        assert!(resolve_and_validate_input_positions(&mut inputs, &existing, "test").is_ok());

        assert_eq!(inputs[0].position, Some(0));
        assert_eq!(inputs[1].position, Some(1));
        assert_eq!(inputs[2].position, Some(2));
    }

    #[test]
    fn test_resolve_all_omitted_update_existing_only() {
        // Update with all positions omitted - existing items keep their positions
        let existing = vec![TestEntity::new(0), TestEntity::new(1), TestEntity::new(2)];
        let mut inputs = vec![
            TestInput::with_id(existing[0].id, None),
            TestInput::with_id(existing[1].id, None),
            TestInput::with_id(existing[2].id, None),
        ];

        assert!(resolve_and_validate_input_positions(&mut inputs, &existing, "test").is_ok());

        // Should preserve existing positions
        assert_eq!(inputs[0].position, Some(0));
        assert_eq!(inputs[1].position, Some(1));
        assert_eq!(inputs[2].position, Some(2));
    }

    #[test]
    fn test_resolve_all_omitted_update_with_new_items() {
        // Update with existing items plus new items - new items append to end
        let existing = vec![TestEntity::new(0), TestEntity::new(1)];
        let mut inputs = vec![
            TestInput::with_id(existing[0].id, None), // existing
            TestInput::with_id(existing[1].id, None), // existing
            TestInput::new(None),                     // new
            TestInput::new(None),                     // new
        ];

        assert!(resolve_and_validate_input_positions(&mut inputs, &existing, "test").is_ok());

        // Existing keep their positions
        assert_eq!(inputs[0].position, Some(0));
        assert_eq!(inputs[1].position, Some(1));
        // New items appended
        assert_eq!(inputs[2].position, Some(2));
        assert_eq!(inputs[3].position, Some(3));
    }

    #[test]
    fn test_resolve_all_specified_valid() {
        // All positions specified and valid
        let mut inputs = vec![
            TestInput::new(Some(0)),
            TestInput::new(Some(1)),
            TestInput::new(Some(2)),
        ];
        let existing: Vec<TestEntity> = vec![];

        assert!(resolve_and_validate_input_positions(&mut inputs, &existing, "test").is_ok());
    }

    #[test]
    fn test_resolve_all_specified_invalid() {
        // All positions specified but not sequential
        let mut inputs = vec![
            TestInput::new(Some(0)),
            TestInput::new(Some(2)), // gap
            TestInput::new(Some(3)),
        ];
        let existing: Vec<TestEntity> = vec![];

        let result = resolve_and_validate_input_positions(&mut inputs, &existing, "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("sequential"));
    }

    #[test]
    fn test_resolve_mixed_error() {
        // Mixed: some specified, some not - should error
        let mut inputs = vec![
            TestInput::new(Some(0)),
            TestInput::new(None), // mixed!
            TestInput::new(Some(2)),
        ];
        let existing: Vec<TestEntity> = vec![];

        let result = resolve_and_validate_input_positions(&mut inputs, &existing, "test");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("all specified or all omitted")
        );
    }

    // =========================================================================
    // validate_position_range tests
    // =========================================================================

    #[test]
    fn test_validate_position_range_valid() {
        assert!(validate_position_range(0, 3, "test").is_ok());
        assert!(validate_position_range(1, 3, "test").is_ok());
        assert!(validate_position_range(2, 3, "test").is_ok());
    }

    #[test]
    fn test_validate_position_range_negative() {
        assert!(validate_position_range(-1, 3, "test").is_err());
    }

    #[test]
    fn test_validate_position_range_too_high() {
        assert!(validate_position_range(3, 3, "test").is_err());
        assert!(validate_position_range(10, 3, "test").is_err());
    }

    #[test]
    fn test_validate_position_range_empty_collection() {
        assert!(validate_position_range(0, 0, "test").is_err());
    }

    // =========================================================================
    // validate_no_position_conflict tests
    // =========================================================================

    #[test]
    fn test_validate_no_conflict_empty() {
        let entities: Vec<TestEntity> = vec![];
        assert!(validate_no_position_conflict(0, None, &entities).is_ok());
    }

    #[test]
    fn test_validate_no_conflict_no_conflict() {
        let entities = vec![TestEntity::new(0), TestEntity::new(1)];
        assert!(validate_no_position_conflict(2, None, &entities).is_ok());
    }

    #[test]
    fn test_validate_no_conflict_has_conflict() {
        let entities = vec![TestEntity::new(0), TestEntity::new(1)];
        assert!(validate_no_position_conflict(1, None, &entities).is_err());
    }

    #[test]
    fn test_validate_no_conflict_exclude_self() {
        let entities = vec![TestEntity::new(0), TestEntity::new(1)];
        let self_id = entities[1].id;
        // Position 1 is occupied by self, so no conflict when excluding self
        assert!(validate_no_position_conflict(1, Some(self_id), &entities).is_ok());
    }

    // =========================================================================
    // renumber_positions tests
    // =========================================================================

    #[test]
    fn test_renumber_empty() {
        let mut entities: Vec<TestEntity> = vec![];
        assert!(!renumber_positions(&mut entities));
    }

    #[test]
    fn test_renumber_already_sequential() {
        let mut entities = vec![TestEntity::new(0), TestEntity::new(1), TestEntity::new(2)];
        assert!(!renumber_positions(&mut entities));
    }

    #[test]
    fn test_renumber_closes_gaps() {
        let mut entities = vec![TestEntity::new(0), TestEntity::new(2), TestEntity::new(5)];

        assert!(renumber_positions(&mut entities));

        // Should be renumbered to 0, 1, 2
        entities.sort_by_key(|e| e.position());
        assert_eq!(entities[0].position(), 0);
        assert_eq!(entities[1].position(), 1);
        assert_eq!(entities[2].position(), 2);
    }

    #[test]
    fn test_renumber_handles_unordered() {
        let mut entities = vec![TestEntity::new(5), TestEntity::new(0), TestEntity::new(2)];

        assert!(renumber_positions(&mut entities));

        entities.sort_by_key(|e| e.position());
        assert_eq!(entities[0].position(), 0);
        assert_eq!(entities[1].position(), 1);
        assert_eq!(entities[2].position(), 2);
    }

    // =========================================================================
    // reorder_positions tests
    // =========================================================================

    #[test]
    fn test_reorder_same_position() {
        let mut entities = vec![TestEntity::new(0), TestEntity::new(1), TestEntity::new(2)];
        assert!(!reorder_positions(&mut entities, 1, 1));
    }

    #[test]
    fn test_reorder_out_of_bounds() {
        let mut entities = vec![TestEntity::new(0), TestEntity::new(1)];
        assert!(!reorder_positions(&mut entities, 0, 5));
        assert!(!reorder_positions(&mut entities, -1, 0));
    }

    #[test]
    fn test_reorder_move_down() {
        // [A:0, B:1, C:2] -> move A from 0 to 2 -> [B:0, C:1, A:2]
        let mut entities = vec![TestEntity::new(0), TestEntity::new(1), TestEntity::new(2)];
        let a_id = entities[0].id;

        assert!(reorder_positions(&mut entities, 0, 2));

        // Find entity A and verify it's at position 2
        let a = entities.iter().find(|e| e.id == a_id).unwrap();
        assert_eq!(a.position(), 2);

        // Verify all positions are still sequential
        let mut positions: Vec<i32> = entities.iter().map(|e| e.position()).collect();
        positions.sort();
        assert_eq!(positions, vec![0, 1, 2]);
    }

    #[test]
    fn test_reorder_move_up() {
        // [A:0, B:1, C:2] -> move C from 2 to 0 -> [C:0, A:1, B:2]
        let mut entities = vec![TestEntity::new(0), TestEntity::new(1), TestEntity::new(2)];
        let c_id = entities[2].id;

        assert!(reorder_positions(&mut entities, 2, 0));

        // Find entity C and verify it's at position 0
        let c = entities.iter().find(|e| e.id == c_id).unwrap();
        assert_eq!(c.position(), 0);

        // Verify all positions are still sequential
        let mut positions: Vec<i32> = entities.iter().map(|e| e.position()).collect();
        positions.sort();
        assert_eq!(positions, vec![0, 1, 2]);
    }

    // =========================================================================
    // next_position tests
    // =========================================================================

    #[test]
    fn test_next_position_empty() {
        let entities: Vec<TestEntity> = vec![];
        assert_eq!(next_position(&entities), 0);
    }

    #[test]
    fn test_next_position_with_entities() {
        let entities = vec![TestEntity::new(0), TestEntity::new(1), TestEntity::new(2)];
        assert_eq!(next_position(&entities), 3);
    }

    // =========================================================================
    // capitalize tests
    // =========================================================================

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("interface"), "Interface");
        assert_eq!(capitalize("service"), "Service");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("a"), "A");
    }
}
