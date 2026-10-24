//! Pure node-selection logic — no I/O, no display.
//!
//! Turns the raw nodedb into a [`Plan`]: the list of nodes to show and which
//! of them are scheduled for removal, following the user's [`Selection`].

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::time::Duration;

use clap::ValueEnum;
use meshtastic::protobufs::{HardwareModel, NodeInfo};

/// A nodedb entry, normalised for selection and display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub num: u32,
    pub long_name: Option<String>,
    pub short_name: Option<String>,
    pub hw_model: Option<String>,
    /// Unix time of the last packet heard, `None` when the device never
    /// recorded one (e.g. it had no valid clock at the time).
    pub last_heard: Option<u32>,
    /// The device never received a complete NodeInfo for this node.
    pub unknown: bool,
    pub favorite: bool,
    pub ignored: bool,
    /// Matches the selection but is kept because it is a favorite or ignored.
    pub protected: bool,
    /// Scheduled for removal.
    pub delete: bool,
}

impl Node {
    pub fn from_info(info: &NodeInfo) -> Self {
        let user = info.user.as_ref();
        let non_empty = |s: &str| (!s.is_empty()).then(|| s.to_string());
        Node {
            num: info.num,
            long_name: user.and_then(|u| non_empty(&u.long_name)),
            short_name: user.and_then(|u| non_empty(&u.short_name)),
            hw_model: user
                .filter(|u| u.hw_model != HardwareModel::Unset as i32)
                .map(|u| match HardwareModel::try_from(u.hw_model) {
                    Ok(hw) => hw.as_str_name().to_string(),
                    Err(_) => format!("#{}", u.hw_model),
                }),
            last_heard: (info.last_heard != 0).then_some(info.last_heard),
            unknown: user.is_none_or(|u| u.hw_model == HardwareModel::Unset as i32),
            favorite: info.is_favorite,
            ignored: info.is_ignored,
            protected: false,
            delete: false,
        }
    }

    /// Node id in the usual Meshtastic `!aabbccdd` form.
    pub fn id(&self) -> String {
        node_id(self.num)
    }
}

pub fn node_id(num: u32) -> String {
    format!("!{num:08x}")
}

/// Which nodes the user wants removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// Nodes matching every given criterion.
    Filter {
        unknown: bool,
        older_than: Option<Duration>,
        /// With `older_than`, also match nodes with no last-heard time.
        include_never: bool,
    },
    /// Exactly these node numbers.
    Explicit(Vec<u32>),
}

/// Why a node explicitly requested with `--remove` will not be removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    NotFound(u32),
    Own(u32),
    Favorite(u32),
    Ignored(u32),
}

/// Result of applying a [`Selection`] to the nodedb.
#[derive(Debug, Default)]
pub struct Plan {
    /// The device's own node, never removed.
    pub own: Option<Node>,
    /// Every other node to display (only the requested ones for
    /// [`Selection::Explicit`]).
    pub nodes: Vec<Node>,
    /// Explicitly requested nodes that will not be removed.
    pub skipped: Vec<Skip>,
}

impl Plan {
    pub fn to_delete(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(|n| n.delete)
    }

    pub fn delete_count(&self) -> usize {
        self.to_delete().count()
    }

    pub fn protected_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.protected).count()
    }

    /// Nodes that `--older-than` left alone only because they have no
    /// last-heard time.
    pub fn never_heard_kept(&self, selection: &Selection) -> usize {
        match selection {
            Selection::Filter {
                older_than: Some(_),
                include_never: false,
                ..
            } => self
                .nodes
                .iter()
                .filter(|n| n.last_heard.is_none() && !n.delete)
                .count(),
            _ => 0,
        }
    }
}

/// Build the removal plan for `nodedb`.
///
/// `now` is the current Unix time.  Favorites and ignored nodes are never
/// removed: both flags live in the nodedb entry itself, so removing the node
/// would also drop them.
pub fn plan(
    nodedb: &BTreeMap<u32, NodeInfo>,
    my_node_num: u32,
    selection: &Selection,
    now: u64,
) -> Plan {
    let own = nodedb.get(&my_node_num).map(Node::from_info);
    let mut plan = Plan {
        own,
        ..Plan::default()
    };

    match selection {
        Selection::Filter {
            unknown,
            older_than,
            include_never,
        } => {
            for info in nodedb.values().filter(|n| n.num != my_node_num) {
                let mut node = Node::from_info(info);
                let stale = older_than.is_none_or(|age| match node.last_heard {
                    Some(t) => now.saturating_sub(t.into()) > age.as_secs(),
                    None => *include_never,
                });
                let matches = (!unknown || node.unknown) && stale;
                node.protected = matches && (node.favorite || node.ignored);
                node.delete = matches && !node.protected;
                plan.nodes.push(node);
            }
        }
        Selection::Explicit(nums) => {
            let mut seen = Vec::new();
            for &num in nums {
                if seen.contains(&num) {
                    continue;
                }
                seen.push(num);
                let Some(info) = nodedb.get(&num) else {
                    plan.skipped.push(Skip::NotFound(num));
                    continue;
                };
                if num == my_node_num {
                    plan.skipped.push(Skip::Own(num));
                    continue;
                }
                let mut node = Node::from_info(info);
                if node.favorite {
                    plan.skipped.push(Skip::Favorite(num));
                    node.protected = true;
                } else if node.ignored {
                    plan.skipped.push(Skip::Ignored(num));
                    node.protected = true;
                } else {
                    node.delete = true;
                }
                plan.nodes.push(node);
            }
        }
    }
    plan
}

