use crate::domain::room::Participant;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

const VOCATIVE_MAX_WORDS: usize = 4;
pub const ROUND_BUDGET: usize = 4;
const TEAM_WORDS: [&str; 5] = ["equipo", "team", "todos", "everyone", "all"];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", content = "agent_id", rename_all = "snake_case")]
pub enum Selection {
    Auto(Option<String>),
    Nobody,
    Team,
    Agent(String),
    Confirmed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceReason {
    Ambiguous,
    Several,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Resolution {
    Nobody,
    Agent {
        agent_id: String,
    },
    Proposed {
        agent_id: String,
    },
    Round {
        agent_ids: Vec<String>,
    },
    Choose {
        reason: ChoiceReason,
        candidates: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotParticipant;

pub fn resolve(
    selection: &Selection,
    body: &str,
    mention_ids: &[String],
    participants: &[Participant],
) -> Result<Resolution, NotParticipant> {
    let participates = |id: &str| participants.iter().any(|participant| participant.id == id);
    match selection {
        Selection::Nobody => Ok(Resolution::Nobody),
        Selection::Team => Ok(round(participants)),
        Selection::Confirmed(id) if participates(id) => Ok(Resolution::Agent {
            agent_id: id.clone(),
        }),
        Selection::Agent(id) if participates(id) => {
            let references = references(body, mention_ids, participants);
            if references.iter().all(|candidates| candidates.contains(id)) {
                return Ok(Resolution::Agent {
                    agent_id: id.clone(),
                });
            }
            let mut candidates = vec![id.clone()];
            candidates.extend(distinct(&references));
            Ok(Resolution::Choose {
                reason: ChoiceReason::Conflict,
                candidates: ordered(distinct(&[candidates]), participants),
            })
        }
        Selection::Confirmed(_) | Selection::Agent(_) => Err(NotParticipant),
        Selection::Auto(proposed) => {
            let references = references(body, mention_ids, participants);
            let candidates = ordered(distinct(&references), participants);
            if candidates.is_empty() && team_vocative(body) {
                return Ok(round(participants));
            }
            Ok(match candidates.as_slice() {
                [] => match proposed {
                    Some(id) if participates(id) => Resolution::Proposed {
                        agent_id: id.clone(),
                    },
                    Some(_) => return Err(NotParticipant),
                    None => Resolution::Nobody,
                },
                [only] if references.iter().all(|found| found.len() == 1) => Resolution::Agent {
                    agent_id: only.clone(),
                },
                _ => Resolution::Choose {
                    reason: if references.iter().all(|found| found.len() == 1) {
                        ChoiceReason::Several
                    } else {
                        ChoiceReason::Ambiguous
                    },
                    candidates,
                },
            })
        }
    }
}

fn round(participants: &[Participant]) -> Resolution {
    if participants.is_empty() {
        return Resolution::Nobody;
    }
    Resolution::Round {
        agent_ids: participants
            .iter()
            .take(ROUND_BUDGET)
            .map(|participant| participant.id.clone())
            .collect(),
    }
}

fn team_vocative(body: &str) -> bool {
    vocative(body).is_some_and(|head| TEAM_WORDS.contains(&head.as_str()))
}

fn references(
    body: &str,
    mention_ids: &[String],
    participants: &[Participant],
) -> Vec<Vec<String>> {
    let mentions: Vec<Vec<String>> = mention_starts(body)
        .filter_map(|start| {
            let found = longest_matches(&normalize(&body[start..]), participants, false);
            let selected: Vec<String> = found
                .iter()
                .filter(|id| mention_ids.contains(id))
                .cloned()
                .collect();
            let found = if selected.is_empty() { found } else { selected };
            (!found.is_empty()).then_some(found)
        })
        .collect();
    if !mentions.is_empty() {
        return mentions;
    }
    vocative(body)
        .map(|head| longest_matches(&head, participants, true))
        .filter(|found| !found.is_empty())
        .into_iter()
        .collect()
}

fn mention_starts(body: &str) -> impl Iterator<Item = usize> + '_ {
    body.char_indices().filter_map(move |(index, character)| {
        let previous = body[..index].chars().next_back();
        (character == '@' && !previous.is_some_and(char::is_alphanumeric))
            .then_some(index + character.len_utf8())
    })
}

fn vocative(body: &str) -> Option<String> {
    let body = body.trim_start();
    let end = body.find([',', ':'])?;
    let head = normalize(&body[..end]);
    let words = head.split_whitespace().count();
    (1..=VOCATIVE_MAX_WORDS)
        .contains(&words)
        .then(|| head.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn longest_matches(text: &str, participants: &[Participant], whole: bool) -> Vec<String> {
    let scored: Vec<(usize, &Participant)> = participants
        .iter()
        .map(|participant| {
            let score = [&participant.name, &participant.role]
                .into_iter()
                .flat_map(|label| prefixes(&normalize(label)))
                .filter(|prefix| matches_at_start(text, prefix, whole))
                .map(|prefix| prefix.chars().count())
                .max()
                .unwrap_or(0);
            (score, participant)
        })
        .collect();
    let best = scored.iter().map(|(score, _)| *score).max().unwrap_or(0);
    if best == 0 {
        return Vec::new();
    }
    scored
        .into_iter()
        .filter(|(score, _)| *score == best)
        .map(|(_, participant)| participant.id.clone())
        .collect()
}

fn prefixes(label: &str) -> Vec<String> {
    let words: Vec<&str> = label.split_whitespace().collect();
    (1..=words.len())
        .map(|count| words[..count].join(" "))
        .collect()
}

fn matches_at_start(text: &str, prefix: &str, whole: bool) -> bool {
    if whole {
        return text == prefix;
    }
    text.strip_prefix(prefix)
        .is_some_and(|rest| !rest.chars().next().is_some_and(char::is_alphanumeric))
}

fn normalize(text: &str) -> String {
    text.nfd()
        .filter(|character| !is_combining_mark(*character))
        .flat_map(char::to_lowercase)
        .collect()
}

fn distinct(groups: &[Vec<String>]) -> Vec<String> {
    let mut unique: Vec<String> = Vec::new();
    for id in groups.iter().flatten() {
        if !unique.contains(id) {
            unique.push(id.clone());
        }
    }
    unique
}

fn ordered(ids: Vec<String>, participants: &[Participant]) -> Vec<String> {
    participants
        .iter()
        .filter(|participant| ids.contains(&participant.id))
        .map(|participant| participant.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participant(id: &str, name: &str, role: &str) -> Participant {
        Participant {
            id: id.into(),
            name: name.into(),
            role: role.into(),
            provider: "ollama".into(),
            model: "llama3".into(),
        }
    }

    fn room() -> Vec<Participant> {
        vec![
            participant("be", "Bruno", "Backend Developer"),
            participant("po", "Paula", "Product Owner"),
            participant("qa", "Quique", "QA Engineer"),
        ]
    }

    fn auto(body: &str) -> Resolution {
        resolve(&Selection::Auto(None), body, &[], &room()).unwrap()
    }

    fn agent(id: &str) -> Resolution {
        Resolution::Agent {
            agent_id: id.into(),
        }
    }

    fn choose(reason: ChoiceReason, candidates: &[&str]) -> Resolution {
        Resolution::Choose {
            reason,
            candidates: candidates.iter().map(|id| id.to_string()).collect(),
        }
    }

    #[test]
    fn a_mention_by_name_or_role_resolves_one_agent_ignoring_case_and_accents() {
        assert_eq!(auto("@Bruno, how would you do OAuth?"), agent("be"));
        assert_eq!(auto("What do you think, @backend?"), agent("be"));
        assert_eq!(auto("@BACKEND DEVELOPER ¿qué opinas?"), agent("be"));
        assert_eq!(auto("@qa engineer"), agent("qa"));
        let accented = vec![participant("jo", "José", "Tech Lead")];
        assert_eq!(
            resolve(&Selection::Auto(None), "@jose revisa esto", &[], &accented),
            Ok(agent("jo"))
        );
        assert_eq!(
            resolve(&Selection::Auto(None), "@JOSÉ", &[], &accented),
            Ok(agent("jo"))
        );
    }

    #[test]
    fn an_unknown_mention_or_an_email_is_plain_text() {
        assert_eq!(auto("@Marta can you help?"), Resolution::Nobody);
        assert_eq!(auto("Write to bruno@backend.dev"), Resolution::Nobody);
        assert_eq!(auto("@Brunoo"), Resolution::Nobody);
        assert_eq!(auto("A plain message"), Resolution::Nobody);
    }

    #[test]
    fn repeated_names_are_ambiguous_unless_the_autocomplete_chose_one() {
        let twins = vec![
            participant("b1", "Backend", "Backend Developer"),
            participant("b2", "Backend", "Software Developer"),
        ];
        assert_eq!(
            resolve(&Selection::Auto(None), "@Backend hi", &[], &twins),
            Ok(choose(ChoiceReason::Ambiguous, &["b1", "b2"]))
        );
        assert_eq!(
            resolve(
                &Selection::Auto(None),
                "@Backend hi",
                &["b2".into()],
                &twins
            ),
            Ok(agent("b2"))
        );
        assert_eq!(
            resolve(&Selection::Auto(None), "@Backend Developer hi", &[], &twins),
            Ok(agent("b1"))
        );
    }

    #[test]
    fn selected_ids_outside_the_room_never_resolve() {
        assert_eq!(
            resolve(
                &Selection::Auto(None),
                "@Marta hi",
                &["outsider".into()],
                &room()
            ),
            Ok(Resolution::Nobody)
        );
        assert_eq!(
            resolve(&Selection::Agent("outsider".into()), "hi", &[], &room()),
            Err(NotParticipant)
        );
        assert_eq!(
            resolve(&Selection::Confirmed("outsider".into()), "hi", &[], &room()),
            Err(NotParticipant)
        );
    }

    #[test]
    fn several_distinct_agents_must_be_chosen() {
        assert_eq!(
            auto("@Quique and @Bruno, review it"),
            choose(ChoiceReason::Several, &["be", "qa"])
        );
        assert_eq!(auto("@Bruno @backend"), agent("be"));
    }

    fn round_of(ids: &[&str]) -> Resolution {
        Resolution::Round {
            agent_ids: ids.iter().map(|id| id.to_string()).collect(),
        }
    }

    #[test]
    fn the_team_selection_makes_a_round_in_room_order() {
        assert_eq!(
            resolve(&Selection::Team, "What do you think?", &[], &room()),
            Ok(round_of(&["be", "po", "qa"]))
        );
        assert_eq!(
            resolve(&Selection::Team, "Anyone?", &[], &[]),
            Ok(Resolution::Nobody)
        );
    }

    #[test]
    fn a_round_never_exceeds_the_budget() {
        let crowd: Vec<Participant> = (0..6)
            .map(|index| participant(&format!("a{index}"), &format!("Agent{index}"), "Dev"))
            .collect();
        assert_eq!(
            resolve(&Selection::Team, "Hi", &[], &crowd),
            Ok(round_of(&["a0", "a1", "a2", "a3"]))
        );
    }

    #[test]
    fn a_team_vocative_makes_a_round_unless_someone_is_mentioned() {
        assert_eq!(auto("Equipo, ¿qué opinan?"), round_of(&["be", "po", "qa"]));
        assert_eq!(auto("  Team: thoughts?"), round_of(&["be", "po", "qa"]));
        assert_eq!(auto("Todos, a revisar"), round_of(&["be", "po", "qa"]));
        assert_eq!(auto("@Bruno, team: thoughts?"), agent("be"));
        assert_eq!(auto("The team is late"), Resolution::Nobody);
    }

    #[test]
    fn a_vocative_at_the_start_resolves_when_it_is_unequivocal() {
        assert_eq!(auto("Backend, how would you do OAuth?"), agent("be"));
        assert_eq!(auto("  paula: prioritise this"), agent("po"));
        assert_eq!(auto("Product Owner, what now?"), agent("po"));
        assert_eq!(auto("Well, Backend should do it"), Resolution::Nobody);
        assert_eq!(auto("Backend should do it, right?"), Resolution::Nobody);
        let devs = vec![
            participant("a", "Ana", "Backend Developer"),
            participant("b", "Bea", "Backend Engineer"),
        ];
        assert_eq!(
            resolve(&Selection::Auto(None), "Backend, hi", &[], &devs),
            Ok(choose(ChoiceReason::Ambiguous, &["a", "b"]))
        );
    }

    #[test]
    fn the_proposal_applies_only_when_the_message_names_nobody() {
        let following = Selection::Auto(Some("be".into()));
        let proposed = Resolution::Proposed {
            agent_id: "be".into(),
        };
        assert_eq!(resolve(&following, "Why PKCE?", &[], &room()), Ok(proposed));
        assert_eq!(
            resolve(&following, "@Quique why?", &[], &room()),
            Ok(agent("qa"))
        );
        assert_eq!(
            resolve(&following, "Paula, why?", &[], &room()),
            Ok(agent("po"))
        );
        assert_eq!(
            resolve(&following, "@Backend Developer and @Paula", &[], &room()),
            Ok(choose(ChoiceReason::Several, &["be", "po"]))
        );
        assert_eq!(
            resolve(&Selection::Nobody, "Why PKCE?", &[], &room()),
            Ok(Resolution::Nobody)
        );
    }

    #[test]
    fn a_proposal_outside_the_room_is_rejected() {
        assert_eq!(
            resolve(&Selection::Auto(Some("gone".into())), "Why?", &[], &room()),
            Err(NotParticipant)
        );
    }

    #[test]
    fn mentions_take_precedence_over_a_vocative() {
        assert_eq!(auto("Bruno, ask @QA about it"), agent("qa"));
    }

    #[test]
    fn explicit_nobody_keeps_mentions_as_text() {
        assert_eq!(
            resolve(
                &Selection::Nobody,
                "@Bruno look at this",
                &["be".into()],
                &room()
            ),
            Ok(Resolution::Nobody)
        );
    }

    #[test]
    fn an_explicit_agent_wins_unless_a_mention_points_elsewhere() {
        let selected = Selection::Agent("be".into());
        assert_eq!(resolve(&selected, "How?", &[], &room()), Ok(agent("be")));
        assert_eq!(
            resolve(&selected, "@Bruno how?", &[], &room()),
            Ok(agent("be"))
        );
        assert_eq!(
            resolve(&selected, "Paula, hi", &[], &room()),
            Ok(choose(ChoiceReason::Conflict, &["be", "po"]))
        );
        assert_eq!(
            resolve(&selected, "@Quique how?", &[], &room()),
            Ok(choose(ChoiceReason::Conflict, &["be", "qa"]))
        );
    }

    #[test]
    fn a_confirmed_choice_settles_the_conflict() {
        assert_eq!(
            resolve(
                &Selection::Confirmed("be".into()),
                "@Quique and @Paula",
                &[],
                &room()
            ),
            Ok(agent("be"))
        );
    }

    #[test]
    fn the_resolution_is_written_for_the_ui() {
        let json = |resolution: Resolution| serde_json::to_value(resolution).unwrap();
        assert_eq!(
            json(Resolution::Nobody),
            serde_json::json!({ "kind": "nobody" })
        );
        assert_eq!(
            json(agent("be")),
            serde_json::json!({ "kind": "agent", "agent_id": "be" })
        );
        assert_eq!(
            json(choose(ChoiceReason::Several, &["be", "qa"])),
            serde_json::json!({ "kind": "choose", "reason": "several", "candidates": ["be", "qa"] })
        );
    }

    #[test]
    fn the_selection_is_read_from_the_ui_payload() {
        let parse = |json: &str| serde_json::from_str::<Selection>(json).unwrap();
        assert_eq!(
            parse(r#"{"kind":"auto","agent_id":null}"#),
            Selection::Auto(None)
        );
        assert_eq!(
            parse(r#"{"kind":"auto","agent_id":"be"}"#),
            Selection::Auto(Some("be".into()))
        );
        assert_eq!(parse(r#"{"kind":"nobody"}"#), Selection::Nobody);
        assert_eq!(
            parse(r#"{"kind":"agent","agent_id":"be"}"#),
            Selection::Agent("be".into())
        );
        assert_eq!(
            parse(r#"{"kind":"confirmed","agent_id":"be"}"#),
            Selection::Confirmed("be".into())
        );
    }
}
