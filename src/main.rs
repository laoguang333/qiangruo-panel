//! 强弱对照面板：左侧是计数、结论和各项状态，右侧逐项点选"强 / 弱"。

use std::{collections::HashMap, fs, path::PathBuf};

use gpui_kit::component::{
    ActiveTheme,
    button::Button,
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement,
};
use gpui_kit::{prelude::*, *};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Up,
    Down,
}

struct Item {
    key: &'static str,
    title: &'static str,
    hint: Option<&'static str>,
    up: &'static [&'static str],
    down: &'static [&'static str],
}

struct Group {
    name: &'static str,
    items: &'static [Item],
}

const GROUPS: &[Group] = &[
    Group {
        name: "趋势判断",
        items: &[
            Item {
                key: "main",
                title: "主周期上涨",
                hint: None,
                up: &["接近 45° 连续上涨", "停顿少、没有长上影", "红柱面积大，是附近最猛的一波"],
                down: &[
                    "停顿多、重叠多、长上影多",
                    "有 MACD 背离迹象",
                    "涨得平缓无力，不是附近最猛的一波",
                    "周线上涨时 MACD 还没上零轴",
                ],
            },
            Item {
                key: "top",
                title: "冲顶嫌疑",
                hint: None,
                up: &["前面强势上涨后的冲高回落", "冲高后在顶部横住、没有马上回落"],
                down: &["弱势上涨 + 突然上冲 + 立马回落", "周线不强、日线却暴涨", "比附近明显陡得多的直线拉升"],
            },
        ],
    },
    Group {
        name: "调整结构",
        items: &[
            Item {
                key: "struct",
                title: "结构",
                hint: None,
                up: &["调整时有波形", "有收敛，均线粘合"],
                down: &["调整时上冲下冲，没有波形", "有很多发散的结构"],
            },
            Item {
                key: "macd",
                title: "回调 MACD 力度",
                hint: None,
                up: &["MACD 有波形", "内部有背离迹象"],
                down: &["MACD 在零轴很远的下方徘徊", "黄白线直直下穿零轴"],
            },
            Item {
                key: "kline",
                title: "回调 K 线结构",
                hint: None,
                up: &["分两三波才跌到", "下冲失败、低点守住"],
                down: &["一波直接跌到位"],
            },
            Item {
                key: "relay",
                title: "中继时长",
                hint: None,
                up: &["调整没拖太久就走出上涨"],
                down: &["之前上涨不强、中继拖太久", "弱势上涨后窄幅久盘（已经买了就及时退出）"],
            },
        ],
    },
    Group {
        name: "进场与执行",
        items: &[
            Item {
                key: "left",
                title: "左侧进场位置",
                hint: Some("强趋势 / 弱趋势"),
                up: &["小周期的破位是好的左侧买点", "也可以买在小周期加速下跌的位置", "最好不要连续下跌放量"],
                down: &["左侧买入要小心", "可以等放巨量阴线，然后有连续的阳量拉起", "最后后面要有好的结构"],
            },
            Item {
                key: "right",
                title: "右侧进场位置",
                hint: Some("大周期强 / 大周期弱"),
                up: &["大周期仍在强趋势", "右侧收敛，或呈现好的结构的突破可以买入"],
                down: &["大周期弱或走势已经变弱", "需要结构非常好、大盘也好，不然没有右侧买点"],
            },
            Item {
                key: "retry",
                title: "止损后再试",
                hint: None,
                up: &["仍有把握前面上涨强势", "小周期不恶劣"],
                down: &["拿不准前面上涨质量", "已经试过两次"],
            },
        ],
    },
];

fn all_items() -> impl Iterator<Item = &'static Item> {
    GROUPS.iter().flat_map(|g| g.items.iter())
}

/// 沿用 A 股配色：红为强，绿为弱。
struct Palette {
    bg: Hsla,
    panel: Hsla,
    ink: Hsla,
    muted: Hsla,
    line: Hsla,
    up: Hsla,
    up_soft: Hsla,
    down: Hsla,
    down_soft: Hsla,
    gold: Hsla,
}