/// Column to sort the node table by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum SortKey {
    #[default]
    Date,
    Name,
    Id,
    Hardware,
}

/// Sort nodes in place.
///
/// Dates sort newest first (never heard last) and text columns A→Z;
/// `reverse` flips either order.  Ties keep a stable order by node number.
pub fn sort_nodes(nodes: &mut [Node], key: SortKey, reverse: bool) {
    let text = |s: &Option<String>| s.as_deref().map(str::to_lowercase);
    nodes.sort_by_key(|n| n.num);
    match key {
        SortKey::Date => nodes.sort_by_key(|n| Reverse(n.last_heard)),
        // Nodes without the field go last in A→Z order.
        SortKey::Name => nodes.sort_by_key(|n| (n.long_name.is_none(), text(&n.long_name))),
        SortKey::Hardware => nodes.sort_by_key(|n| (n.hw_model.is_none(), text(&n.hw_model))),
        SortKey::Id => {}
    }
    if reverse {
        nodes.reverse();
    }
}

/// Format a duration in seconds as e.g. `3d 2h 15m`, omitting zero parts
/// but never returning an empty string.
pub fn human_delta(secs: u64) -> String {
    let (days, hours, minutes) = (secs / 86_400, secs % 86_400 / 3_600, secs % 3_600 / 60);
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 || parts.is_empty() {
        parts.push(format!("{minutes}m"));
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use meshtastic::protobufs::User;
    use rstest::rstest;

    const NOW: u64 = 1_800_000_000;
    const DAY: u32 = 86_400;
    const ME: u32 = 0x1;

    fn info(num: u32, hw: Option<HardwareModel>, last_heard: u32) -> NodeInfo {
        NodeInfo {
            num,
            user: hw.map(|hw| User {
                id: node_id(num),
                long_name: format!("Node {num}"),
                short_name: format!("N{num}"),
                hw_model: hw as i32,
                ..Default::default()
            }),
            last_heard,
            ..Default::default()
        }
    }

    fn ago(days: u32) -> u32 {
        NOW as u32 - days * DAY
    }

    /// Own node, plus known/unknown nodes heard recently, long ago and never.
    fn nodedb() -> BTreeMap<u32, NodeInfo> {
        let hw = Some(HardwareModel::HeltecV3);
        let unset = Some(HardwareModel::Unset);
        [
            info(ME, hw, 0),
            info(0x10, hw, ago(1)),
            info(0x11, hw, ago(30)),
            info(0x12, hw, 0),
            info(0x20, None, ago(1)),
            info(0x21, unset, ago(30)),
            info(0x22, None, 0),
        ]
        .into_iter()
        .map(|n| (n.num, n))
        .collect()
    }

    fn deleted(plan: &Plan) -> Vec<u32> {
        plan.to_delete().map(|n| n.num).collect()
    }

    fn filter(unknown: bool, older_days: Option<u64>, include_never: bool) -> Selection {
        Selection::Filter {
            unknown,
            older_than: older_days.map(|d| Duration::from_secs(d * 86_400)),
            include_never,
        }
    }

    #[test]
    fn from_info_known() {
        let n = Node::from_info(&info(0xaabbccdd, Some(HardwareModel::Tbeam), 42));
        assert_eq!(n.id(), "!aabbccdd");
        assert_eq!(n.long_name.as_deref(), Some("Node 2864434397"));
        assert_eq!(n.hw_model.as_deref(), Some("TBEAM"));
        assert_eq!(n.last_heard, Some(42));
        assert!(!n.unknown);
    }

    #[rstest]
    #[case::no_user(None)]
    #[case::hw_unset(Some(HardwareModel::Unset))]
    fn from_info_unknown(#[case] hw: Option<HardwareModel>) {
        let n = Node::from_info(&info(5, hw, 0));
        assert!(n.unknown);
        assert_eq!(n.hw_model, None);
        assert_eq!(n.last_heard, None);
    }

    #[rstest]
    #[case::unknown_only(filter(true, None, false), vec![0x20, 0x21, 0x22])]
    #[case::older(filter(false, Some(7), false), vec![0x11, 0x21])]
    #[case::older_include_never(filter(false, Some(7), true), vec![0x11, 0x12, 0x21, 0x22])]
    #[case::unknown_and_older(filter(true, Some(7), false), vec![0x21])]
    #[case::unknown_and_older_never(filter(true, Some(7), true), vec![0x21, 0x22])]
    fn filter_selection(#[case] selection: Selection, #[case] expected: Vec<u32>) {
        let plan = plan(&nodedb(), ME, &selection, NOW);
        assert_eq!(deleted(&plan), expected);
    }

    #[test]
    fn own_node_is_separate_and_never_deleted() {
        let plan = plan(&nodedb(), ME, &filter(false, Some(0), true), NOW);
        assert_eq!(plan.own.as_ref().map(|n| n.num), Some(ME));
        assert!(plan.nodes.iter().all(|n| n.num != ME));
    }

    #[test]
    fn favorites_and_ignored_are_protected() {
        let mut db = nodedb();
        db.get_mut(&0x20).unwrap().is_favorite = true;
        db.get_mut(&0x21).unwrap().is_ignored = true;
        let plan = plan(&db, ME, &filter(true, None, false), NOW);
        assert_eq!(deleted(&plan), vec![0x22]);
        assert_eq!(plan.protected_count(), 2);
    }

    #[test]
    fn never_heard_kept_is_reported_only_for_older_than() {
        let db = nodedb();
        let sel = filter(false, Some(7), false);
        assert_eq!(plan(&db, ME, &sel, NOW).never_heard_kept(&sel), 2);
        let sel = filter(false, Some(7), true);
        assert_eq!(plan(&db, ME, &sel, NOW).never_heard_kept(&sel), 0);
        let sel = filter(true, None, false);
        assert_eq!(plan(&db, ME, &sel, NOW).never_heard_kept(&sel), 0);
    }

    #[test]
    fn explicit_selection() {
        let mut db = nodedb();
        db.get_mut(&0x11).unwrap().is_favorite = true;
        db.get_mut(&0x12).unwrap().is_ignored = true;
        let sel = Selection::Explicit(vec![0x10, 0x10, 0x11, 0x12, ME, 0x99]);
        let plan = plan(&db, ME, &sel, NOW);
        assert_eq!(deleted(&plan), vec![0x10]);
        assert_eq!(
            plan.nodes.iter().map(|n| n.num).collect::<Vec<_>>(),
            vec![0x10, 0x11, 0x12]
        );
        assert_eq!(
            plan.skipped,
            vec![
                Skip::Favorite(0x11),
                Skip::Ignored(0x12),
                Skip::Own(ME),
                Skip::NotFound(0x99)
            ]
        );
    }

    fn sorted(key: SortKey, reverse: bool) -> Vec<u32> {
        let mut nodes: Vec<Node> = nodedb().values().map(Node::from_info).collect();
        sort_nodes(&mut nodes, key, reverse);
        nodes.iter().map(|n| n.num).collect()
    }

    #[test]
    fn sort_by_date_newest_first_never_last() {
        assert_eq!(
            sorted(SortKey::Date, false),
            vec![0x10, 0x20, 0x11, 0x21, ME, 0x12, 0x22]
        );
        assert_eq!(
            sorted(SortKey::Date, true),
            vec![0x22, 0x12, ME, 0x21, 0x11, 0x20, 0x10]
        );
    }

    #[test]
    fn sort_by_name_missing_last() {
        // "Node 1" < "Node 16" < "Node 17" < "Node 18" < "Node 33"; 0x20/0x22 have no user.
        assert_eq!(
            sorted(SortKey::Name, false),
            vec![ME, 0x10, 0x11, 0x12, 0x21, 0x20, 0x22]
        );
    }

    #[test]
    fn sort_by_id() {
        assert_eq!(
            sorted(SortKey::Id, false),
            vec![ME, 0x10, 0x11, 0x12, 0x20, 0x21, 0x22]
        );
        assert_eq!(
            sorted(SortKey::Id, true),
            vec![0x22, 0x21, 0x20, 0x12, 0x11, 0x10, ME]
        );
    }

    #[test]
    fn sort_by_hardware_missing_last() {
        let order = sorted(SortKey::Hardware, false);
        assert_eq!(&order[4..], &[0x20, 0x21, 0x22]);
    }

    #[rstest]
    #[case(0, "0m")]
    #[case(59, "0m")]
    #[case(60, "1m")]
    #[case(3_600, "1h")]
    #[case(86_400, "1d")]
    #[case(3 * 86_400 + 2 * 3_600 + 15 * 60, "3d 2h 15m")]
    #[case(86_400 + 60, "1d 1m")]
    fn human_delta_formats(#[case] secs: u64, #[case] expected: &str) {
        assert_eq!(human_delta(secs), expected);
    }
}
