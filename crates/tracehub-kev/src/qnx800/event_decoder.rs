//! イベントの表示文字列(traceprinter -n と同じ書式)。
//!
//! 書式はすべて args.rs から来る。ここにあるのは「どの書式をどう当てはめるか」という手順だけ。
//! - S と、CB だけで途切れた組(payloads が 2 語以下)は fast の書式、それ以外の組は wide の書式
//! - KER_CALL は種別(enter / exit / int)と 64 ビット版で表示名・表示クラス名が決まる
//! - PR_TH の小さい番号はスレッドの状態
//! - SYSTEM の SLOG は、payloads が slog2 のレコードなら SLOG2(slog2.rs)

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use super::{args, class_code, def, event_code};
use crate::tp_format::{self, CItem, Cond, Missing, Spec, Table, Template, Var};

/// 1 行の部品
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    pub class: String,
    pub name: String,
    /// 名前より後ろ(先頭の区切りの空白を含む)
    pub args: String,
}

#[derive(Debug)]
enum CSpec {
    None,
    Silent,
    Items(Vec<CItem>),
}

fn cspec(s: Spec) -> CSpec {
    match s {
        Spec::None => CSpec::None,
        Spec::Silent => CSpec::Silent,
        Spec::Items(i) => CSpec::Items(tp_format::compile(i)),
    }
}

#[derive(Debug)]
struct CEv {
    name: Template,
    fast: CSpec,
    wide: CSpec,
    min_words: usize,
    intcall: bool,
}

/// args.rs をコンパイルしたもの
struct Spec800 {
    events: HashMap<(u8, u16), CEv>,
    wildcard: HashMap<u8, CEv>,
    blank: HashSet<(u8, u16)>,
    tables: HashMap<&'static str, Table>,
    class_display: HashMap<u8, &'static str>,
    fast_fallback_wide: HashMap<u8, bool>,
    kc_names: HashMap<u16, &'static str>,
    kc_names_exit: HashMap<u16, &'static str>,
    kc_raw_names: HashMap<u16, &'static str>,
    int_calls: HashSet<u16>,
    kc_name: Template,
    kc_unknown_call: Template,
    intcall_name: Template,
    intcall_unknown_call: Template,
    failure_when: Cond,
    failure: Vec<CItem>,
    th_names: HashMap<u16, &'static str>,
    th_name: Template,
    th_body: Vec<CItem>,
    th_overrides: HashMap<u16, Vec<CItem>>,
    /// SLOG2 として読む (内部クラス, 内部イベント番号)
    slog2: (u8, u16),
}

fn spec() -> &'static Spec800 {
    static S: OnceLock<Spec800> = OnceLock::new();
    S.get_or_init(build)
}

/// テンプレートの括弧を文字そのものにする(表示名をテンプレートとして扱うため)
fn literal(s: &'static str) -> Template {
    Template::parse(Box::leak(s.replace('{', "{{").replace('}', "}}").into_boxed_str()))
}

