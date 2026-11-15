// 【KDE窗口识别】【单次读取】只获取几何和可见状态，不订阅事件或修改窗口
const records = [];
for (const window of workspace.windowList().slice(0, 4096)) {
    const geometry = window.frameGeometry;
    const desktop = typeof workspace.currentDesktopForScreen === "function"
        ? workspace.currentDesktopForScreen(window.output) : workspace.currentDesktop;
    records.push({
        x: geometry.x, y: geometry.y, width: geometry.width, height: geometry.height,
        z: window.stackingOrder,
        eligible: Boolean(window.normalWindow || window.dialog || window.utility || window.toolbar),
        minimized: Boolean(window.minimized), hidden: Boolean(window.hidden), deleted: Boolean(window.deleted),
        on_desktop: !window.desktops.length || window.desktops.includes(desktop),
        on_activity: !window.activities.length || window.activities.includes(workspace.currentActivity)
    });
}
callDBus(__DESTINATION__, "/io/github/jswysnemc/MarkShot/WindowSnapshot",
    "io.github.jswysnemc.MarkShot.WindowSnapshot", "Submit", JSON.stringify(records));
