# ADR 0010：完整歌词主题与 Android 柔光背景

日期：2026-10-12。用户反馈要求覆盖歌词页播放控件，并明确参考自己的 Android 项目；修订 [ADR 0009](0009-lyrics-theme-and-layout.md) 的弱色光背景。验收见[报告](../validation/lyrics-theme.md)。

## 参考与决定

只读核对 YYPlayerforAndroid 的 `ui/MusicPlayerScreen.kt`：`MusicAmbientBackground` 使用深色 #11151B / 浅色 #F4F4F1 混入封面色 17% / 12%，左侧中心 (8%,25%)、半径为高度 75% 的 46% / 29% 主题光，右侧中心 (110%,56%)、半径为高度 62% 的辅色光，再覆底部渐变。该项目换色 tween 为 950ms；背景没有无限计时器。未修改、运行或重新验收 Android 工程。

共享 MusicAmbient 用静态 Slint GPU 渐变实现同一结构，放在 AppWindow 的整窗背景；原 NowPlaying 改为透明内容。歌词背景预览复用该组件，四种矢量装饰与自定义图也在该基底上绘制。Slint 1.17.1 默认径向半径为矩形半对角线，正方形色场在 70.71% 处透明以得到目标圆半径和无矩形接缝。

## 页面颜色与控件

保留真实后台取色 / 过期资源保护。AppWindow 将歌曲颜色动画与页面混合分开：实际歌曲颜色 950ms；页面主题混合直接采用 420ms 歌词页透明度。Theme 仍是唯一语义令牌层，背景 / 表面 / 边框 / 活动色 / 图标和前景共用混合结果。浏览状态混合为 0，歌词状态为 1；快速反向操作沿当前值继续，减少动画即时到目标。

ThemeBinding 用属性别名把窗口动画值持续绑定到全局令牌，不在计时回调中重复赋色。歌词样式弹层由 AppWindow 管理，避免页面转场影响条件弹层的几何；滚动内容显式从左上开始并采用表单首选高度，最小窗口也能访问底部重置按钮。勾选与上下箭头使用矢量图标，避免符号缺字。

标准 Slint Palette 的颜色属性为只读，不能按封面改写；编译已拒绝此赋值。用小型共用主题控件覆盖歌词样式、播放栏倍速、播放选项 / EQ 的按钮、勾选、数值输入、文本输入及下拉弹层，直接读取 Theme；其余页面原控件保留。主题控件保持同一 controller 命令，支持焦点、基本键盘和可访问性属性，不改 Engine / 音频默认值。播放按钮前景依据已有相对亮度策略；正常图标使用可读的封面强调色。

有控制器状态源的控件把值作为输入，只通过回调提交用户选择，不赋值破坏该输入绑定；倍速始终显示内核投影值，包含非预设值和恢复 1x。文本编辑保留焦点中的草稿。没有对应投影的章节 / 画面比例 / 单文件循环沿用原有局部选择状态。去掉歌词边缘的平色遮罩，避免整窗径向背景在浅色下出现横条。

官方依据：[Palette](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/globals/palette/)、锁定 [1.17.1 Palette 源码](https://github.com/slint-ui/slint/blob/v1.17.1/internal/compiler/widgets/fluent/style-base.slint)、[同版本颜色 mix](https://github.com/slint-ui/slint/blob/v1.17.1/internal/core/graphics/color.rs)、[径向默认半径](https://github.com/slint-ui/slint/blob/v1.17.1/internal/core/graphics/brush.rs)。不是升级 Slint 或修改系统主题的方案。

实际 GPU 像素验收按锁定 FemtoVG OpenGL 实现处理：take_snapshot 读取上一帧后缓冲，测试主动请求重绘并在限时内等待颜色一致；生产界面不增加重绘计时器。转场截图只把已复制的像素交给一次性后台 PNG 编码，避免编码阻塞 UI 后跳过回弹采样。

## 边界

没有新依赖、资源衍生大图、逐帧 CPU 像素处理或 GL presenter 改动；GPU snapshot 仅用于验收。没有全局持久改变应用主题，退出歌词后恢复当前应用外观。中性色、缺封面和关闭取色遵循原回退；用户自定义图片与低透明度仍需自身对比度判断。物理输入、复杂可访问性、跨屏 DPI 和 Linux / macOS 运行仍须独立验证。
