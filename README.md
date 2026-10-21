# 强弱对照面板

用 [GPUI Kit](https://github.com/longbridge/gpui-kit) 写的桌面小工具，逐项判断当前走势是「强 · 可以做」还是「弱 · 要小心」。

- 左侧栏：标的 / 日期输入，强、弱、未判的计数和比例条，自动结论，以及 9 个维度的状态点。
- 右侧：按「趋势判断 / 调整结构 / 进场与执行」分组，每个维度左右两张卡片，点选记录，再点一次取消。
- 红为强、绿为弱，沿用 A 股涨跌配色；跟随系统浅色 / 深色主题。
- 判断保存在本机配置目录（`~/.config/qiangruo/state.json`，Windows 为 `%APPDATA%\qiangruo\state.json`）。

## 运行

```sh
cargo run --release
```

Linux 需要先装 `libxkbcommon-dev libxkbcommon-x11-dev`（以及 Vulkan 驱动）。
