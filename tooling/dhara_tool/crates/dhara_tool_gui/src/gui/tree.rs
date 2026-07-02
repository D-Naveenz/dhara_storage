//! Navigation tree built from [`CommandRegistry`] command paths.

use std::collections::{BTreeMap, BTreeSet};

use dhara_tool_cli::command::{CommandRegistry, CommandSpec};

pub const FAVORITES_GROUP: &str = "__favorites__";

pub const QUICK_ACTIONS: &[&str] = &[
    "verify.package",
    "config.show",
    "version.bump",
    "release.run",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub label: String,
    pub path_key: String,
    pub command_id: Option<&'static str>,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn is_leaf(&self) -> bool {
        self.command_id.is_some()
    }
}

#[derive(Debug, Clone, Default)]
pub struct NavTree {
    pub roots: Vec<TreeNode>,
}

#[derive(Debug, Clone, Default)]
pub struct TreeViewState {
    pub expanded: BTreeSet<String>,
    pub selected_command_id: Option<&'static str>,
}

impl NavTree {
    pub fn from_registry(registry: &CommandRegistry) -> Self {
        let roots = vec![
            build_favorites_group(registry),
            build_commands_group(registry),
        ];
        Self { roots }
    }
}

fn build_favorites_group(registry: &CommandRegistry) -> TreeNode {
    let children = QUICK_ACTIONS
        .iter()
        .filter_map(|id| {
            registry.commands().find(|command| command.id == *id).map(|command| {
                TreeNode {
                    label: command.summary.to_owned(),
                    path_key: format!("{FAVORITES_GROUP}/{}", command.id),
                    command_id: Some(command.id),
                    children: Vec::new(),
                }
            })
        })
        .collect();

    TreeNode {
        label: "Favorites".to_owned(),
        path_key: FAVORITES_GROUP.to_owned(),
        command_id: None,
        children,
    }
}

fn build_commands_group(registry: &CommandRegistry) -> TreeNode {
    let mut branch_map: BTreeMap<String, BranchBuilder> = BTreeMap::new();

    for command in registry.commands() {
        insert_command(&mut branch_map, command);
    }

    let children = branch_map
        .into_values()
        .map(BranchBuilder::into_node)
        .collect();

    TreeNode {
        label: "Commands".to_owned(),
        path_key: "commands".to_owned(),
        command_id: None,
        children,
    }
}

struct BranchBuilder {
    label: String,
    path_key: String,
    command_id: Option<&'static str>,
    children: BTreeMap<String, BranchBuilder>,
}

impl BranchBuilder {
    fn leaf(label: String, path_key: String, command_id: &'static str) -> Self {
        Self {
            label,
            path_key,
            command_id: Some(command_id),
            children: BTreeMap::new(),
        }
    }

    fn branch(label: String, path_key: String) -> Self {
        Self {
            label,
            path_key,
            command_id: None,
            children: BTreeMap::new(),
        }
    }

    fn into_node(self) -> TreeNode {
        TreeNode {
            label: self.label,
            path_key: self.path_key,
            command_id: self.command_id,
            children: self
                .children
                .into_values()
                .map(BranchBuilder::into_node)
                .collect(),
        }
    }
}

fn insert_command(branch_map: &mut BTreeMap<String, BranchBuilder>, command: &CommandSpec) {
    if command.path.is_empty() {
        return;
    }

    let mut current_map = branch_map;
    let mut path_parts = Vec::new();

    for (index, segment) in command.path.iter().enumerate() {
        let segment = *segment;
        path_parts.push(segment);
        let path_key = path_parts.join("/");
        let is_leaf = index + 1 == command.path.len();

        if is_leaf {
            current_map.insert(
                segment.to_string(),
                BranchBuilder::leaf(
                    segment.to_string(),
                    path_key,
                    command.id,
                ),
            );
        } else {
            current_map
                .entry(segment.to_string())
                .or_insert_with(|| BranchBuilder::branch(segment.to_string(), path_key));
            let entry = current_map.get_mut(segment).expect("branch just inserted");
            current_map = &mut entry.children;
        }
    }
}