fn build() -> Spec800 {
    let cls = |n: &str| class_code(n).unwrap_or_else(|| panic!("args.rs: unknown internal class {n}"));
    let names = |c: &str, e: &str| args::EVENT_NAMES.iter().find(|r| r.0 == c && r.1 == e).map(|r| r.2);
    let names64 = |c: &str, e: &str| args::EVENT_NAMES_64.iter().find(|r| r.0 == c && r.1 == e).map(|r| r.2);

    let mut blank = HashSet::new();
    for (c, e) in args::BLANK_AFTER {
        let code = event_code(cls(c), e, "", false).unwrap_or_else(|| panic!("args.rs: BLANK_AFTER {c} {e}"));
        blank.insert((cls(c), code));
    }

    let mut events = HashMap::new();
    let mut wildcard = HashMap::new();
    for e in args::EVENTS {
        let c = cls(e.class);
        let compiled = |name: Template| CEv {
            name,
            fast: cspec(e.fast),
            wide: cspec(e.wide),
            min_words: e.min_words as usize,
            intcall: e.intcall,
        };
        if e.event == "*" {
            let name = Template::parse(e.name.expect("args.rs: an event \"*\" needs a name"));
            assert!(
                wildcard.insert(c, compiled(name)).is_none(),
                "args.rs: duplicate \"*\" for {}",
                e.class
            );
            continue;
        }
        let code = event_code(c, e.event, e.kind, e.is64).unwrap_or_else(|| {
            panic!(
                "args.rs: {} {} {} {} is not in def.rs",
                e.class, e.event, e.kind, e.is64
            )
        });
        let name = match e.name {
            Some(n) => Template::parse(n),
            None if c == def::CLASS_KER_CALL => Template::default(), // カーネルコールの名前の規則で決まる
            None => {
                let disp = if e.is64 {
                    names64(e.class, e.event).or_else(|| names(e.class, e.event).unwrap_or(Some(e.event)))
                } else {
                    names(e.class, e.event).unwrap_or(Some(e.event))
                };
                disp.map(literal).unwrap_or_default()
            }
        };
        assert!(
            events.insert((c, code), compiled(name)).is_none(),
            "args.rs: duplicate definition of {} {}",
            e.class,
            e.event
        );
    }

    let kc_num = |n: &str| {
        def::KERCALLS
            .iter()
            .find(|r| r.1 == n)
            .unwrap_or_else(|| panic!("args.rs: unknown kernel call {n}"))
            .0
    };
    let mut kc_names: HashMap<u16, &'static str> = def::KERCALLS.iter().map(|r| (r.0, r.1)).collect();
    for (n, d) in args::KERCALL_NAMES {
        kc_names.insert(kc_num(n), d);
    }
    let th_hidden: HashSet<&str> = args::THREAD_HIDDEN.iter().copied().collect();
    let th_code = |n: &str| {
        def::THREAD_STATES
            .iter()
            .find(|r| r.1 == n)
            .unwrap_or_else(|| panic!("args.rs: unknown thread state {n}"))
            .0
    };

    Spec800 {
        events,
        wildcard,
        blank,
        tables: args::TABLES.iter().copied().collect(),
        class_display: args::CLASS_DISPLAY.iter().map(|(c, d)| (cls(c), *d)).collect(),
        fast_fallback_wide: args::FAST_FALLBACK_WIDE.iter().map(|(c, w)| (cls(c), *w)).collect(),
        kc_names,
        kc_names_exit: args::KERCALL_NAMES_EXIT.iter().map(|(n, d)| (kc_num(n), *d)).collect(),
        kc_raw_names: def::KERCALLS.iter().map(|r| (r.0, r.1)).collect(),
        int_calls: args::INT_CALLS.iter().map(|n| kc_num(n)).collect(),
        kc_name: Template::parse(args::KERCALL_NAME),
        kc_unknown_call: Template::parse(args::KERCALL_UNKNOWN_CALL),
        intcall_name: Template::parse(args::INTCALL_NAME),
        intcall_unknown_call: Template::parse(args::INTCALL_UNKNOWN_CALL),
        failure_when: Cond::parse(args::KERCALL_FAILURE_WHEN),
        failure: tp_format::compile(args::KERCALL_FAILURE),
        th_names: def::THREAD_STATES
            .iter()
            .filter(|r| !th_hidden.contains(r.1))
            .map(|r| (r.0, r.1))
            .collect(),
        th_name: Template::parse(args::THREAD_NAME),
        th_body: tp_format::compile(args::THREAD_BODY),
        th_overrides: args::THREAD_OVERRIDES
            .iter()
            .map(|(n, i)| (th_code(n), tp_format::compile(i)))
            .collect(),
        slog2: {
            let (c, e) = args::SLOG2_EVENT;
            let c = cls(c);
            (
                c,
                event_code(c, e, "", false).unwrap_or_else(|| panic!("args.rs: SLOG2_EVENT {e}")),
            )
        },
    }
}

/// 定義の誤りを確かめる(試験用。定義が読めなければ panic)
#[cfg(test)]
pub(crate) fn check_spec() {
    let _ = spec();
}

/// 書式の選択: S(と CB だけで途切れた組)は fast、組は wide
fn choose<'a>(sp: &Spec800, class: u8, multi: bool, ev: &'a CEv) -> Option<&'a CSpec> {
    if multi {
        return Some(&ev.wide);
    }
    match &ev.fast {
        CSpec::None => {
            if sp
                .fast_fallback_wide
                .get(&class)
                .copied()
                .unwrap_or(args::FAST_FALLBACK_WIDE_DEFAULT)
            {
                Some(&ev.wide)
            } else {
                None
            }
        }
        s => Some(s),
    }
}

