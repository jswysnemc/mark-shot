//! 【原生弹层】【父子契约】有序弹层链和定位规则，不保存协议对象或业务动作

use tiny_skia::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    Point,
    BelowControl(Rect),
    Submenu {
        parent_width: f32,
        row_top: f32,
        item_inset: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopupNode {
    pub id: u64,
    pub parent: Option<u64>,
    pub placement: Placement,
}

impl PopupNode {
    /// 【原生弹层】【根菜单】用 id 返回默认点定位的根节点，不包含子菜单或抓取序号
    pub fn root(id: u64) -> Self {
        Self {
            id,
            parent: None,
            placement: Placement::Point,
        }
    }
}

/// 【原生弹层】【拓扑校验】按 nodes 和 root 验证唯一编号、父节点顺序与深度，返回是否合法，不接受独立兄弟弹层
pub(crate) fn valid(nodes: &[PopupNode], root: Option<u64>) -> bool {
    if nodes.is_empty() {
        return root.is_none();
    }
    nodes.len() <= 32
        && Some(nodes[0].id) == root
        && nodes.iter().enumerate().all(|(index, node)| {
            node.id != 0
                && !nodes[..index].iter().any(|previous| previous.id == node.id)
                && node.parent == index.checked_sub(1).map(|previous| nodes[previous].id)
                && match node.placement {
                    Placement::Submenu {
                        parent_width,
                        row_top,
                        item_inset,
                    } => {
                        index > 0
                            && parent_width.is_finite()
                            && parent_width > 4.0
                            && row_top.is_finite()
                            && item_inset.is_finite()
                            && row_top >= item_inset
                    }
                    _ => index == 0,
                }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【弹层契约测试】【父子隔离】有效链保留多级父子关系，重复、孤立和循环节点全部拒绝，无参数、无返回值
    #[test]
    fn popup_chains_reject_reused_and_unrelated_ids() {
        let root = PopupNode::root(1);
        let child = PopupNode {
            id: 2,
            parent: Some(1),
            placement: Placement::Submenu {
                parent_width: 190.0,
                row_top: 4.0,
                item_inset: 4.0,
            },
        };
        assert!(valid(&[root, child], Some(1)));
        assert!(!valid(&[root, PopupNode { id: 1, ..child }], Some(1)));
        assert!(!valid(
            &[
                root,
                PopupNode {
                    parent: Some(2),
                    ..child
                }
            ],
            Some(1)
        ));
        assert!(!valid(
            &[
                root,
                PopupNode {
                    parent: None,
                    ..child
                }
            ],
            Some(1)
        ));
        assert!(!valid(&[root], None));
    }
}
