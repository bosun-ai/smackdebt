//! Source-unit comparison values and their participation in a debt verdict.

use crate::health::{Rating, is_rated};
use crate::measurements::Measurements;
use crate::report::{ComparisonId, FileId};
use crate::source::{SourceRole, SourceSpan, UnitIdentity};

/// Whether a diff unit was added, removed, or changed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComparisonKind {
    Added,
    Removed,
    Improved,
    Regressed,
    MetricChanged,
    Ambiguous,
    Unchanged,
}

/// The user-facing direction of a retained diff comparison.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComparisonDirection {
    Worse,
    Better,
    Changed,
}

/// Whether a retained source comparison may move the diff verdict.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComparisonParticipation {
    Verdict,
    Context,
}

/// A named unit comparison between two source versions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comparison {
    id: ComparisonId,
    identity: UnitIdentity,
    kind: ComparisonKind,
    before: Option<Measurements>,
    after: Option<Measurements>,
    before_rating: Option<Rating>,
    after_rating: Option<Rating>,
    file: Option<FileId>,
    span: Option<SourceSpan>,
    origin: Option<(FileId, SourceSpan)>,
    anonymous_ambiguity: bool,
    unpaired_anonymous: bool,
    role_participation: ComparisonParticipation,
}

impl Comparison {
    pub fn new(
        id: ComparisonId,
        identity: UnitIdentity,
        kind: ComparisonKind,
        before: Option<Measurements>,
        after: Option<Measurements>,
        before_rating: Option<Rating>,
        after_rating: Option<Rating>,
    ) -> Self {
        Self {
            id,
            identity,
            kind,
            before,
            after,
            before_rating,
            after_rating,
            file: None,
            span: None,
            origin: None,
            anonymous_ambiguity: false,
            unpaired_anonymous: false,
            role_participation: ComparisonParticipation::Verdict,
        }
    }

    pub const fn id(&self) -> ComparisonId {
        self.id
    }
    pub fn identity(&self) -> &UnitIdentity {
        &self.identity
    }
    pub const fn kind(&self) -> ComparisonKind {
        self.kind
    }
    pub const fn before(&self) -> Option<Measurements> {
        self.before
    }
    pub const fn after(&self) -> Option<Measurements> {
        self.after
    }
    pub const fn before_rating(&self) -> Option<Rating> {
        self.before_rating
    }
    pub const fn after_rating(&self) -> Option<Rating> {
        self.after_rating
    }
    pub const fn file(&self) -> Option<FileId> {
        self.file
    }
    pub fn with_file(mut self, file: FileId) -> Self {
        self.file = Some(file);
        self
    }
    /// The unit's location on the side that still has one.
    ///
    /// The span locates a comparison for a reader; it is presentation context
    /// and stays out of the machine report, which identifies a unit by name.
    pub const fn span(&self) -> Option<SourceSpan> {
        self.span
    }
    /// Where the unit answered from before the change followed it here.
    ///
    /// A unit the change moved between files is one comparison, located on
    /// the side it landed. The origin states the side it left, so a reader
    /// who knew it by its old home can still find it.
    pub const fn origin(&self) -> Option<(FileId, SourceSpan)> {
        self.origin
    }
    #[must_use]
    pub const fn with_origin(mut self, file: FileId, span: SourceSpan) -> Self {
        self.origin = Some((file, span));
        self
    }
    pub const fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }
    pub const fn is_anonymous_ambiguity(&self) -> bool {
        self.anonymous_ambiguity
    }
    pub const fn with_anonymous_ambiguity(mut self) -> Self {
        self.anonymous_ambiguity = true;
        self
    }
    /// Whether this is an anonymous unit the matcher gave up on pairing.
    pub const fn is_unpaired_anonymous(&self) -> bool {
        self.unpaired_anonymous
    }
    /// Records that no evidence paired this anonymous unit with the other side.
    ///
    /// The comparison stays in the machine report and reaches a reader under
    /// `--all`, but it never moves debt: one edit the matcher could not follow
    /// must not read as debt added over there and debt removed over here.
    pub const fn with_unpaired_anonymous(mut self) -> Self {
        self.unpaired_anonymous = true;
        self
    }
    /// Records whether every source side present may move diff debt.
    ///
    /// A fixture or generated side makes the whole comparison context, even
    /// when the file's role changes on the other side of the diff.
    /// Records a participation another pass settled, when the roles it read
    /// are not the ones this comparison's file carries.
    #[must_use]
    pub const fn with_participation(mut self, participation: ComparisonParticipation) -> Self {
        self.role_participation = participation;
        self
    }
    pub const fn with_source_roles(
        mut self,
        before: Option<SourceRole>,
        after: Option<SourceRole>,
    ) -> Self {
        self.role_participation =
            if role_is_verdict_eligible(before) && role_is_verdict_eligible(after) {
                ComparisonParticipation::Verdict
            } else {
                ComparisonParticipation::Context
            };
        self
    }
    pub const fn participation(&self) -> ComparisonParticipation {
        if self.unpaired_anonymous {
            return ComparisonParticipation::Context;
        }
        self.role_participation
    }
    pub const fn affects_verdict(&self) -> bool {
        matches!(self.participation(), ComparisonParticipation::Verdict)
    }
    /// Whether the source this comparison names may move debt at all.
    ///
    /// An unpaired anonymous comparison is withheld from the verdict while
    /// still naming verdict-eligible source. The file's diagnostic reads this
    /// rather than participation, so a warning about the verdict is not filed
    /// as context beside it.
    pub const fn source_moves_debt(&self) -> bool {
        matches!(self.role_participation, ComparisonParticipation::Verdict)
    }
    /// The direction a reader may read off this comparison.
    ///
    /// An unpaired anonymous unit has no direction to state: the matcher can
    /// see the file changed here and cannot see which way, so calling the
    /// leftover side added or removed debt would be a claim it has not earned.
    pub const fn direction(&self) -> ComparisonDirection {
        if self.unpaired_anonymous {
            return ComparisonDirection::Changed;
        }
        rated_direction(self.kind, self.before_rating, self.after_rating)
    }
}

/// Which way a paired or one-sided comparison moved debt.
///
/// A unit that was added or removed only moves debt while it was rated: the
/// healthy units a refactor shuffles are changes, not debt.
const fn rated_direction(
    kind: ComparisonKind,
    before_rating: Option<Rating>,
    after_rating: Option<Rating>,
) -> ComparisonDirection {
    match kind {
        ComparisonKind::Regressed => ComparisonDirection::Worse,
        ComparisonKind::Improved => ComparisonDirection::Better,
        ComparisonKind::Added if is_rated(after_rating) => ComparisonDirection::Worse,
        ComparisonKind::Removed if is_rated(before_rating) => ComparisonDirection::Better,
        _ => ComparisonDirection::Changed,
    }
}

const fn role_is_verdict_eligible(role: Option<SourceRole>) -> bool {
    match role {
        Some(role) => role.affects_verdict(),
        None => true,
    }
}