impl Palette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                bg: rgb(0x131213).into(),
                panel: rgb(0x1c1b1d).into(),
                ink: rgb(0xece8e4).into(),
                muted: rgb(0x9a938d).into(),
                line: rgb(0x2f2c2e).into(),
                up: rgb(0xff5a4e).into(),
                up_soft: rgb(0x3a1d1b).into(),
                down: rgb(0x2fc07a).into(),
                down_soft: rgb(0x15301f).into(),
                gold: rgb(0xe0a743).into(),
            }
        } else {
            Self {
                bg: rgb(0xf6f4f1).into(),
                panel: rgb(0xffffff).into(),
                ink: rgb(0x1d1a19).into(),
                muted: rgb(0x6f6863).into(),
                line: rgb(0xe4dfd9).into(),
                up: rgb(0xd1342b).into(),
                up_soft: rgb(0xfbe9e7).into(),
                down: rgb(0x178a52).into(),
                down_soft: rgb(0xe5f4ec).into(),
                gold: rgb(0xb07a1e).into(),
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Tone {
    None,
    Up,
    Down,
    Mixed,
}

struct Verdict {
    tone: Tone,
    label: &'static str,
    text: String,
}

fn verdict(state: &HashMap<&'static str, Side>) -> Verdict {
    let u = state.values().filter(|s| **s == Side::Up).count();
    let d = state.values().filter(|s| **s == Side::Down).count();
    let main = state.get("main").copied();
    let top = state.get("top").copied();
    let (tone, label, text) = if u + d == 0 {
        (Tone::None, "待判断", "先从「主周期上涨」开始，它决定后面的标准放多宽。".into())
    } else if state.get("retry") == Some(&Side::Down) {
        (Tone::Down, "停手", "拿不准上涨质量或已试过两次，这一笔不再追。".into())
    } else if main == Some(Side::Down) && top == Some(Side::Down) {
        (Tone::Down, "回避", "主周期弱，又有弱势冲顶嫌疑，没有值得做的理由。".into())
    } else if main == Some(Side::Down) {
        (
            Tone::Down,
            "偏弱",
            "主周期上涨不强。左侧等巨量阴线后阳量拉起，右侧只做结构非常好且大盘配合的。".into(),
        )
    } else if main == Some(Side::Up) && d == 0 && u >= 5 {
        (
            Tone::Up,
            "强势可做",
            "主周期强、各项没有弱信号。小周期破位或加速下跌处可左侧，收敛突破可右侧。".into(),
        )
    } else if main == Some(Side::Up) && d <= 2 {
        (Tone::Up, "偏强", format!("主周期强，但有 {d} 项偏弱，进场时盯住这几项。"))
    } else if d > u {
        (Tone::Down, "偏弱", format!("弱项 {d} 多于强项 {u}，降低仓位或等待结构改善。"))
    } else {
        let lead = if main.is_none() { "主周期还没判。" } else { "" };
        (Tone::Mixed, "分歧", format!("{lead}强 {u} / 弱 {d}，信号不一致，等结构更清楚。"))
    };
    Verdict { tone, label, text }
}

/// 判断保存在用户配置目录下的一个 JSON 文件里。
fn store_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("qiangruo").join("state.json"))
}

fn load() -> (HashMap<&'static str, Side>, String) {
    let mut state = HashMap::new();
    let Some(json) = store_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
    else {
        return (state, String::new());
    };
    if let Some(map) = json["state"].as_object() {
        for item in all_items() {
            match map.get(item.key).and_then(|v| v.as_str()) {
                Some("u") => state.insert(item.key, Side::Up),
                Some("d") => state.insert(item.key, Side::Down),
                _ => None,
            };
        }
    }
    (state, json["code"].as_str().unwrap_or_default().to_string())
}

struct Panel {
    state: HashMap<&'static str, Side>,
    code: Entity<InputState>,
    _subscription: Subscription,
}

