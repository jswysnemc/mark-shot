#pragma once

namespace markshot::cli {

/**
 * 查询当前录制状态并输出 JSON。
 * @return 查询完成返回 0。
 */
int printRecordingStatus();

/**
 * 请求停止当前录制并输出 JSON。
 * @return 已提交停止请求返回 0，没有可停止录制时返回 1。
 */
int stopRecordingFromCommandLine();

/**
 * 在暂停与继续之间切换当前录制并输出 JSON。
 * @return 切换成功返回 0，没有活动录制时返回 1。
 */
int togglePauseRecordingFromCommandLine();

}  // namespace markshot::cli
