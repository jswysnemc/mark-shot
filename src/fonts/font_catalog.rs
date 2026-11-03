//! 【字体目录】【家族登记】按原版家族、子家族与代工厂规则组织列表，枚举与字形预览分开

use super::fontconfig::Record;
use std::collections::BTreeMap;

type Foundries = Vec<(String, Vec<Face>)>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Face {
    pub file: String,
    pub index: u32,
}

#[derive(Clone, Debug)]
pub struct Family {
    pub name: String,
    pub faces: Vec<Face>,
}

#[derive(Default)]
pub struct FontCatalog {
    pub families: Vec<Family>,
}

impl FontCatalog {
    /// 【字体目录】【首次读取】返回系统字体目录；调用方只在首次打开字体弹层时执行，不影响启动路径
    pub fn load() -> Self {
        Self::from_records(super::fontconfig::records())
    }

    /// 【字体目录】【子家族构造】按 records 返回目录，别名不作为独立字体，多个代工厂附带方括号名称
    fn from_records(records: Vec<Record>) -> Self {
        let mut groups = BTreeMap::<String, (String, Foundries)>::new();
        for record in records {
            if record.families.is_empty() {
                continue;
            }
            let style = record.styles.first().map(String::as_str).unwrap_or("");
            let language = record.languages.first().map(String::as_str).unwrap_or("");
            for (index, name) in record.families.iter().enumerate() {
                let alternative_style = record
                    .styles
                    .get(index)
                    .map(String::as_str)
                    .unwrap_or(style);
                let alternative_language = record
                    .languages
                    .get(index)
                    .map(String::as_str)
                    .unwrap_or(language);
                if index != 0 && !(alternative_language == language && alternative_style != style) {
                    continue;
                }
                let (_, foundries) = groups
                    .entry(name.to_lowercase())
                    .or_insert_with(|| (name.clone(), Vec::new()));
                // 1. 【字体目录】【代工厂顺序】原版只排序家族名称，家族内部保留代工厂登记顺序
                let index = foundries
                    .iter()
                    .position(|(name, _)| name.eq_ignore_ascii_case(&record.foundry))
                    .unwrap_or_else(|| {
                        foundries.push((record.foundry.clone(), Vec::new()));
                        foundries.len() - 1
                    });
                let faces = &mut foundries[index].1;
                let face = Face {
                    file: record.file.clone(),
                    index: record.index,
                };
                if !faces.contains(&face) {
                    faces.push(face);
                }
            }
        }
        for generic in ["Serif", "Sans Serif", "Monospace"] {
            groups
                .entry(generic.to_lowercase())
                .or_insert_with(|| (generic.into(), vec![(String::new(), Vec::new())]));
        }
        let mut families = Vec::new();
        for (name, foundries) in groups.into_values() {
            let multiple = foundries.len() > 1;
            for (foundry, faces) in foundries {
                families.push(Family {
                    name: if multiple && !foundry.is_empty() {
                        format!("{name} [{foundry}]")
                    } else {
                        name.clone()
                    },
                    faces,
                });
            }
        }
        Self { families }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【字体目录测试】【规则样本】返回具有别名和子家族的元数据，不依赖系统安装字体
    fn record(foundry: &str) -> Record {
        Record {
            families: vec!["Sample".into(), "Sample Light".into(), "本地别名".into()],
            languages: vec!["en".into(), "en".into(), "zh".into()],
            styles: vec!["Light".into(), "Regular".into()],
            foundry: foundry.into(),
            file: "/test.ttf".into(),
            index: 0,
        }
    }

    #[test]
    fn subfamilies_are_separate_but_translated_aliases_are_not() {
        let catalog = FontCatalog::from_records(vec![record("A")]);
        let names = catalog
            .families
            .iter()
            .map(|family| family.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"Sample"));
        assert!(names.contains(&"Sample Light"));
        assert!(!names.contains(&"本地别名"));
    }

    #[test]
    fn multiple_foundries_are_named_and_duplicates_are_bounded() {
        let catalog = FontCatalog::from_records(vec![record("A"), record("A"), record("B")]);
        let family = catalog
            .families
            .iter()
            .find(|family| family.name == "Sample [A]")
            .unwrap();
        assert_eq!(family.faces.len(), 1);
        assert!(
            catalog
                .families
                .iter()
                .any(|family| family.name == "Sample [B]")
        );
    }

    #[test]
    fn foundries_preserve_registration_order_instead_of_alphabetic_order() {
        let catalog = FontCatalog::from_records(vec![record("B"), record("A")]);
        let names = catalog
            .families
            .iter()
            .filter(|family| family.name.starts_with("Sample ["))
            .map(|family| family.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Sample [B]", "Sample [A]"]);
    }
}