impl Panel {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (state, code) = load();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("标的 / 日期")
                .default_value(code)
        });
        let _subscription = cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                this.save(cx);
            }
        });
        Self { state, code: input, _subscription }
    }

    fn save(&self, cx: &App) {
        let Some(path) = store_path() else { return };
        let state: serde_json::Map<_, _> = self
            .state
            .iter()
            .map(|(k, v)| (k.to_string(), (if *v == Side::Up { "u" } else { "d" }).into()))
            .collect();
        let json = serde_json::json!({ "state": state, "code": self.code.read(cx).value().to_string() });
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(path, json.to_string());
    }

    fn toggle(&mut self, key: &'static str, side: Side, cx: &mut Context<Self>) {
        if self.state.get(key) == Some(&side) {
            self.state.remove(key);
        } else {
            self.state.insert(key, side);
        }
        self.save(cx);
        cx.notify();
    }

    fn render_sidebar(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let total = all_items().count();
        let u = self.state.values().filter(|s| **s == Side::Up).count();
        let d = self.state.values().filter(|s| **s == Side::Down).count();
        let v = verdict(&self.state);
        let (pill_bg, pill_fg) = match v.tone {
            Tone::None => (p.line, p.muted),
            Tone::Up => (p.up_soft, p.up),
            Tone::Down => (p.down_soft, p.down),
            Tone::Mixed => (p.line, p.gold),
        };

        let count = |n: usize, label: &'static str, color: Hsla| {
            div()
                .flex()
                .flex_col()
                .items_center()
                .flex_1()
                .child(div().text_size(px(28.)).font_weight(FontWeight::BOLD).text_color(color).child(n.to_string()))
                .child(div().text_xs().text_color(p.muted).child(label))
        };

        // 强 / 弱 / 未判 的比例条
        let bar = div()
            .flex()
            .h(px(8.))
            .w_full()
            .rounded(px(4.))
            .overflow_hidden()
            .bg(p.line)
            .child(div().h_full().w(relative(u as f32 / total as f32)).bg(p.up))
            .child(div().h_full().w(relative(d as f32 / total as f32)).bg(p.down));

        // 各项状态索引：一眼看出哪些还没判
        let index = GROUPS.iter().map(|g| {
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(div().text_xs().text_color(p.muted).mt(px(6.)).child(g.name))
                .children(g.items.iter().map(|it| {
                    let s = self.state.get(it.key).copied();
                    let (dot, status, status_color) = match s {
                        Some(Side::Up) => (p.up, "强", p.up),
                        Some(Side::Down) => (p.down, "弱", p.down),
                        None => (p.line, "—", p.muted),
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_sm()
                        .child(div().size(px(8.)).rounded_full().bg(dot))
                        .child(div().flex_1().text_color(p.ink).child(it.title))
                        .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(status_color).child(status))
                }))
        });

        div()
            .w(px(264.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(14.))
            .p(px(18.))
            .bg(p.panel)
            .border_r_1()
            .border_color(p.line)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(div().text_size(px(22.)).font_weight(FontWeight::BLACK).text_color(p.ink).child("强弱对照面板"))
                    .child(div().text_xs().text_color(p.muted).child("点选即记录，再点一次取消")),
            )
            .child(Input::new(&self.code).id("code"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .child(count(u, "强", p.up))
                            .child(count(d, "弱", p.down))
                            .child(count(total - u - d, "未判", p.muted)),
                    )
                    .child(bar),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .p(px(12.))
                    .rounded(px(8.))
                    .bg(pill_bg)
                    .child(div().text_base().font_weight(FontWeight::BOLD).text_color(pill_fg).child(v.label))
                    .child(div().text_sm().text_color(p.ink).child(v.text)),
            )
            .child(
                div()
                    .id("index")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .border_t_1()
                    .border_color(p.line)
                    .overflow_y_scrollbar()
                    .children(index),
            )
            .child(
                Button::new("reset")
                    .outline()
                    .label("清空判断")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.clear();
                        this.save(cx);
                        cx.notify();
                    })),
            )
    }

    fn render_option(
        &self,
        p: &Palette,
        it: &'static Item,
        side: Side,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let chosen = self.state.get(it.key).copied();
        let pressed = chosen == Some(side);
        let dimmed = chosen.is_some() && !pressed;
        let (accent, soft, lines, id) = match side {
            Side::Up => (p.up, p.up_soft, it.up, "u"),
            Side::Down => (p.down, p.down_soft, it.down, "d"),
        };
        div()
            .id(SharedString::from(format!("{}-{id}", it.key)))
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .px(px(10.))
            .py(px(5.))
            .line_height(px(19.))
            .rounded(px(6.))
            .border_1()
            .cursor_pointer()
            .text_sm()
            .text_color(p.ink)
            .when(pressed, |el| el.bg(soft).border_color(accent).border_l_4())
            .when(!pressed, |el| el.bg(p.panel).border_color(p.line).hover(|s| s.border_color(p.muted)))
            .when(dimmed, |el| el.opacity(0.42))
            .children(lines.iter().map(move |l| {
                div()
                    .flex()
                    .gap(px(6.))
                    .child(div().text_color(accent).child("·"))
                    .child(div().flex_1().min_w_0().child(*l))
            }))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle(it.key, side, cx)))
    }
}

impl Render for Panel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::new(cx.theme().is_dark());
        let sidebar = self.render_sidebar(&p, cx);

        let header = div()
            .flex()
            .gap(px(8.))
            .pl(px(132.))
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .child(div().flex_1().text_color(p.up).child("强 · 可以做"))
            .child(div().flex_1().text_color(p.down).child("弱 · 要小心"));

        let mut groups = Vec::new();
        for g in GROUPS {
            let mut rows = Vec::new();
            for it in g.items {
                let state = self.state.get(it.key).copied();
                let (sub, sub_color) = match state {
                    Some(Side::Up) => ("判为强", p.up),
                    Some(Side::Down) => ("判为弱", p.down),
                    None => (it.hint.unwrap_or(""), p.muted),
                };
                rows.push(
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(
                            div()
                                .w(px(124.))
                                .flex_none()
                                .flex()
                                .flex_col()
                                .justify_center()
                                .child(div().text_sm().font_weight(FontWeight::BOLD).text_color(p.ink).child(it.title))
                                .child(div().text_xs().text_color(sub_color).child(sub)),
                        )
                        .child(self.render_option(&p, it, Side::Up, cx))
                        .child(self.render_option(&p, it, Side::Down, cx)),
                );
            }
            groups.push(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(p.muted).child(g.name))
                    .children(rows),
            );
        }

        div()
            .size_full()
            .flex()
            .bg(p.bg)
            .child(sidebar)
            .child(
                div()
                    .id("main")
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .overflow_y_scrollbar()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .px(px(18.))
                            .py(px(12.))
                            .child(header)
                            .children(groups),
                    ),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        let bounds = Bounds::centered(None, size(px(1180.), px(760.)), cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(860.), px(560.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("强弱对照面板".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Panel::new(window, cx)),
        )
        .expect("failed to open window");
        cx.activate(true);
    });
}
