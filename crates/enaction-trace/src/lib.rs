// SPDX-License-Identifier: AGPL-3.0-or-later
//! A deliberately small, game-neutral event seam.
//!
//! The crate makes two structural commitments and is otherwise nothing but
//! their consequences.
//!
//! # Typed by domain
//!
//! Domains are separate types in the trace: affect never silently becomes
//! knowledge, a belief never silently becomes a goal, and none of them chooses
//! behaviour without an explicit selection event.
//!
//! # Particularised by trope
//!
//! A [`Trope`] is an *abstract particular* — **this** bearer's **this**
//! property-instance, with its own identity. Trope theory's claim (Williams,
//! Campbell) is that properties are particulars rather than universals: there
//! is no single shared "suspicion" that several guards each instantiate. There
//! are several suspicions, which *resemble* one another.
//!
//! That distinction is not decoration here — it changes what the engine can
//! say:
//!
//! * **Separate causal histories.** Two guards suspecting the same object are
//!   two particulars with independent pasts. Keyed as `(bearer, state)` pairs
//!   into a shared universal, that independence is representable only by
//!   convention; as tropes it is structural.
//! * **A belief can be referred to.** A trope has an id, so an event can point
//!   at *the belief that caused this*, not merely at an earlier event.
//! * **Resemblance replaces identity.** [`Trope::resembles`] asks whether two
//!   particulars are of a kind, rather than testing equality — which is what
//!   [`Domain::SocialRelational`] needs in order to have anything to reason
//!   over.
//! * **Domain becomes structural.** `domain` lives on the trope, not on the
//!   event, so one property-instance cannot have one event call it affective
//!   and the next call it epistemic. Domain consistency stops being something
//!   a validator has to hope for.
//!
//! # What the engine does *not* supply
//!
//! A game supplies the vocabulary in [`Trope::kind`] and [`Trope::about`]; the
//! engine supplies identity, ordering, causality and domain separation. The
//! engine never interprets a game word.
//!
//! Magnitudes are integer milliunits ([`TraceEvent::value_milli`]), never
//! floats: a trace must compare bit-for-bit across machines and replays, and
//! float arithmetic near identity does not (see `enaction-time`'s `lerp`, which
//! meets the same constraint the same way).

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Which kind of state a trope carries, and therefore which stage of the
/// cognitive chain any event about it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Mechanical,
    Epistemic,
    Affective,
    Conative,
    BehaviouralSelection,
    SocialRelational,
}

/// Identity of an abstract particular.
///
/// Distinct from a [`TraceEvent`] id: the trope is what persists, the events
/// are what happened to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TropeId(pub u64);

/// An abstract particular: *this* bearer's *this* property-instance.
///
/// Abstract because it is a property rather than a whole thing; particular
/// because it belongs to one bearer and is not shared. Billy's
/// interest-in-the-note is one trope; another guard's interest in the same note
/// is a different one that merely resembles it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trope {
    pub id: TropeId,
    /// Whose property-instance this is.
    pub bearer: String,
    /// Its intentional object, where it has one — what the state is *about*.
    /// `None` for non-relational states such as bare arousal.
    pub about: Option<String>,
    /// The game's word for the resemblance class. The engine never reads it.
    pub kind: String,
    /// Fixed for the life of the trope; every event about it inherits this.
    pub domain: Domain,
}

impl Trope {
    /// Whether two tropes are of a kind — same domain, same game word, and the
    /// same intentional object.
    ///
    /// **Resemblance is not identity.** Two bearers' suspicions of the same
    /// object resemble each other and remain two separate particulars with
    /// separate histories. This is the relation [`Domain::SocialRelational`]
    /// reasoning is meant to be built over: *"does what he feels resemble what
    /// I feel?"* is answerable; *"is it the same feeling?"* is not a question
    /// tropes permit.
    #[must_use]
    pub fn resembles(&self, other: &Self) -> bool {
        self.domain == other.domain && self.kind == other.kind && self.about == other.about
    }

    /// Whether this trope resembles `other` but is borne by someone else.
    ///
    /// The companion to [`resembles`](Self::resembles), which a trope trivially
    /// satisfies against itself.
    #[must_use]
    pub fn is_peer_of(&self, other: &Self) -> bool {
        self.bearer != other.bearer && self.resembles(other)
    }
}

/// One change to, or observation of, a trope.
///
/// Carries no domain of its own: the domain is the trope's, so an event cannot
/// reclassify the particular it is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub id: u64,
    pub tick: u64,
    /// The particular this event is about.
    pub trope: TropeId,
    /// Magnitude in integer milliunits. Never a float — see the crate docs.
    pub value_milli: i32,
    /// Ids of the events that brought this one about.
    pub caused_by: Vec<u64>,
}

