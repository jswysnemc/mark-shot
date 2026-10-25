export const DBUS_XML = `
<node>
  <interface name="org.gnome.Shell.Extensions.MarkShotScrollHelper">
    <method name="Version">
      <arg type="s" name="version" direction="out"/>
    </method>
    <method name="PointerPosition">
      <arg type="i" name="x" direction="out"/>
      <arg type="i" name="y" direction="out"/>
    </method>
    <method name="ScreenshotArea">
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
      <arg type="i" name="width" direction="in"/>
      <arg type="i" name="height" direction="in"/>
      <arg type="s" name="filename" direction="in"/>
      <arg type="b" name="success" direction="out"/>
      <arg type="s" name="filename_used" direction="out"/>
    </method>
    <method name="ShowScrollPreview">
      <arg type="s" name="session_id" direction="in"/>
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
      <arg type="i" name="width" direction="in"/>
      <arg type="i" name="height" direction="in"/>
      <arg type="s" name="preview_path" direction="in"/>
      <arg type="s" name="status" direction="in"/>
      <arg type="s" name="axis" direction="in"/>
      <arg type="i" name="capture_pos" direction="in"/>
      <arg type="i" name="capture_len" direction="in"/>
      <arg type="i" name="total_len" direction="in"/>
      <arg type="b" name="paused" direction="in"/>
      <arg type="b" name="axis_locked" direction="in"/>
      <arg type="i" name="preview_gap" direction="in"/>
    </method>
    <method name="HideScrollPreview">
      <arg type="s" name="session_id" direction="in"/>
    </method>
    <method name="WindowGeometries">
      <arg type="s" name="json" direction="out"/>
    </method>
    <method name="SetWindowsAbove">
      <arg type="s" name="title" direction="in"/>
      <arg type="b" name="above" direction="in"/>
      <arg type="i" name="changed" direction="out"/>
    </method>
    <signal name="PreviewAction">
      <arg type="s" name="session_id"/>
      <arg type="s" name="action"/>
    </signal>
  </interface>
</node>
`;
