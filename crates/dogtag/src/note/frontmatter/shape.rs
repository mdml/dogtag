//! The four shapes a frontmatter value may take.
//!
//! The subset admits values "nested at most one level below the top", and that
//! is a **shape** rule rather than a depth count. Written as a counter it would
//! refuse a list of records — whose mappings sit two levels below the top — and
//! that is the one shape the `record` kind was added in order to write. Written
//! as a shape it says what it means:
//!
//! - a scalar;
//! - a sequence of scalars;
//! - a mapping of scalars, which is a record value;
//! - a sequence of mappings of scalars, which is a list of records.
//!
//! Everything else — a mapping inside a mapping, a sequence inside a sequence,
//! a sequence inside a record's field — is refused. A reader arriving at this
//! module from the record's sentence should read this paragraph first: the code
//! is not wrong, the sentence is shorter than the rule.

use super::{Entry, Fault, FaultKind, Shape, Value};

/// Refuses every value outside the four shapes.
pub(super) fn check(entries: &[Entry], faults: &mut Vec<Fault>) {
    for entry in entries {
        match &entry.value.shape {
            Shape::Scalar(_) => {}
            Shape::Sequence(items) => items
                .iter()
                .for_each(|item| element(item, &entry.key, faults)),
            Shape::Mapping(fields) => scalars(fields, faults),
        }
    }
}

/// One element of a sequence: a scalar, or a mapping of scalars.
fn element(item: &Value, key: &str, faults: &mut Vec<Fault>) {
    match &item.shape {
        Shape::Scalar(_) => {}
        Shape::Mapping(fields) => scalars(fields, faults),
        Shape::Sequence(_) => faults.push(refusal(
            item,
            format!("`{key}` holds a sequence inside a sequence"),
        )),
    }
}

/// Every entry of a mapping carries a scalar, and nothing deeper.
fn scalars(entries: &[Entry], faults: &mut Vec<Fault>) {
    for entry in entries {
        if !matches!(entry.value.shape, Shape::Scalar(_)) {
            faults.push(refusal(
                &entry.value,
                format!("the field `{}` holds {}", entry.key, entry.value.describe()),
            ));
        }
    }
}

fn refusal(value: &Value, what: String) -> Fault {
    Fault::new(
        FaultKind::Unsupported,
        format!(
            "{what}: frontmatter admits a scalar, a sequence of scalars, a mapping of scalars, \
             or a sequence of mappings of scalars"
        ),
        value.span.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::super::{Front, read};

    fn refused(block: &str) -> Vec<String> {
        let parsed = read(&format!("---\n{block}---\n"));
        assert_eq!(parsed.front, Front::Refused, "the block must be refused");
        parsed
            .faults
            .iter()
            .map(|fault| fault.message.clone())
            .collect()
    }

    fn accepted(block: &str) {
        let parsed = read(&format!("---\n{block}---\n"));
        assert!(
            matches!(parsed.front, Front::Read(_)),
            "the block must load: {:?}",
            parsed.faults
        );
    }

    #[test]
    fn every_shape_the_subset_admits_loads() {
        accepted("a: one\n");
        accepted("a: [one, two]\n");
        accepted("a:\n  given: one\n");
        accepted("a:\n  - given: one\n  - given: two\n");
        accepted("a: [{given: one}]\n");
    }

    #[test]
    fn a_mapping_inside_a_mapping_is_refused_by_the_shape_rule() {
        let messages = refused("a:\n  b:\n    c: one\n");
        assert_eq!(messages.len(), 1);
        assert!(
            messages[0].contains("the field `b` holds a mapping"),
            "{messages:?}"
        );
        assert!(messages[0].contains("a mapping of scalars"));
    }

    #[test]
    fn a_sequence_inside_a_mapping_is_refused_by_the_shape_rule() {
        let messages = refused("a:\n  b: [one]\n");
        assert!(
            messages[0].contains("the field `b` holds a sequence"),
            "{messages:?}"
        );
    }

    #[test]
    fn a_sequence_inside_a_sequence_is_refused_by_the_shape_rule() {
        let messages = refused("a: [[one]]\n");
        assert!(
            messages[0].contains("`a` holds a sequence inside a sequence"),
            "{messages:?}"
        );
    }

    #[test]
    fn a_sequence_inside_a_records_field_is_refused_wherever_the_record_sits() {
        let messages = refused("a:\n  - b: [one]\n");
        assert!(
            messages[0].contains("the field `b` holds a sequence"),
            "{messages:?}"
        );
    }
}
