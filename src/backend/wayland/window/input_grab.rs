//! 【原生窗口】【输入抓取】仅在同一输入来源的按下与释放之间传递原始序号，完成后不能复用

use std::collections::BTreeMap;

pub(super) struct PressGrabs<Origin> {
    presses: BTreeMap<u32, (u32, Origin)>,
}

impl<Origin> Default for PressGrabs<Origin> {
    /// 【原生窗口】【抓取初始化】建立没有未完成按下的状态，无参数，返回配对容器
    fn default() -> Self {
        Self {
            presses: BTreeMap::new(),
        }
    }
}

impl<Origin: PartialEq> PressGrabs<Origin> {
    /// 【原生窗口】【记录按下】用 code、serial、origin 记录真实新按下，替换合成器抓取期间未收到释放的旧记录，无返回值
    pub fn begin(&mut self, code: u32, serial: u32, origin: Origin) {
        self.presses.insert(code, (serial, origin));
    }

    /// 【原生窗口】【配对释放】消费 code 对应按下，仅 origin 相同才返回原序号，来源不符也回收记录
    pub fn release(&mut self, code: u32, origin: &Origin) -> Option<u32> {
        self.presses
            .remove(&code)
            .filter(|(_, previous)| previous == origin)
            .map(|(serial, _)| serial)
    }

    /// 【原生窗口】【抓取取消】取消全部未完成按下，焦点或设备失效后旧释放不能打开菜单，无参数、无返回值
    pub fn cancel(&mut self) {
        self.presses.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::PressGrabs;

    /// 【原生窗口测试】【序号来源】释放只交付真实按下序号，后续释放不能复用，无参数、无返回值
    #[test]
    fn release_consumes_the_matching_press_serial_once() {
        let mut grabs = PressGrabs::default();
        grabs.begin(57, 12, (1, 2));
        assert_eq!(grabs.release(57, &(1, 2)), Some(12));
        assert_eq!(grabs.release(57, &(1, 2)), None);
    }

    /// 【原生窗口测试】【截获释放】合成器移动抓取截获旧释放后，真实新按下必须替换旧序号，无参数、无返回值
    #[test]
    fn a_new_press_replaces_a_transaction_whose_release_was_intercepted() {
        let mut grabs = PressGrabs::default();
        grabs.begin(57, 12, (1, 2));
        grabs.begin(57, 15, (1, 2));
        assert_eq!(grabs.release(57, &(1, 2)), Some(15));
    }

    /// 【原生窗口测试】【输入隔离】其他按钮释放不消费当前按钮的真实按下，无参数、无返回值
    #[test]
    fn another_button_cannot_consume_a_press() {
        let mut grabs = PressGrabs::default();
        grabs.begin(272, 12, (1, 2));
        assert_eq!(grabs.release(273, &(1, 2)), None);
        assert_eq!(grabs.release(272, &(1, 2)), Some(12));
    }

    /// 【原生窗口测试】【来源校验】其他座席或表面的释放回收旧记录但不交付序号，无参数、无返回值
    #[test]
    fn another_seat_or_surface_invalidates_the_old_press() {
        for origin in [(2, 2), (1, 3)] {
            let mut grabs = PressGrabs::default();
            grabs.begin(57, 12, (1, 2));
            assert_eq!(grabs.release(57, &origin), None);
            assert_eq!(grabs.release(57, &(1, 2)), None);
        }
    }

    /// 【原生窗口测试】【来源更换】新来源的真实按下替换旧来源，不能沿用旧序号，无参数、无返回值
    #[test]
    fn a_new_origin_starts_a_new_transaction() {
        let mut grabs = PressGrabs::default();
        grabs.begin(57, 12, (1, 2));
        grabs.begin(57, 15, (2, 2));
        assert_eq!(grabs.release(57, &(2, 2)), Some(15));
    }

    /// 【原生窗口测试】【失效清理】菜单、失焦或设备退出取消记录后，任意旧释放都不取得抓取，无参数、无返回值
    #[test]
    fn cancellation_removes_all_pending_presses() {
        let mut grabs = PressGrabs::default();
        grabs.begin(57, 12, (1, 2));
        grabs.begin(42, 15, (1, 2));
        grabs.cancel();
        assert_eq!(grabs.release(57, &(1, 2)), None);
        assert_eq!(grabs.release(42, &(1, 2)), None);
    }
}
