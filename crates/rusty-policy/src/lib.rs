use std::collections::HashSet;

use rusty_core::action::ActionKind;
use rusty_core::capability::Capability;
use rusty_core::policy::{PolicyConfig, PolicyEffect, PolicyRule};

#[derive(Debug, Clone)]
pub struct PolicyContext {
    pub plugin_id: String,
    pub action_id: String,
    pub action_kind: ActionKind,
    pub capabilities: HashSet<Capability>,
    pub tags: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub effect: PolicyEffect,
    pub matched_rule: Option<String>,
    pub reason: String,
}

pub struct PolicyEngine {
    config: PolicyConfig,
}

impl PolicyEngine {
    pub fn new(config: PolicyConfig) -> Self {
        Self { config }
    }

    pub fn evaluate(&self, ctx: &PolicyContext) -> PolicyDecision {
        for rule in &self.config.rules {
            if rule_matches(rule, ctx) {
                return PolicyDecision {
                    effect: rule.effect,
                    matched_rule: Some(rule.id.clone()),
                    reason: rule
                        .description
                        .clone()
                        .unwrap_or_else(|| format!("matched rule '{}'", rule.id)),
                };
            }
        }

        PolicyDecision {
            effect: self.config.default_effect,
            matched_rule: None,
            reason: format!("no rule matched, using default: {}", self.config.default_effect),
        }
    }
}

fn rule_matches(rule: &PolicyRule, ctx: &PolicyContext) -> bool {
    if let Some(ref kind) = rule.match_action_kind {
        if ctx.action_kind != *kind {
            return false;
        }
    }
    if let Some(ref cap) = rule.match_capability {
        if !ctx.capabilities.contains(cap) {
            return false;
        }
    }
    if let Some(ref tag) = rule.match_tag {
        if !ctx.tags.contains(tag) {
            return false;
        }
    }
    if let Some(ref pid) = rule.match_plugin_id {
        if ctx.plugin_id != *pid {
            return false;
        }
    }
    if let Some(ref aid) = rule.match_action_id {
        if ctx.action_id != *aid {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> PolicyContext {
        PolicyContext {
            plugin_id: "test-plugin".into(),
            action_id: "do-stuff".into(),
            action_kind: ActionKind::Mutating,
            capabilities: HashSet::from([Capability::NetworkFetch]),
            tags: HashSet::from(["network".into()]),
        }
    }

    #[test]
    fn default_allows_when_no_rules() {
        let engine = PolicyEngine::new(PolicyConfig::default());
        let decision = engine.evaluate(&make_ctx());
        assert_eq!(decision.effect, PolicyEffect::Allow);
        assert!(decision.matched_rule.is_none());
    }

    #[test]
    fn first_matching_rule_wins() {
        let config = PolicyConfig {
            default_effect: PolicyEffect::Allow,
            rules: vec![
                PolicyRule {
                    id: "deny-mutating".into(),
                    description: Some("Deny mutating actions".into()),
                    effect: PolicyEffect::Deny,
                    match_action_kind: Some(ActionKind::Mutating),
                    match_capability: None,
                    match_tag: None,
                    match_plugin_id: None,
                    match_action_id: None,
                },
                PolicyRule {
                    id: "allow-network".into(),
                    description: None,
                    effect: PolicyEffect::Allow,
                    match_action_kind: None,
                    match_capability: Some(Capability::NetworkFetch),
                    match_tag: None,
                    match_plugin_id: None,
                    match_action_id: None,
                },
            ],
        };
        let engine = PolicyEngine::new(config);
        let decision = engine.evaluate(&make_ctx());
        assert_eq!(decision.effect, PolicyEffect::Deny);
        assert_eq!(decision.matched_rule.as_deref(), Some("deny-mutating"));
    }

    #[test]
    fn deny_by_default() {
        let config = PolicyConfig {
            default_effect: PolicyEffect::Deny,
            rules: vec![PolicyRule {
                id: "allow-read".into(),
                description: None,
                effect: PolicyEffect::Allow,
                match_action_kind: Some(ActionKind::ReadOnly),
                match_capability: None,
                match_tag: None,
                match_plugin_id: None,
                match_action_id: None,
            }],
        };
        let engine = PolicyEngine::new(config);
        // Mutating action should not match the read-only rule
        let decision = engine.evaluate(&make_ctx());
        assert_eq!(decision.effect, PolicyEffect::Deny);
    }

    #[test]
    fn match_by_plugin_id() {
        let config = PolicyConfig {
            default_effect: PolicyEffect::Allow,
            rules: vec![PolicyRule {
                id: "deny-test-plugin".into(),
                description: None,
                effect: PolicyEffect::Deny,
                match_action_kind: None,
                match_capability: None,
                match_tag: None,
                match_plugin_id: Some("test-plugin".into()),
                match_action_id: None,
            }],
        };
        let engine = PolicyEngine::new(config);
        let decision = engine.evaluate(&make_ctx());
        assert_eq!(decision.effect, PolicyEffect::Deny);
    }

    #[test]
    fn require_approval_for_destructive() {
        let config = PolicyConfig {
            default_effect: PolicyEffect::Allow,
            rules: vec![PolicyRule {
                id: "approve-destructive".into(),
                description: None,
                effect: PolicyEffect::RequireApproval,
                match_action_kind: Some(ActionKind::Destructive),
                match_capability: None,
                match_tag: None,
                match_plugin_id: None,
                match_action_id: None,
            }],
        };
        let engine = PolicyEngine::new(config);

        let mut ctx = make_ctx();
        ctx.action_kind = ActionKind::Destructive;
        let decision = engine.evaluate(&ctx);
        assert_eq!(decision.effect, PolicyEffect::RequireApproval);
    }

    #[test]
    fn rule_with_multiple_conditions_requires_all() {
        let config = PolicyConfig {
            default_effect: PolicyEffect::Allow,
            rules: vec![PolicyRule {
                id: "deny-mutating-network".into(),
                description: None,
                effect: PolicyEffect::Deny,
                match_action_kind: Some(ActionKind::Mutating),
                match_capability: Some(Capability::NetworkFetch),
                match_tag: None,
                match_plugin_id: None,
                match_action_id: None,
            }],
        };
        let engine = PolicyEngine::new(config);

        // Both conditions met
        let decision = engine.evaluate(&make_ctx());
        assert_eq!(decision.effect, PolicyEffect::Deny);

        // Only action kind matches, missing capability
        let mut ctx2 = make_ctx();
        ctx2.capabilities.clear();
        let decision2 = engine.evaluate(&ctx2);
        assert_eq!(decision2.effect, PolicyEffect::Allow); // falls to default
    }
}
