//! Who is acting, in what capacity, and when.
//!
//! All three are **inputs**, never things this module reads for itself. The
//! actor resolves from the installation record and is overridable per
//! invocation, and the contract never carries actor identity at all; the clock
//! is an ambient fact of the machine. Resolving each is the consumer's job on
//! its side of the boundary, which is what keeps every entry point here a pure
//! function of its arguments.
//!
//! Nothing here is [`crate::provenance`]. That module answers *which file a
//! resolved configuration value came from* and its `Source` is closed around
//! the contract, the record, and a format default. This one answers *who
//! performed an act, and in what capacity*. The two words are the same and the
//! questions are not.

use std::time::{SystemTime, UNIX_EPOCH};

/// In what capacity an act was performed.
///
/// A closed set, so that a corpus's history can be read by capacity without
/// anyone agreeing on a vocabulary first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProvenanceKind {
    /// A person, acting directly.
    Human,
    /// An agent acting on a person's behalf.
    Agent,
    /// A scheduled or triggered process, acting on nobody's immediate behalf.
    Automation,
}

impl ProvenanceKind {
    /// Every kind the format defines, in the order this enum declares them.
    pub const ALL: &'static [ProvenanceKind] = &[Self::Human, Self::Agent, Self::Automation];

    /// The spelling every structured format and every commit trailer writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Agent => "agent",
            Self::Automation => "automation",
        }
    }

    /// The kind `name` spells, if the format defines one.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.as_str() == name)
    }
}

/// Who performed an act.
///
/// The name is optional and its absence is a state rather than a fault: an
/// installation that has not been configured has no actor, and **a missing
/// actor does not gate a write**. The act lands, provenance records as
/// unattributed, and the result says so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    name: Option<String>,
    kind: ProvenanceKind,
}

impl Actor {
    /// An actor of `kind`, named or not.
    pub fn new(name: Option<String>, kind: ProvenanceKind) -> Self {
        Self { name, kind }
    }

    /// The name, when the installation names one.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The capacity the act was performed in, which is always known.
    pub fn kind(&self) -> ProvenanceKind {
        self.kind
    }

    /// Whether this act can be attributed to anyone.
    pub fn is_attributed(&self) -> bool {
        self.name.is_some()
    }
}

/// The instant an act's identity derives from, as whole seconds since the Unix
/// epoch.
///
/// A value rather than a clock read on the caller's behalf: identical input
/// must produce an identical plan, and a function that read the clock could
/// never be asked twice about the same act. It is also what lets a conformance
/// run pin the one part of a capture that is not a function of its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapturedAt(u64);

impl CapturedAt {
    /// The instant `seconds` after the epoch.
    pub fn from_unix_seconds(seconds: u64) -> Self {
        Self(seconds)
    }

    /// The instant `time` names.
    ///
    /// A clock set before the epoch saturates to it rather than refusing. A
    /// misconfigured machine clock is not a reason to stand between a thought
    /// and its capture, and the note it produces is findable by every other
    /// means a note is findable by.
    pub fn at(time: SystemTime) -> Self {
        Self(
            time.duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_secs()),
        )
    }

    /// Whole seconds since the epoch.
    pub fn unix_seconds(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    #[test]
    fn every_provenance_kind_spells_itself_and_answers_to_that_spelling() {
        for kind in ProvenanceKind::ALL {
            assert_eq!(ProvenanceKind::named(kind.as_str()), Some(*kind));
        }
        let spellings: Vec<&str> = ProvenanceKind::ALL
            .iter()
            .map(|kind| kind.as_str())
            .collect();
        assert_eq!(spellings, ["human", "agent", "automation"]);
        assert_eq!(ProvenanceKind::named("robot"), None);
    }

    #[test]
    fn provenance_kinds_copy_compare_and_format() {
        let kind = ProvenanceKind::Agent;
        assert_eq!(kind, ProvenanceKind::Agent);
        assert_ne!(kind, ProvenanceKind::Human);
        assert!(format!("{kind:?}").contains("Agent"));
    }

    #[test]
    fn an_actor_answers_for_its_name_and_its_capacity() {
        let named = Actor::new(Some("A Maintainer".to_owned()), ProvenanceKind::Human);
        assert_eq!(named.name(), Some("A Maintainer"));
        assert_eq!(named.kind(), ProvenanceKind::Human);
        assert!(named.is_attributed());
        assert_eq!(named.clone(), named);
        assert!(format!("{named:?}").contains("A Maintainer"));
    }

    /// An unconfigured installation has no actor, and that is a state: the
    /// capacity is still known, because the invocation always knows it.
    #[test]
    fn an_unattributed_actor_still_carries_a_capacity() {
        let anonymous = Actor::new(None, ProvenanceKind::Agent);
        assert_eq!(anonymous.name(), None);
        assert_eq!(anonymous.kind(), ProvenanceKind::Agent);
        assert!(!anonymous.is_attributed());
        assert_ne!(anonymous, Actor::new(None, ProvenanceKind::Human));
    }

    #[test]
    fn an_instant_answers_with_the_seconds_it_was_built_from() {
        let at = CapturedAt::from_unix_seconds(1_786_000_000);
        assert_eq!(at.unix_seconds(), 1_786_000_000);
        assert_eq!(at, CapturedAt::from_unix_seconds(1_786_000_000));
        assert!(at > CapturedAt::from_unix_seconds(0));
        assert!(format!("{at:?}").contains("1786000000"));
    }

    #[test]
    fn a_system_time_reduces_to_whole_seconds_since_the_epoch() {
        let time = UNIX_EPOCH + Duration::from_millis(1_786_000_000_500);
        assert_eq!(CapturedAt::at(time).unix_seconds(), 1_786_000_000);
        assert_eq!(CapturedAt::at(UNIX_EPOCH).unix_seconds(), 0);
    }

    /// A clock set before the epoch saturates rather than refusing: never-lossy
    /// outranks a machine's misconfiguration.
    #[test]
    fn a_clock_set_before_the_epoch_saturates_to_it() {
        let before = UNIX_EPOCH - Duration::from_secs(1);
        assert_eq!(CapturedAt::at(before).unix_seconds(), 0);
    }
}