/// 1 イベントの行。traceprinter が行を出さないなら None
///
/// - words: payloads(Truncated・Orphan なら取り出したもの)
/// - multi: 組として扱うか(payloads が 2 語より多い)
pub(crate) fn format(class: u8, code: u16, words: &[u32], missing: Missing) -> Option<Line> {
    let sp = spec();
    let multi = words.len() > 2;
    if class == def::CLASS_KER_CALL {
        return format_kercall(sp, code, words, multi, missing);
    }
    if class == def::CLASS_PR_TH && code < def::PR_TH_PROCESS_BASE {
        return format_thread(sp, code, words, multi, missing);
    }
    let class_disp = if class == def::CLASS_PR_TH {
        args::DISPLAY_PROCESS
    } else {
        *sp.class_display.get(&class)?
    };
    if (class, code) == sp.slog2
        && let Some(a) = super::slog2::render(words)
    {
        return Some(Line {
            class: class_disp.into(),
            name: args::SLOG2_NAME.into(),
            args: a,
        });
    }
    let ev = sp.events.get(&(class, code)).or_else(|| sp.wildcard.get(&class))?;
    if words.len() < ev.min_words {
        return None;
    }
    let CSpec::Items(items) = choose(sp, class, multi, ev)? else {
        return None;
    };
    let mut name = String::new();
    ev.name.render_vars(&[("code", Var::Num(code as u64))], &mut name);
    let mut a = String::new();
    tp_format::render(items, words, &sp.tables, missing, &mut a);
    Some(Line {
        class: class_disp.into(),
        name,
        args: a,
    })
}

fn format_thread(sp: &Spec800, code: u16, words: &[u32], multi: bool, missing: Missing) -> Option<Line> {
    if code > args::THREAD_MAX_PRINTED {
        return None;
    }
    let name = thread_name(sp, code);
    let mut a = String::new();
    match sp.th_overrides.get(&code) {
        Some(items) => tp_format::render(items, words, &sp.tables, missing, &mut a),
        None => tp_format::render(&sp.th_body, words, &sp.tables, missing, &mut a),
    }
    let _ = multi; // スレッドの状態は fast・wide で同じ書式
    Some(Line {
        class: args::DISPLAY_THREAD.into(),
        name,
        args: a,
    })
}

fn thread_name(sp: &Spec800, code: u16) -> String {
    let mut name = String::new();
    match sp.th_names.get(&code) {
        Some(n) => sp.th_name.render_vars(&[("name", Var::Str(n))], &mut name),
        None => name.push_str(args::THREAD_UNKNOWN_NAME),
    }
    name
}

/// KER_CALL の表示名と表示クラス名。種別の無い番号(exit と int の両方)は None
fn kercall_names(sp: &Spec800, code: u16) -> Option<(&'static str, String)> {
    let num = code & def::KERCALL_NUM_MASK;
    let exit = code & def::KERCALL_EXIT != 0;
    let int = code & def::KERCALL_INT != 0;
    let is64 = code & def::BIT64 != 0;
    if exit && int {
        return None;
    }
    let intcall = sp.events.get(&(def::CLASS_KER_CALL, code)).is_some_and(|e| e.intcall)
        || (int && !is64 && sp.int_calls.contains(&num));
    let mut name = String::new();
    if intcall {
        let mut call = String::new();
        match sp.kc_raw_names.get(&num) {
            Some(n) => call.push_str(n),
            None => sp
                .intcall_unknown_call
                .render_vars(&[("num", Var::Num(num as u64))], &mut call),
        }
        sp.intcall_name
            .render_vars(&[("call", Var::Str(&call)), ("num", Var::Num(num as u64))], &mut name);
        return Some((args::DISPLAY_INT_CALL, name));
    }
    let disp = if int {
        args::DISPLAY_KER_INT
    } else if exit {
        args::DISPLAY_KER_EXIT
    } else {
        args::DISPLAY_KER_ENTER
    };
    let call_name = if exit { sp.kc_names_exit.get(&num) } else { None }.or_else(|| sp.kc_names.get(&num));
    let mut call = String::new();
    match call_name {
        Some(n) => call.push_str(n),
        None => sp
            .kc_unknown_call
            .render_vars(&[("num", Var::Num(num as u64))], &mut call),
    }
    sp.kc_name
        .render_vars(&[("call", Var::Str(&call)), ("num", Var::Num(num as u64))], &mut name);
    Some((disp, name))
}

