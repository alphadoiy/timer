# timer

## Intro

`timer` 现在是一个基于 `ratatui` 的多模式终端仪表盘，而不只是一个倒计时器。结合现有代码和最近一次 `feat(music): add multi-provider support and vim-style command mode` 提交，当前实现已经覆盖三条主线：

- `Clock` 模式提供模拟时钟主视图和系统时间读数。
- `Pomodoro` 模式提供可启动/重置的番茄钟，并把本地天气接进了像素风的动态道路场景里；当前天气动画基于 Unicode half-block 渲染，天气、昼夜、风雨雪都会影响画面。
- `Music` 模式已经演进成一个独立的 TUI 播放器：它有播放队列、频谱/波形可视化、音量与静音控制、随机播放、循环模式、选中播放、全屏可视化，以及来源统计覆盖层。

音乐部分这次提交的重点是“统一来源 + 统一交互”。程序会把输入自动识别为本地文件或目录、普通 HTTP 音频地址、播客 RSS、`yt-dlp` 可解析链接、以及 `m3u/pls` 电台列表，然后全部编译成同一条播放队列。界面内新增了类 Vim 的命令行，按 `:` 可以直接做队列管理，例如 `:add`、`:load`、`:radio`、`:clear`、`:vol`、`:seek`、`:station add/rm/list`、`:help`，不需要退出 TUI。

当前版本还补齐了对 live radio / live stream 的专门处理：

- 电台与其他实时流会以 `LIVE` 状态显示，而不是伪装成普通可定位音频。
- live 流不会参与时长探测，也不会允许 seek，避免把实时广播错误地当成可跳转文件。
- 即使总时长未知，Music 视图里的 seek 区域仍会保留动态 braille 波形动画，用来表达“正在播放，但不可拖动”。

最近这次天气渲染改动还把旧的 braille 画布替换成了 half-block pixel canvas。新的实现把每个终端单元拆成上下两个彩色像素，分辨率比 braille 更低，但颜色表达更稳定，也更适合太阳、月亮、云层这类偏像素画的造型。

从交互体验上看，这个项目已经更接近一个“终端里的个人仪表盘”：顶部保留统一外壳和模式切换，左侧是视觉主舞台，右侧是状态读数；进入音乐模式后又会切换成一套更像 CLIamp 的播放器布局。也就是说，当前实现的核心价值不是单点功能，而是把时钟、专注计时、实时天气和多来源音乐播放放进了同一个键盘驱动的终端工作流。

## Music Sources

当前代码能识别并入队这些来源：

- 本地音频文件和目录扫描
- 普通 HTTP 音频 URL
- 播客 RSS/Atom feed
- `yt-dlp` 支持的网站链接
- `m3u` / `pls` 电台播放列表
- `~/.config/timer/radios.toml` 中保存的自定义电台

显式执行 `:radio` 时，还会加载内置的 Code Radio 默认台，方便开箱即用地验证 live 播放链路。

## Controls

- `Tab` / 左右方向键：切换模式
- `Space`：播放/暂停或启动/暂停番茄钟
- `n` / `p`：下一首 / 上一首
- `v` / `V`：切换可视化 / 全屏可视化
- `Q` / `S`：打开播放队列 / 来源统计覆盖层
- `:`：进入命令模式
- `q`：退出

## Music Commands

- `:add <url>`：向当前队列追加一个来源
- `:load <url-or-path>`：重新加载队列
- `:radio`：载入电台列表与默认 Code Radio
- `:vol <0-100>`：设置音量
- `:seek <+secs/-secs>`：对可 seek 的音频快进/快退
- `:station add <name> <url>`：保存自定义电台
- `:station rm <name>`：删除自定义电台
- `:station list`：列出已保存电台