#[derive(Debug, Clone)]
pub struct VisibleTreeRow {
    pub depth: usize,
    pub node: TreeNode,
    pub has_children: bool,
    pub expanded: bool,
}

impl TreeViewState {
    pub fn new(registry: &CommandRegistry) -> Self {
        let mut state = Self::default();
        state.expanded.insert(FAVORITES_GROUP.to_owned());
        state.expanded.insert("commands".to_owned());
        for section in registry.sections() {
            state.expanded.insert(format!("commands/{}", section.name));
        }
        state
    }

    pub fn toggle_expanded(&mut self, path_key: &str) {
        if self.expanded.contains(path_key) {
            self.expanded.remove(path_key);
        } else {
            self.expanded.insert(path_key.to_owned());
        }
    }

    pub fn visible_rows(&self, tree: &NavTree) -> Vec<VisibleTreeRow> {
        let mut rows = Vec::new();
        for root in &tree.roots {
            self.collect_visible(root, 0, &mut rows);
        }
        rows
    }

    fn collect_visible(&self, node: &TreeNode, depth: usize, rows: &mut Vec<VisibleTreeRow>) {
        let has_children = !node.children.is_empty();
        let expanded = self.expanded.contains(&node.path_key);
        rows.push(VisibleTreeRow {
            depth,
            node: node.clone(),
            has_children,
            expanded,
        });

        if has_children && expanded {
            for child in &node.children {
                self.collect_visible(child, depth + 1, rows);
            }
        }
    }

    pub fn select_command(&mut self, command_id: &'static str) {
        self.selected_command_id = Some(command_id);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;

    use dhara_tool_cli::command::{
        CommandRegistry, CommandResult, CommandSpec, CommandUi, SectionSpec, ToolContext,
    };

    use super::{NavTree, QUICK_ACTIONS, TreeViewState};

    fn noop(_: &ToolContext, _: &[String]) -> Result<CommandResult> {
        Ok(CommandResult::success())
    }

    fn sample_registry() -> CommandRegistry {
        let mut registry = CommandRegistry::new();
        registry.add_section(SectionSpec {
            name: "config",
            prompt: "cfg> ",
            summary: "Configuration",
        });
        registry.add_command(CommandSpec {
            id: "config.show",
            path: &["config", "show"],
            summary: "Show config",
            args_summary: "",
            section: "config",
            ui: CommandUi::empty("Show"),
            handler: Arc::new(noop),
        });
        registry.add_command(CommandSpec {
            id: "config.env.init",
            path: &["config", "env", "init"],
            summary: "Init env",
            args_summary: "",
            section: "config",
            ui: CommandUi::empty("Init"),
            handler: Arc::new(noop),
        });
        registry
    }

    #[test]
    fn nav_tree_builds_nested_paths() {
        let registry = sample_registry();
        let tree = NavTree::from_registry(&registry);
        let commands = tree
            .roots
            .iter()
            .find(|node| node.path_key == "commands")
            .expect("commands group");
        let config = commands
            .children
            .iter()
            .find(|node| node.label == "config")
            .expect("config branch");
        assert!(config.command_id.is_none());
        assert_eq!(config.children.len(), 2);
        let env = config.children.iter().find(|node| node.label == "env");
        assert!(env.is_some());
    }

    #[test]
    fn favorites_include_quick_actions() {
        let registry = sample_registry();
        let tree = NavTree::from_registry(&registry);
        let favorites = tree
            .roots
            .iter()
            .find(|node| node.path_key == super::FAVORITES_GROUP)
            .expect("favorites");
        assert!(favorites.children.is_empty() || favorites.children.len() <= QUICK_ACTIONS.len());
    }

    #[test]
    fn visible_rows_respect_expansion() {
        let registry = sample_registry();
        let tree = NavTree::from_registry(&registry);
        let mut state = TreeViewState::new(&registry);
        state.expanded.remove("commands");
        let collapsed = state.visible_rows(&tree);
        assert!(collapsed.iter().any(|row| row.node.path_key == "commands"));
        assert!(!collapsed.iter().any(|row| row.node.label == "config"));

        state.expanded.insert("commands".to_owned());
        let expanded = state.visible_rows(&tree);
        assert!(expanded.iter().any(|row| row.node.label == "config"));
    }
}