fn format_kercall(sp: &Spec800, code: u16, words: &[u32], multi: bool, missing: Missing) -> Option<Line> {
    let (disp, name) = kercall_names(sp, code)?;
    if disp == args::DISPLAY_INT_CALL {
        return Some(Line {
            class: disp.into(),
            name,
            args: String::new(),
        });
    }
    let ev = sp.events.get(&(def::CLASS_KER_CALL, code))?;
    let CSpec::Items(items) = choose(sp, def::CLASS_KER_CALL, multi, ev)? else {
        return None;
    };
    let exit = code & def::KERCALL_EXIT != 0;
    let mut a = String::new();
    // 失敗した exit は共通の書式(表示するイベントに限る)
    if exit && sp.failure_when.eval(words) {
        tp_format::render(&sp.failure, words, &sp.tables, missing, &mut a);
    } else {
        tp_format::render(items, words, &sp.tables, missing, &mut a);
    }
    Some(Line {
        class: disp.into(),
        name,
        args: a,
    })
}

/// traceprinter がイベントの後ろに空行を入れるか(行を出さないイベントでも入れる)
pub(crate) fn blank_after(class: u8, code: u16) -> bool {
    spec().blank.contains(&(class, code))
}

/// 表示クラス名と表示イベント名(行を出さないイベントでも、名前の規則で決まれば返す)
pub(crate) fn names(class: u8, code: u16) -> (Option<String>, Option<String>) {
    let sp = spec();
    if class == def::CLASS_KER_CALL {
        return match kercall_names(sp, code) {
            Some((c, n)) => (Some(c.into()), Some(n)),
            None => (None, None),
        };
    }
    if class == def::CLASS_PR_TH && code < def::PR_TH_PROCESS_BASE {
        return (Some(args::DISPLAY_THREAD.into()), Some(thread_name(sp, code)));
    }
    let disp = if class == def::CLASS_PR_TH {
        Some(args::DISPLAY_PROCESS)
    } else {
        sp.class_display.get(&class).copied()
    };
    let name = sp
        .events
        .get(&(class, code))
        .or_else(|| sp.wildcard.get(&class))
        .map(|ev| {
            let mut n = String::new();
            ev.name.render_vars(&[("code", Var::Num(code as u64))], &mut n);
            n
        });
    (disp.map(String::from), name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(class: u8, code: u16, w: &[u32]) -> Option<String> {
        format(class, code, w, Missing::Omit).map(|l| format!("{}|{}|{}", l.class, l.name, l.args))
    }

    #[test]
    fn spec_compiles() {
        check_spec();
    }

    #[test]
    fn formats() {
        let aaaa = 0x41414141;
        // fast と wide は payloads の語数で選ぶ
        assert_eq!(
            line(def::CLASS_KER_CALL, 11, &[3, aaaa]).unwrap(),
            "KER_CALL|MSG_SENDV/11| coid:0x00000003 msg[0]:\"AAAA\" (0x41414141)"
        );
        assert_eq!(
            line(def::CLASS_KER_CALL, 11, &[3, 1, 2, aaaa, 0x42424242, 0x43434343]).unwrap(),
            "KER_CALL|MSG_SENDV/11| coid:0x00000003 sparts:1 rparts:2 msg[0]:\"AAAABBBBCCCC\" (0x41414141 0x42424242 0x43434343)"
        );
        assert_eq!(
            line(def::CLASS_PR_TH, 1, &[1, 6]).unwrap(),
            "THREAD|THRUNNING| pid:1 tid:6 "
        );
        assert_eq!(
            line(def::CLASS_PR_TH, 1, &[1, 6, 10, 2]).unwrap(),
            "THREAD|THRUNNING| pid:1 tid:6 priority:10 policy:2"
        );
        assert_eq!(
            line(def::CLASS_SYSTEM, 12, &[3, 5, 0x6261, 0]).unwrap(),
            "SYSTEM|SLOG| opcode:3 severity:5 message:ab"
        );
        assert_eq!(
            line(def::CLASS_CONTROL, 2, &[1, 6]).unwrap(),
            "CONTROL|BUFFER| sequence = 1, num_events = 6"
        );
        assert_eq!(line(0, 0, &[0, 0]), None);
    }
}
