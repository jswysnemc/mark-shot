//! 【标注多选】【编号集合】保留生产选择顺序，过滤无效编号和重复值，单项索引由当前图层派生

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    ids: Vec<u64>,
}

impl Selection {
    /// 【标注多选】【编号读取】返回当前有序编号切片，不分配副本，无参数
    pub fn ids(&self) -> &[u64] {
        &self.ids
    }

    /// 【标注多选】【单项查询】仅选择一个编号时返回该编号，空集合和多选返回空值，无参数
    pub fn single(&self) -> Option<u64> {
        match self.ids.as_slice() {
            [id] => Some(*id),
            _ => None,
        }
    }

    /// 【标注多选】【选择更新】按ids和exists保留有效编号顺序并去重，返回集合是否实际改变
    pub fn replace(
        &mut self,
        ids: impl IntoIterator<Item = u64>,
        exists: impl Fn(u64) -> bool,
    ) -> bool {
        let mut valid = Vec::new();
        for id in ids {
            if exists(id) && !valid.contains(&id) {
                valid.push(id);
            }
        }
        if valid == self.ids {
            return false;
        }
        self.ids = valid;
        true
    }
}
