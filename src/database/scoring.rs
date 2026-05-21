// ─── database/scoring.rs ──────────────────────────────────────────────────────
//
// Frecency = frequency × recency_multiplier × length_weight
//
// Recency bands (seconds since last use):
//   ≤ 1 h   → ×4.0   (very hot)
//   ≤ 1 d   → ×2.0   (warm)
//   ≤ 1 w   → ×0.5   (cooling)
//   > 1 w   → ×0.25  (cold)
//
// Length weight: longer commands save more typing → reward them slightly.
//   weight = length^(3/5)
//
// The formula is intentionally simple so it's easy to reason about,
// test, and tweak without hidden surprises.

use super::structs::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const HOUR: i64  = 3_600;
const DAY:  i64  = 86_400;
const WEEK: i64  = 604_800;

/// Compute frecency score for a command.  Pure function — no side effects.
pub fn compute(cmd: &Command) -> i32 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let age = (now - cmd.last_seen).max(0);

    let recency: f64 = match age {
        a if a <= HOUR => 4.0,
        a if a <= DAY  => 2.0,
        a if a <= WEEK => 0.5,
        _              => 0.25,
    };

    let length_weight = (cmd.length as f64).powf(0.6);
    let freq          = cmd.frequency as f64;

    (recency * length_weight * freq) as i32
}

/// Decay pass: halve all frequencies, recompute scores, prune zero-freq entries.
/// Called when the total score exceeds a threshold to prevent unbounded growth.
pub fn decay_all(commands: impl Iterator<Item = Command>) -> Vec<Command> {
    commands
        .filter_map(|mut cmd| {
            cmd.frequency = (cmd.frequency as f32 * 0.5).round() as u32;
            if cmd.frequency == 0 {
                return None; // pruned
            }
            cmd.score = compute(&cmd);
            Some(cmd)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cmd(freq: u32, age_secs: i64, length: u16) -> Command {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Command {
            score: 0,
            last_seen: now - age_secs,
            frequency: freq,
            length,
            word_count: 1,
            text: "test".into(),
        }
    }

    #[test]
    fn hot_command_scores_higher_than_cold() {
        let hot  = make_cmd(5, 60,     20);  // 1-minute-old
        let cold = make_cmd(5, 10_000, 20);  // ~3 hours old
        assert!(compute(&hot) > compute(&cold));
    }

    #[test]
    fn longer_command_scores_higher() {
        let short = make_cmd(3, 100, 5);
        let long  = make_cmd(3, 100, 50);
        assert!(compute(&long) > compute(&short));
    }

    #[test]
    fn decay_halves_frequency() {
        let cmd = make_cmd(10, 100, 20);
        let decayed = decay_all(std::iter::once(cmd)).pop().unwrap();
        assert_eq!(decayed.frequency, 5);
    }

    #[test]
    fn decay_prunes_zero_freq() {
        let cmd = make_cmd(1, 100, 20); // frequency=1 → halves to 0 → pruned
        let result = decay_all(std::iter::once(cmd));
        assert!(result.is_empty());
    }
}