impl TraceEvent {
    /// The engine's total order over events: by tick, then by id.
    ///
    /// Deliberately independent of position in any slice, so causality can be
    /// judged without first requiring the caller to sort.
    fn stamp(&self) -> (u64, u64) {
        (self.tick, self.id)
    }
}

/// A trace: the particulars, and what happened to them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trace {
    pub tropes: Vec<Trope>,
    pub events: Vec<TraceEvent>,
}

impl Trace {
    /// Validate the structural contract without interpreting any game word.
    ///
    /// # Errors
    ///
    /// Returns every violation found, not only the first.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Pass 1 — index the particulars.
        let mut tropes = BTreeMap::new();
        for trope in &self.tropes {
            if tropes.insert(trope.id, trope).is_some() {
                errors.push(format!("duplicate trope id {}", trope.id.0));
            }
            if trope.bearer.is_empty() || trope.kind.is_empty() {
                errors.push(format!("trope {} has an empty bearer or kind", trope.id.0));
            }
        }

        // Pass 2 — index the events, so causality can be judged without
        // depending on whatever order the caller happened to supply.
        let mut stamps = BTreeMap::new();
        for event in &self.events {
            if stamps.insert(event.id, event.stamp()).is_some() {
                errors.push(format!("duplicate trace event id {}", event.id));
            }
        }

        // Pass 3 — the real checks.
        for event in &self.events {
            if !tropes.contains_key(&event.trope) {
                errors.push(format!(
                    "event {} is about trope {}, which is not declared",
                    event.id, event.trope.0
                ));
            }
            for cause in &event.caused_by {
                match stamps.get(cause) {
                    // Absent and out-of-order are different faults and are now
                    // reported as such. The previous single-pass check could
                    // only say "absent or not earlier", because it judged
                    // causality by position in the slice.
                    None => errors.push(format!(
                        "event {} names cause {}, which is not in the trace",
                        event.id, cause
                    )),
                    Some(&cause_stamp) if cause_stamp >= event.stamp() => {
                        errors.push(format!(
                            "event {} names cause {}, which is not strictly earlier \
                             (cause tick {} id {}, effect tick {} id {})",
                            event.id, cause, cause_stamp.0, cause_stamp.1, event.tick, event.id
                        ));
                    }
                    Some(_) => {}
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Whether the events are already in the engine's canonical order.
    ///
    /// Presentation only. An unordered trace is **not** invalid — causality is
    /// judged by `(tick, id)` rather than by position — so this is reported
    /// separately from [`validate`](Self::validate) rather than folded into it.
    #[must_use]
    pub fn is_canonically_ordered(&self) -> bool {
        self.events.windows(2).all(|w| w[0].stamp() < w[1].stamp())
    }

    /// Sort the events into the engine's canonical order, in place.
    pub fn canonicalise(&mut self) {
        self.events.sort_by_key(TraceEvent::stamp);
    }

    /// Every trope that resembles `subject` but is borne by someone else.
    ///
    /// The primitive that [`Domain::SocialRelational`] reasoning is built from.
    pub fn peers_of<'a>(&'a self, subject: &'a Trope) -> impl Iterator<Item = &'a Trope> {
        self.tropes.iter().filter(move |t| subject.is_peer_of(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trope(id: u64, bearer: &str, kind: &str, domain: Domain) -> Trope {
        Trope {
            id: TropeId(id),
            bearer: bearer.into(),
            about: None,
            kind: kind.into(),
            domain,
        }
    }

    fn event(id: u64, tick: u64, trope: u64, caused_by: Vec<u64>) -> TraceEvent {
        TraceEvent {
            id,
            tick,
            trope: TropeId(trope),
            value_milli: 500,
            caused_by,
        }
    }

    /// The chain every domain must pass through, kept explicit end to end.
    fn appraisal_chain() -> Trace {
        let domains = [
            Domain::Mechanical,
            Domain::Epistemic,
            Domain::Affective,
            Domain::Conative,
            Domain::BehaviouralSelection,
            Domain::SocialRelational,
        ];
        let kinds = [
            "signal_changed",
            "possible_cause",
            "arousal_changed",
            "goal_priority_changed",
            "verification_selected",
            "protect_relation",
        ];
        Trace {
            tropes: domains
                .into_iter()
                .zip(kinds)
                .enumerate()
                .map(|(i, (domain, kind))| trope(i as u64 + 1, "agent", kind, domain))
                .collect(),
            events: (0..6)
                .map(|i| event(i + 1, 12, i + 1, if i == 0 { vec![] } else { vec![i] }))
                .collect(),
        }
    }

    #[test]
    fn deterministic_appraisal_chain_keeps_every_domain_explicit() {
        let trace = appraisal_chain();
        assert_eq!(trace.validate(), Ok(()));
        assert_eq!(
            trace.tropes.iter().map(|t| t.domain).collect::<Vec<_>>(),
            vec![
                Domain::Mechanical,
                Domain::Epistemic,
                Domain::Affective,
                Domain::Conative,
                Domain::BehaviouralSelection,
                Domain::SocialRelational,
            ]
        );
    }

    #[test]
    fn causality_must_point_backwards() {
        let trace = Trace {
            tropes: vec![trope(1, "agent", "anxiety", Domain::Affective)],
            events: vec![event(1, 0, 1, vec![2]), event(2, 0, 1, vec![])],
        };
        let errors = trace.validate().unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains("not strictly earlier")),
            "expected an ordering fault, got {errors:?}"
        );
    }

    #[test]
    fn an_absent_cause_is_distinguished_from_an_out_of_order_one() {
        // The whole point of the two-pass validator: these were previously one
        // message ("absent or not earlier") and could not be told apart.
        let trace = Trace {
            tropes: vec![trope(1, "agent", "anxiety", Domain::Affective)],
            events: vec![event(1, 5, 1, vec![99])],
        };
        let errors = trace.validate().unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains("not in the trace")),
            "an absent cause must say so: {errors:?}"
        );
        assert!(
            !errors.iter().any(|e| e.contains("not strictly earlier")),
            "an absent cause is not an ordering fault: {errors:?}"
        );
    }

    #[test]
    fn a_shuffled_trace_with_sound_causality_is_valid() {
        // Causality is judged by (tick, id), not by position, so a caller is
        // not required to sort before validating. Under the previous
        // single-pass check this trace failed purely for arriving unsorted.
        let mut trace = appraisal_chain();
        trace.events.reverse();
        assert_eq!(trace.validate(), Ok(()));
        assert!(!trace.is_canonically_ordered());

        trace.canonicalise();
        assert!(trace.is_canonically_ordered());
        assert_eq!(trace.validate(), Ok(()));
    }

    #[test]
    fn an_event_must_be_about_a_declared_trope() {
        let trace = Trace {
            tropes: vec![],
            events: vec![event(1, 0, 7, vec![])],
        };
        let errors = trace.validate().unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains("not declared")),
            "{errors:?}"
        );
    }

