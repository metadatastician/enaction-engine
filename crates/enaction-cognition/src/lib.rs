// SPDX-License-Identifier: AGPL-3.0-or-later
//! A deliberately small, game-neutral event seam.
//!
//! Domains are separate types in the trace: affect never silently becomes
//! knowledge, a belief never silently becomes a goal, and none of them chooses
//! behaviour without an explicit selection event.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Which kind of state one trace event changes or observes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Mechanical,
    Epistemic,
    Affective,
    Conative,
    BehaviouralSelection,
    SocialRelational,
}

/// A game supplies the vocabulary in `state`; the engine supplies ordering,
/// causality and domain separation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub id: u64,
    pub tick: u64,
    pub subject: String,
    pub domain: Domain,
    pub state: String,
    pub value_milli: i32,
    pub caused_by: Vec<u64>,
}

/// Validate the small structural contract without interpreting game words.
pub fn validate_trace(events: &[TraceEvent]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut previous_tick = 0;
    for (index, event) in events.iter().enumerate() {
        if !seen.insert(event.id) {
            errors.push(format!("duplicate trace event id {}", event.id));
        }
        if index > 0 && event.tick < previous_tick {
            errors.push(format!("event {} moves backwards in time", event.id));
        }
        if event.subject.is_empty() || event.state.is_empty() {
            errors.push(format!("event {} has an empty subject or state", event.id));
        }
        for cause in &event.caused_by {
            if !seen.contains(cause) {
                errors.push(format!(
                    "event {} cause {} is absent or not earlier",
                    event.id, cause
                ));
            }
        }
        previous_tick = event.tick;
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_appraisal_chain_keeps_every_domain_explicit() {
        let domains = [
            Domain::Mechanical,
            Domain::Epistemic,
            Domain::Affective,
            Domain::Conative,
            Domain::BehaviouralSelection,
            Domain::SocialRelational,
        ];
        let states = [
            "signal_changed",
            "possible_cause",
            "arousal_changed",
            "goal_priority_changed",
            "verification_selected",
            "protect_relation",
        ];
        let events: Vec<_> = domains
            .into_iter()
            .zip(states)
            .enumerate()
            .map(|(index, (domain, state))| TraceEvent {
                id: index as u64 + 1,
                tick: 12,
                subject: "agent".into(),
                domain,
                state: state.into(),
                value_milli: 500,
                caused_by: if index == 0 {
                    vec![]
                } else {
                    vec![index as u64]
                },
            })
            .collect();
        assert_eq!(validate_trace(&events), Ok(()));
        assert_eq!(
            events.iter().map(|event| event.domain).collect::<Vec<_>>(),
            domains
        );
    }

    #[test]
    fn causality_must_point_backwards() {
        let event = TraceEvent {
            id: 1,
            tick: 0,
            subject: "agent".into(),
            domain: Domain::Affective,
            state: "anxiety".into(),
            value_milli: 100,
            caused_by: vec![2],
        };
        assert!(validate_trace(&[event]).is_err());
    }
}
