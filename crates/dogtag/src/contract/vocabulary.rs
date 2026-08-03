//! The tag vocabulary a corpus declares: `[tags]` and its namespaces.
//!
//! **Tags are content.** What this module models is the part of a corpus's
//! tagging the corpus chose to *schematize* — never a license to enumerate all
//! of it — and it does so on the lifecycle seam: `[tags]` **names a declared
//! property** rather than reserving a frontmatter word, so the kernel never
//! learns any corpus's word for tags and behavior binds to a declaration rather
//! than to a name.
//!
//! Only contract version 2 defines these constructs, so a version-1 model
//! carries no tag vocabulary at all — absent, rather than present and empty,
//! because a version that does not define a construct is not a version that
//! declares none of it.

/// The property a corpus carries its tags on.
///
/// Optional at the one version that defines it: a corpus with no tag vocabulary
/// declares nothing and loses nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagsDecl {
    pub(crate) property: String,
}

impl TagsDecl {
    /// The property whose values are the corpus's tags.
    pub fn property(&self) -> &str {
        &self.property
    }
}

/// One tag namespace declared on one type.
///
/// A namespace is a **prefix test and nothing more**. The kernel owns no
/// separator convention and never splits a tag, so the prefix carries whatever
/// separator the corpus writes and a member names the remainder after it. This
/// is deliberately not a pattern facility: two fixed behaviors, on the tag
/// plane, where a corpus's own model already put prefixes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagNamespaceDecl {
    pub(crate) prefix: String,
    pub(crate) required: bool,
    pub(crate) membership: NamespaceMembership,
}

impl TagNamespaceDecl {
    /// The literal string a tag starts with to be in this namespace, separator
    /// included. Unique within its type.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Whether every note of the declaring type carries at least one tag in
    /// this namespace.
    pub fn required(&self) -> bool {
        self.required
    }

    /// Whether the namespace bounds its membership, and to what.
    pub fn membership(&self) -> &NamespaceMembership {
        &self.membership
    }

    /// The closed vocabulary's members, when the namespace declares one.
    pub fn values(&self) -> Option<&[String]> {
        self.membership.values()
    }
}

/// Whether a tag namespace bounds its membership.
///
/// Exactly one of the two is declared, mirroring the lifecycle table's own
/// exclusive discipline rather than making omission a decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NamespaceMembership {
    /// A closed vocabulary: each member names the remainder after the prefix.
    Closed {
        /// The members, in declaration order. Non-empty and free of repeats.
        values: Vec<String>,
    },
    /// The namespace is declared without bounding its membership.
    Open,
}

impl NamespaceMembership {
    /// The members of a closed vocabulary.
    pub fn values(&self) -> Option<&[String]> {
        match self {
            Self::Closed { values } => Some(values),
            Self::Open => None,
        }
    }

    /// Whether the namespace leaves its membership unbounded.
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two memberships a namespace can declare, on one type.
    fn namespaces() -> Vec<TagNamespaceDecl> {
        vec![
            TagNamespaceDecl {
                prefix: "role/".to_owned(),
                required: true,
                membership: NamespaceMembership::Closed {
                    values: vec!["founder".to_owned(), "advisor".to_owned()],
                },
            },
            TagNamespaceDecl {
                prefix: "topic/".to_owned(),
                required: false,
                membership: NamespaceMembership::Open,
            },
        ]
    }

    #[test]
    fn a_tags_table_names_the_property_that_carries_the_corpuss_tags() {
        let tags = TagsDecl {
            property: "labels".to_owned(),
        };
        assert_eq!(tags.property(), "labels");
    }

    #[test]
    fn a_closed_namespace_bounds_its_membership_to_what_it_declares() {
        let closed = &namespaces()[0];
        assert_eq!(
            (closed.prefix(), closed.required(), closed.values()),
            (
                "role/",
                true,
                Some(&["founder".to_owned(), "advisor".to_owned()][..])
            )
        );
        assert!(!closed.membership().is_open());
    }

    #[test]
    fn an_open_namespace_declares_that_it_bounds_nothing() {
        let open = &namespaces()[1];
        assert_eq!(
            (open.prefix(), open.required(), open.values()),
            ("topic/", false, None)
        );
        assert!(open.membership().is_open());
        assert_eq!(open.membership(), &NamespaceMembership::Open);
    }

    #[test]
    fn the_tag_declarations_clone_compare_and_format() {
        let tags = TagsDecl {
            property: "labels".to_owned(),
        };
        assert_eq!(tags.clone(), tags);
        assert!(format!("{tags:?}").contains("labels"));
        let declared = namespaces();
        assert_eq!(declared[0].clone(), declared[0]);
        assert_ne!(declared[0], declared[1]);
        assert!(format!("{:?}", declared[0]).contains("founder"));
        assert!(format!("{:?}", NamespaceMembership::Open).contains("Open"));
    }
}