    #[test]
    fn two_bearers_of_one_kind_are_distinct_particulars_that_resemble() {
        let billy = Trope {
            about: Some("note".into()),
            ..trope(1, "billy", "suspicion", Domain::Epistemic)
        };
        let dave = Trope {
            about: Some("note".into()),
            ..trope(2, "dave", "suspicion", Domain::Epistemic)
        };

        assert_ne!(
            billy, dave,
            "same kind, different bearer: not the same trope"
        );
        assert!(billy.resembles(&dave));
        assert!(billy.is_peer_of(&dave));
        assert!(!billy.is_peer_of(&billy), "a trope is not its own peer");
    }

    #[test]
    fn resemblance_respects_domain_and_intentional_object() {
        let felt = trope(1, "billy", "alarm", Domain::Affective);
        let known = trope(2, "dave", "alarm", Domain::Epistemic);
        assert!(
            !felt.resembles(&known),
            "feeling alarm is not knowing alarm"
        );

        let about_note = Trope {
            about: Some("note".into()),
            ..trope(3, "billy", "suspicion", Domain::Epistemic)
        };
        let about_usb = Trope {
            about: Some("usb".into()),
            ..trope(4, "dave", "suspicion", Domain::Epistemic)
        };
        assert!(
            !about_note.resembles(&about_usb),
            "different object, different kind of state"
        );
    }

    #[test]
    fn peers_finds_every_other_bearer_of_a_resembling_trope() {
        let trace = Trace {
            tropes: vec![
                trope(1, "billy", "suspicion", Domain::Epistemic),
                trope(2, "dave", "suspicion", Domain::Epistemic),
                trope(3, "erin", "suspicion", Domain::Epistemic),
                trope(4, "dave", "calm", Domain::Affective),
            ],
            events: vec![],
        };
        let billy = &trace.tropes[0];
        let peers: Vec<_> = trace.peers_of(billy).map(|t| t.bearer.as_str()).collect();
        assert_eq!(peers, vec!["dave", "erin"]);
    }

    #[test]
    fn duplicate_ids_are_caught_for_both_tropes_and_events() {
        let trace = Trace {
            tropes: vec![
                trope(1, "agent", "a", Domain::Affective),
                trope(1, "agent", "b", Domain::Affective),
            ],
            events: vec![event(1, 0, 1, vec![]), event(1, 1, 1, vec![])],
        };
        let errors = trace.validate().unwrap_err();
        assert!(
            errors.iter().any(|e| e.contains("duplicate trope id")),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("duplicate trace event id")),
            "{errors:?}"
        );
    }
}
