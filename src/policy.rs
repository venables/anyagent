//! Permission, network, and enforcement intent.
//!
//! A caller requests a permission/network tier *by intent*; each adapter maps
//! it to the harness's native mechanism and reports the **enforcement class**
//! actually achieved. The honesty principle: where a harness can only enforce a
//! tier by asking the agent nicely (policy) rather than an OS sandbox, we say
//! so -- and `--require-enforcement` lets a caller refuse anything weaker than
//! it demands (exit 32) instead of trusting a uniform-looking flag that lies.

/// Permission tier requested by intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perms {
    ReadOnly,
    WorkspaceWrite,
    Full,
}

/// Network access tier requested by intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    None,
    Restricted,
    Full,
}

/// How strongly a tier is actually enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enforcement {
    /// OS-level sandbox: the agent physically cannot exceed the tier.
    OsSandbox,
    /// Agent policy: enforced by instruction/permission-mode, not the OS.
    AgentPolicy,
    /// Not enforced at all (the tier is requested but nothing stops the agent).
    Unenforced,
}

/// The enforcement class a caller demands via `--require-enforcement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequireEnforcement {
    /// Must be an OS sandbox; anything weaker fails.
    OsSandbox,
    /// Any real enforcement (os-sandbox or agent-policy); only `Unenforced` fails.
    Any,
}

impl Perms {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "read-only" => Some(Self::ReadOnly),
            "workspace-write" => Some(Self::WorkspaceWrite),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::WorkspaceWrite => "workspace-write",
            Self::Full => "full",
        }
    }
}

impl Network {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "none" => Some(Self::None),
            "restricted" => Some(Self::Restricted),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Restricted => "restricted",
            Self::Full => "full",
        }
    }
}

impl Enforcement {
    pub fn label(self) -> &'static str {
        match self {
            Self::OsSandbox => "os-sandbox",
            Self::AgentPolicy => "agent-policy",
            Self::Unenforced => "none",
        }
    }
}

impl RequireEnforcement {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "os-sandbox" => Some(Self::OsSandbox),
            "any" => Some(Self::Any),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::OsSandbox => "os-sandbox",
            Self::Any => "any",
        }
    }

    /// Whether `actual` meets this demand.
    pub fn satisfied_by(self, actual: Enforcement) -> bool {
        match self {
            Self::OsSandbox => actual == Enforcement::OsSandbox,
            Self::Any => actual != Enforcement::Unenforced,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_os_sandbox_only_met_by_os_sandbox() {
        let req = RequireEnforcement::OsSandbox;
        assert!(req.satisfied_by(Enforcement::OsSandbox));
        assert!(!req.satisfied_by(Enforcement::AgentPolicy));
        assert!(!req.satisfied_by(Enforcement::Unenforced));
    }

    #[test]
    fn require_any_rejects_only_unenforced() {
        let req = RequireEnforcement::Any;
        assert!(req.satisfied_by(Enforcement::OsSandbox));
        assert!(req.satisfied_by(Enforcement::AgentPolicy));
        assert!(!req.satisfied_by(Enforcement::Unenforced));
    }

    #[test]
    fn parse_round_trips() {
        assert_eq!(Perms::parse("read-only"), Some(Perms::ReadOnly));
        assert_eq!(Perms::parse("nope"), None);
        assert_eq!(Network::parse("none"), Some(Network::None));
        assert_eq!(RequireEnforcement::parse("any"), Some(RequireEnforcement::Any));
        assert_eq!(Enforcement::OsSandbox.label(), "os-sandbox");
    }
}
